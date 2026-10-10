use std::{collections::HashMap, env, path::PathBuf, thread, time::Duration};

use chrono::{Local, Utc};
use clap::Parser;
use git2::Repository;
use octocrab::models::pulls::Comment;

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(short, long, default_value_t = 5)]
    pub interval: u8,

    #[arg(short, long)]
    pub cwd: Option<PathBuf>,
}

#[derive(Debug)]
struct ReportState {
    comments_fetched_at: Option<chrono::DateTime<Utc>>,
    runs_reported: Vec<String>,
    comments_reported: HashMap<u64, Comment>,
}

impl ReportState {
    pub fn new() -> Self {
        Self {
            comments_fetched_at: None,
            runs_reported: Vec::default(),
            comments_reported: HashMap::default(),
        }
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let cwd = args
        .cwd
        .or(env::current_dir().ok())
        .expect("Failed to read cwd")
        .canonicalize()?;

    let local_repo = Repository::open(cwd)?;
    let octocrab = octocrab::Octocrab::builder()
        .personal_token(env::var("GITHUB_TOKEN")?)
        .build()?;

    let remote = local_repo.find_remote("origin")?;
    let url = remote.url().expect("remote URL is not UTF-8");

    let slug = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
        .ok_or("unsupported GitHub remote URL")?;

    let slug = slug.strip_suffix(".git").unwrap_or(slug);
    let (owner, repo) = slug.split_once('/').ok_or("invalid remote URL")?;

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
            let comments = octocrab
                .pulls(owner, repo)
                .list_comments(Some(pr.number))
                .since(
                    state
                        .comments_fetched_at
                        .or(pr.created_at)
                        .expect("No date"),
                )
                .send()
                .await?;
            state.comments_fetched_at = Some(Local::now().to_utc());

            for comment in comments {
                state.maybe_push_comment(comment);
            }
        }

        thread::sleep(Duration::from_secs(args.interval.into()));
    }
}
