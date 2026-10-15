use std::{collections::HashMap, env, error::Error, path::PathBuf, time::Duration};

use chrono::{Local, Utc};
use clap::Parser;
use git2::{Remote, Repository};
use octocrab::{
    models::{
        Status,
        checks::CheckRun,
        issues,
        pulls::{Comment, PullRequest, Review},
        repos::RepoCommit,
    },
    params::{State, repos::Commitish},
};

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(short, long, default_value_t = 30)]
    pub interval: u64,

    #[arg(short, long)]
    pub cwd: Option<PathBuf>,

    #[arg(short, long)]
    pub pr: Option<u64>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum EventKey {
    ReviewComment(u64),
    ConversationComment(u64),
    CheckRun(u64),
    HeadCommit,
    Review(u64),
    Status(u64),
}

#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum Event {
    ReviewComment(Comment),
    ConversationComment(issues::Comment),
    CheckRun(CheckRun),
    HeadCommit(RepoCommit),
    Review(Review),
    Status(Status),
}

#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum PR<'a> {
    PullRequest(&'a PullRequest),
}

impl Event {
    pub fn key(&self) -> EventKey {
        match self {
            Event::ReviewComment(e) => EventKey::ReviewComment(e.id.0),
            Event::ConversationComment(e) => EventKey::ConversationComment(e.id.0),
            Event::CheckRun(e) => EventKey::CheckRun(e.id.0),
            Event::Review(e) => EventKey::Review(e.id.0),
            Event::Status(e) => EventKey::Status(e.id.map_or(0, |id| id.0)),
            Event::HeadCommit(_) => EventKey::HeadCommit,
        }
    }
}

#[derive(Debug)]
struct ReportState {
    pr: PullRequest,
    fetched_at: Option<chrono::DateTime<Utc>>,
    events: HashMap<EventKey, Event>,
}

impl ReportState {
    pub fn new(pr: PullRequest) -> Self {
        println!(
            "{}",
            serde_json::to_string(&PR::PullRequest(&pr)).expect("Unable to write json")
        );

        Self {
            pr,
            fetched_at: None,
            events: HashMap::with_capacity(6),
        }
    }

    pub fn maybe_update_pr(&mut self, pr: PullRequest) {
        if self.pr == pr {
            return;
        }

        println!(
            "{}",
            serde_json::to_string(&PR::PullRequest(&pr)).expect("Unable to write json")
        );
        self.pr = pr;
    }

    pub fn maybe_set_event(&mut self, event: Event) {
        if self.events.get(&event.key()).is_some_and(|e| e == &event) {
            return;
        }

        println!(
            "{}",
            serde_json::to_string(&event).expect("Unable to write json")
        );
        self.events.insert(event.key(), event);
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

async fn fetch_check_runs(
    octocrab: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
    sha: &str,
) -> octocrab::Result<Vec<CheckRun>> {
    let mut checks = Vec::new();
    let mut page_number = 1u32;
    loop {
        let page = octocrab
            .checks(owner, repo)
            .list_check_runs_for_git_ref(Commitish(sha.to_owned()))
            .per_page(100)
            .page(page_number)
            .send()
            .await?;
        let is_empty = page.check_runs.is_empty();
        checks.extend(page.check_runs);
        if is_empty || checks.len() as u64 >= page.total_count {
            return Ok(checks);
        }
        page_number += 1;
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
        let pulls: Vec<PullRequest> = match args.pr {
            Some(number) => {
                vec![octocrab.pulls(owner, repo).get(number).await?]
            }
            None => {
                let head = local_repo.head()?;
                if !head.is_branch() {
                    continue;
                }
                let branch = head.shorthand()?;

                octocrab
                    .all_pages(
                        octocrab
                            .pulls(owner, repo)
                            .list()
                            .state(State::All)
                            .head(format!("{owner}:{branch}"))
                            .per_page(100)
                            .send()
                            .await?,
                    )
                    .await?
            }
        };

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

            let (reviews, comments, checks, issue_comments, commit, statuses) = tokio::try_join!(
                async {
                    octocrab
                        .all_pages(
                            octocrab
                                .pulls(owner, repo)
                                .list_reviews(state.pr.number)
                                .per_page(100)
                                .send()
                                .await?,
                        )
                        .await
                },
                async {
                    octocrab
                        .all_pages(
                            octocrab
                                .pulls(owner, repo)
                                .list_comments(Some(state.pr.number))
                                .per_page(100)
                                .since(state.fetched_at.or(state.pr.created_at).expect("No date"))
                                .send()
                                .await?,
                        )
                        .await
                },
                async { fetch_check_runs(&octocrab, owner, repo, &state.pr.head.sha).await },
                async {
                    octocrab
                        .all_pages(
                            octocrab
                                .issues(owner, repo)
                                .list_comments(state.pr.number)
                                .per_page(100)
                                .since(state.fetched_at.or(state.pr.created_at).expect("No date"))
                                .send()
                                .await?,
                        )
                        .await
                },
                async { octocrab.commits(owner, repo).get(&state.pr.head.sha).await },
                async {
                    octocrab
                        .all_pages(
                            octocrab
                                .repos(owner, repo)
                                .list_statuses(state.pr.head.sha.to_owned())
                                .per_page(100)
                                .send()
                                .await?,
                        )
                        .await
                }
            )?;

            state.fetched_at = Some(loop_start);
            state.maybe_set_event(Event::HeadCommit(commit));
            for review in reviews {
                state.maybe_set_event(Event::Review(review));
            }

            for status in statuses {
                state.maybe_set_event(Event::Status(status));
            }

            for comment in comments {
                state.maybe_set_event(Event::ReviewComment(comment));
            }

            for check in checks {
                state.maybe_set_event(Event::CheckRun(check));
            }

            for issue_comment in issue_comments {
                state.maybe_set_event(Event::ConversationComment(issue_comment));
            }
        }

        tokio::time::sleep(Duration::from_secs(args.interval)).await;
    }
}
