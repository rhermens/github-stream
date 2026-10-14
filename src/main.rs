use std::{collections::HashMap, env, error::Error, path::PathBuf, thread, time::Duration};

use chrono::{Local, Utc};
use clap::Parser;
use git2::{Remote, Repository};
use octocrab::{
    models::{
        checks::CheckRun,
        issues,
        pulls::{Comment, PullRequest},
        repos::RepoCommit,
    },
    params::repos::Commitish,
};

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(short, long, default_value_t = 30)]
    pub interval: u8,

    #[arg(short, long)]
    pub cwd: Option<PathBuf>,
}

#[derive(Debug)]
struct ReportState {
    pr: PullRequest,
    fetched_at: Option<chrono::DateTime<Utc>>,
    current_check: Option<CheckRun>,
    current_comment: Option<Comment>,
    current_issue_comment: Option<issues::Comment>,
    current_commit: Option<RepoCommit>,
}

impl ReportState {
    pub fn new(pr: PullRequest) -> Self {
        println!(
            "{}",
            serde_json::to_string(&pr).expect("Unable to write json")
        );

        Self {
            pr,
            fetched_at: None,
            current_check: None,
            current_comment: None,
            current_issue_comment: None,
            current_commit: None,
        }
    }

    pub fn maybe_update_pr(&mut self, pr: PullRequest) {
        if self.pr == pr {
            return;
        }

        println!(
            "{}",
            serde_json::to_string(&pr).expect("Unable to write json")
        );
        self.pr = pr;
    }

    pub fn maybe_set_check(&mut self, check: CheckRun) {
        if self
            .current_check
            .as_ref()
            .is_some_and(|current| current == &check)
        {
            return;
        }

        println!(
            "{}",
            serde_json::to_string(&check).expect("Unable to write json")
        );
        self.current_check = Some(check);
    }

    pub fn maybe_set_commit(&mut self, commit: RepoCommit) {
        if self
            .current_commit
            .as_ref()
            .is_some_and(|current| current == &commit)
        {
            return;
        }

        println!(
            "{}",
            serde_json::to_string(&commit).expect("Unable to write json")
        );
        self.current_commit = Some(commit);
    }

    pub fn maybe_set_issue_comment(&mut self, comment: issues::Comment) {
        if self
            .current_issue_comment
            .as_ref()
            .is_some_and(|current| current == &comment)
        {
            return;
        }

        println!(
            "{}",
            serde_json::to_string(&comment).expect("Unable to write json")
        );
        self.current_issue_comment = Some(comment);
    }

    pub fn maybe_set_comment(&mut self, comment: Comment) {
        if self
            .current_comment
            .as_ref()
            .is_some_and(|current| current == &comment)
        {
            return;
        }

        println!(
            "{}",
            serde_json::to_string(&comment).expect("Unable to write json")
        );
        self.current_comment = Some(comment);
    }
}

struct GithubOrigin<'a>(&'a str, &'a str);
impl<'a> GithubOrigin<'a> {
    pub fn from_remote(remote: &'a Remote) -> Result<Self, Box<dyn Error>> {
        let url = remote.url().expect("Unable to read remote");
        let slug = url
            .strip_prefix("https://github.com/")
            .or_else(|| url.strip_prefix("git@github.com:"))
            .or_else(|| url.strip_prefix("ssh://git@github.com/"))
            .ok_or("unsupported GitHub remote URL")?;

        let slug = slug.strip_suffix(".git").unwrap_or(slug);
        let (owner, repo) = slug.split_once('/').ok_or("invalid remote URL")?;

        Ok(GithubOrigin(owner, repo))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let cwd = args
        .cwd
        .or(env::current_dir().ok())
        .expect("Failed to read cwd")
        .canonicalize()?;

    let local_repo = Repository::open(cwd)?;
    let remote = local_repo.find_remote("origin")?;
    let GithubOrigin(owner, repo) = GithubOrigin::from_remote(&remote)?;

    let octocrab = octocrab::Octocrab::builder()
        .personal_token(env::var("GITHUB_TOKEN")?)
        .build()?;

    let mut states: HashMap<u64, ReportState> = HashMap::new();
    loop {
        let head = local_repo.head()?;
        if !head.is_branch() {
            continue;
        }
        let branch = head.shorthand()?;

        let pulls = octocrab
            .pulls(owner, repo)
            .list()
            .state(octocrab::params::State::Open)
            .head(format!("{owner}:{branch}"))
            .send()
            .await?;

        for pr in pulls {
            let state = match states.entry(pr.number) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().maybe_update_pr(pr);
                    entry.into_mut()
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(ReportState::new(pr))
                }
            };

            let loop_start = Local::now().to_utc();
            let comments = octocrab
                .pulls(owner, repo)
                .list_comments(Some(state.pr.number))
                .since(state.fetched_at.or(state.pr.created_at).expect("No date"))
                .send()
                .await?;
            let checks = octocrab
                .checks(owner, repo)
                .list_check_runs_for_git_ref(Commitish(state.pr.head.sha.to_owned()))
                .send()
                .await?;
            let issue_comments = octocrab
                .issues(owner, repo)
                .list_comments(state.pr.number)
                .since(state.fetched_at.or(state.pr.created_at).expect("No date"))
                .send()
                .await?;
            let commit = octocrab
                .commits(owner, repo)
                .get(&state.pr.head.sha)
                .await?;

            state.fetched_at = Some(loop_start);
            state.maybe_set_commit(commit);
            for comment in comments {
                state.maybe_set_comment(comment);
            }

            for check in checks.check_runs {
                state.maybe_set_check(check);
            }

            for issue_comment in issue_comments {
                state.maybe_set_issue_comment(issue_comment);
            }
        }

        thread::sleep(Duration::from_secs(args.interval.into()));
    }
}
