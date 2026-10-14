# github-stream

Watch GitHub pull request activity from your terminal.

`github-stream` polls pull requests in all states, selected by your local branch
or an explicit PR number. It prints PR metadata, formal reviews, inline review
comments, top-level conversation comments, check runs, legacy commit statuses,
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

To watch a specific PR in the checkout's `origin` repository:

```sh
github-stream --pr 42
```

Combine the PR number with another checkout and polling interval:

```sh
github-stream --cwd /path/to/repository --pr 42 --interval 60
```

Options:

- `-c, --cwd <CWD>`: repository directory; defaults to the current directory.
- `-i, --interval <INTERVAL>`: seconds between polling passes; defaults to `30`.
- `-p, --pr <PR>`: PR number in the checkout's `origin` repository.

Without `--pr`, the watcher selects open, closed, and merged PRs using
`ORIGIN_OWNER:CURRENT_BRANCH`. Historical PRs for the same branch can appear.
Fork PRs with a different head owner are not selected in this mode.

With `--pr`, the watcher fetches that PR directly, regardless of its state or the
current branch. A named branch is not required. This also selects fork PRs when
`origin` points to their base repository.

A local Git checkout with an `origin` remote is required in both modes.
PR URLs and a separate repository option are not supported.

## Output

Standard output contains one JSON event per line with a `type` discriminator and
the original GitHub object in `data`. State is tracked separately for each matching PR.

The event types are `pull_request`, `review`, `review_comment`,
`conversation_comment`, `check_run`, `status`, and `head_commit`.

For example (payload abbreviated):

```json
{"type":"review_comment","data":{"id":123,"body":"Please update this","path":"src/main.rs"}}
```

| Object | Behavior | Payload fields (`data`) |
| --- | --- | --- |
| Pull request | Printed on discovery and when returned metadata changes, including state and head SHA. | `number`, `state`, `merged_at`, `head`, `base` |
| Formal review | Fetched on each pass, including approval and request-changes summaries. | `id`, `body`, `state`, `commit_id`, `submitted_at` |
| Inline review comment | Fetched incrementally using `since`; new and updated comments can appear. | `pull_request_review_id`, `path`, `body` |
| Conversation comment | Top-level PR comments, fetched incrementally using `since`. | `issue_url`, `body`, `user` |
| Check run | Fetched for the current PR head on each pass. | `id`, `head_sha`, `name`, `conclusion` |
| Legacy commit status | Fetched for the current PR head on each pass. | `id`, `context`, `state`, `description`, `target_url` |
| Head commit | Printed when the returned commit differs from the previous head commit. Intermediate commits are not printed. | `sha`, `commit`, `parents` |

The first pass includes existing records. Later comment requests use the previous
fetch time. Each object type suppresses only an identical consecutive object for
that PR. Records can repeat, especially when a PR has multiple reviews, statuses,
or check runs.

Merged PRs have `state: "closed"`; use `merged_at` to distinguish them from
unmerged closed PRs. State changes appear as `pull_request` events when the PR
is returned by a later poll. The watcher continues after closure or merge.

For readable output with `jq`:

```sh
github-stream | jq --unbuffered .
```

To select inline review comment payloads:

```sh
github-stream | jq --unbuffered 'select(.type == "review_comment") | .data'
```

## Coverage and limits

- Closed and merged PRs remain eligible for polling; the watcher does not stop
  automatically.
- No matching PR produces no output. Silence does not confirm a successful poll.
- Unhandled API or authentication errors terminate the process. Inspect stderr
  before restarting.
- Stop the watcher with `Ctrl-C`.
