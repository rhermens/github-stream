# github-watch

Watch GitHub pull request activity from your terminal.

`github-watch` polls open pull requests for your local branch and prints inline
review comments and check runs as newline-delimited JSON.

## Installation
## Authentication

Set `GITHUB_TOKEN` before starting the program. Octocrab does not automatically
reuse GitHub CLI authentication.

If you already authenticated with `gh`:

```sh
export GITHUB_TOKEN="$(gh auth token)"
```

## Usage

Run from the root of a GitHub repository with an `origin` remote:

```sh
github-watch
```

To watch another repository with a 60-second polling interval:

```sh
github-watch --cwd /path/to/repository --interval 60
```

## Output

Standard output contains one JSON object per line. Review comment objects and
check run objects appear in the same stream.

- Each inline review comment is printed once per process, identified by its ID.
- Each check run is printed when first seen.
- A check run is printed again when `started_at` or `completed_at` changes.

For readable output with `jq`:

```sh
github-watch | jq --unbuffered .
```

