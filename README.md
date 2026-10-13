# github-stream

Watch GitHub pull request activity from your terminal.

`github-stream` polls open pull requests for your local branch and prints inline
review comments, check runs, and head commits as newline-delimited JSON.

## Installation
## Authentication

Set `GITHUB_TOKEN` before starting the program. Octocrab does not automatically
reuse GitHub CLI authentication.

If you already authenticated with `gh`:

```sh
export GITHUB_TOKEN="$(gh auth token)"
```

## Todos
- [ ] The two bot comments on the PR's main thread (coverage report and Qodana).
- [ ] The body of the Copilot review.
- [ ] No closed or merged event: I'd only notice the PR closing by checking the PR status myself.
- [ ] Partial results: the pr-watch skill says it doesn't fetch every page of results, so a busy PR could lose records.
- [ ] No head-change check: the skill also has me confirm the latest head commit on every pass.

## Usage

Run from the root of a GitHub repository with an `origin` remote:

```sh
github-stream
```

To watch another repository with a 60-second polling interval:

```sh
github-stream --cwd /path/to/repository --interval 60
```

## Output

Standard output contains one JSON object per line. Review comment, check run, and
commit objects appear in the same stream.

- Each inline review comment is printed once per process, identified by its ID.
- Each check run is printed when first seen.
- A check run is printed again when `started_at` or `completed_at` changes.
- The pull request head commit is printed once per process, identified by its
  SHA. A push prints the new head commit; intermediate commits are not printed.

For readable output with `jq`:

```sh
github-stream | jq --unbuffered .
```

