# github-stream

Watch GitHub pull request activity from your terminal.

`github-stream` polls open pull requests for your local branch and prints PR
metadata, inline review comments, top-level conversation comments, check runs,
and head commits as newline-delimited JSON.

## Installation

Install from a local checkout with Rust and Cargo:

```sh
cargo install --path .
```

With Nix, run directly from the checkout:

```sh
nix run . -- --help
```

## Authentication

Set `GITHUB_TOKEN` before starting the program. Octocrab does not automatically
reuse GitHub CLI authentication.

If you already authenticated with `gh`:

```sh
export GITHUB_TOKEN="$(gh auth token)"
```

## Usage

Run from the root of a GitHub repository with an `origin` remote and a named
branch:

```sh
github-stream
```

To watch another repository with a 60-second polling interval:

```sh
github-stream --cwd /path/to/repository --interval 60
```

Options:

- `-c, --cwd <CWD>`: repository directory; defaults to the current directory.
- `-i, --interval <INTERVAL>`: seconds between polling passes; defaults to `30`.

The watcher selects open PRs using `ORIGIN_OWNER:CURRENT_BRANCH`. It does not
accept a PR URL or number. Fork PRs with a different head owner are not selected.

## Output

Standard output contains one JSON object per line, without an event-type wrapper.
State is tracked separately for each matching PR.

| Object | Behavior | Identifying fields |
| --- | --- | --- |
| Pull request | Printed on discovery and when returned metadata changes, including the head SHA. | `number`, `head`, `base` |
| Inline review comment | Fetched incrementally using `since`; new and updated comments can appear. | `pull_request_review_id`, `path`, `body` |
| Conversation comment | Top-level PR comments, fetched incrementally using `since`. | `issue_url`, `body`, `user` |
| Check run | Fetched for the current PR head on each pass. | `head_sha`, `status`, `conclusion` |
| Head commit | Printed when the returned commit differs from the previous head commit. Intermediate commits are not printed. | `sha`, `commit`, `parents` |

The first pass includes existing records. Later comment requests use the previous
fetch time. Each object type suppresses only an identical consecutive object for
that PR. Records can repeat, especially when a PR has multiple check runs.
Consumers should deduplicate by object type and ID or SHA, while tracking changes.

For readable output with `jq`:

```sh
github-stream | jq --unbuffered .
```

## Coverage and limits

- API lists are not paginated. Busy PRs can have missing records; use paginated
  GitHub CLI requests when complete coverage is required.
- Formal review bodies and legacy commit statuses are not streamed.
- Only open PRs are queried. Closing or merging a PR does not emit a final
  PR-state object or stop the watcher; refresh PR metadata separately.
- No matching PR produces no output. Silence does not confirm a successful poll.
- Unhandled API or authentication errors terminate the process. Inspect stderr
  before restarting.
- Stop the watcher with `Ctrl-C`.
