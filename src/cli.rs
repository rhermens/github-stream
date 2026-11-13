use std::path::PathBuf;

/// Watch GitHub pull request activity from your terminal.
///
/// Poll pull requests in all states, selected by your local branch or an explicit
/// PR number. Print PR metadata, reviews, inline review comments, conversation
/// comments, check runs, legacy commit statuses, and head commits as
/// newline-delimited JSON.
///
/// Set GITHUB_TOKEN before starting. To reuse GitHub CLI authentication, run:
/// export GITHUB_TOKEN="$(gh auth token)".
///
/// A local Git checkout with an origin remote is required in both modes.
/// The watcher continues after closure or merge. Stop it with Ctrl-C.
#[derive(Debug, clap::Parser)]
pub struct Args {
    /// Seconds between polling passes.
    #[arg(short, long, default_value_t = 30)]
    pub interval: u64,

    /// Repository directory; defaults to the current directory.
    ///
    /// The checkout's origin remote identifies the GitHub repository.
    #[arg(short, long)]
    pub cwd: Option<PathBuf>,

    /// PR number in the checkout's origin repository.
    ///
    /// Fetch this PR directly, regardless of its state or the current branch.
    /// A named branch is not required. Fork PRs are supported when origin points
    /// to their base repository.
    ///
    /// Without this option, select open, closed, and merged PRs using
    /// ORIGIN_OWNER:CURRENT_BRANCH. Historical PRs for the same branch can appear.
    /// Fork PRs with a different head owner are not selected in this mode.
    #[arg(short, long)]
    pub pr: Option<u64>,
}
