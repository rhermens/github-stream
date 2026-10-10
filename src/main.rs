use std::{collections::HashMap, env, error::Error, path::PathBuf, thread, time::Duration};

use chrono::{Local, Utc};
use clap::Parser;
use git2::{Remote, Repository};
use octocrab::{
    models::{checks::CheckRun, pulls::Comment},
    params::repos::Commitish,
};

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(short, long, default_value_t = 5)]
    pub interval: u8,

    #[arg(short, long)]
    pub cwd: Option<PathBuf>,
}

#[derive(Debug)]
struct ReportState {
    fetched_at: Option<chrono::DateTime<Utc>>,
    checks_reported: HashMap<u64, CheckRun>,
    comments_reported: HashMap<u64, Comment>,
}

impl ReportState {
    pub fn new() -> Self {
        Self {
            fetched_at: None,
            checks_reported: HashMap::default(),
            comments_reported: HashMap::default(),
        }
    }

    pub fn maybe_push_check(&mut self, check: CheckRun) {
        if self.checks_reported.contains_key(&check.id) {
            return;
        }

        println!(
            "{}",
            serde_json::to_string(&check).expect("Unable to write json")
        );
        self.checks_reported.insert(check.id.into_inner(), check);
    }

    pub fn maybe_push_comment(&mut self, comment: Comment) {
        if self.comments_reported.contains_key(&comment.id) {
            return;
        }

        println!(
            "{}",
            serde_json::to_string(&comment).expect("Unable to write json")
        );
        self.comments_reported
            .insert(comment.id.into_inner(), comment);
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

    let mut state = ReportState::new();
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
            let loop_start = Local::now().to_utc();
            let comments = octocrab
                .pulls(owner, repo)
                .list_comments(Some(pr.number))
                .since(state.fetched_at.or(pr.created_at).expect("No date"))
                .send()
                .await?;
            let checks = octocrab
                .checks(owner, repo)
                .list_check_runs_for_git_ref(Commitish(pr.head.sha))
                .send()
                .await?;
            state.fetched_at = Some(loop_start);

            for comment in comments {
                state.maybe_push_comment(comment);
            }

            for check in checks.check_runs {
                state.maybe_push_check(check);
            }
        }

        thread::sleep(Duration::from_secs(args.interval.into()));
    }
}
