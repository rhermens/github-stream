use super::*;
use serde::Serialize;
use serde_json::{Value, json};

fn assert_event(event: Event<'_>, event_type: &str, payload: &impl Serialize) {
    let line = serde_json::to_string(&event).expect("event should serialize");
    assert!(!line.contains('\n'), "events must fit on one JSON line");
    let actual: Value = serde_json::from_str(&line).expect("event should be valid JSON");
    assert_eq!(actual, json!({"type": event_type, "data": payload}));
}

#[test]
fn pull_request_event_preserves_payload() {
    let pr: PullRequest = serde_json::from_value(json!({
        "id": 1,
        "number": 42,
        "url": "https://api.github.com/repos/owner/repo/pulls/42",
        "title": "Add event wrappers",
        "head": {"ref": "feature", "sha": "abc"},
        "base": {"ref": "main", "sha": "def"}
    }))
    .unwrap();

    assert_event(Event::PullRequest(&pr), "pull_request", &pr);
}

#[test]
fn review_comment_event_preserves_payload() {
    let comment: Comment = serde_json::from_value(json!({
        "id": 2,
        "node_id": "review-comment",
        "url": "https://api.github.com/repos/owner/repo/pulls/comments/2",
        "html_url": "https://github.com/owner/repo/pull/42#discussion_r2",
        "pull_request_review_id": 3,
        "diff_hunk": "@@ -1 +1 @@\n-old\n+new",
        "path": "src/main.rs",
        "commit_id": "abc",
        "original_commit_id": "def",
        "body": "Please update this.\nThanks! 🦀",
        "created_at": "2026-10-14T00:00:00Z",
        "updated_at": "2026-10-14T01:00:00Z",
        "_links": {}
    }))
    .unwrap();

    assert_event(Event::ReviewComment(&comment), "review_comment", &comment);
}

#[test]
fn conversation_comment_event_preserves_payload() {
    let url = "https://api.github.com/users/reviewer";
    let comment: issues::Comment = serde_json::from_value(json!({
        "id": 4,
        "node_id": "conversation-comment",
        "url": "https://api.github.com/repos/owner/repo/issues/comments/4",
        "html_url": "https://github.com/owner/repo/pull/42#issuecomment-4",
        "issue_url": "https://api.github.com/repos/owner/repo/issues/42",
        "body": "Looks good.\nThank you!",
        "created_at": "2026-10-14T00:00:00Z",
        "user": {
            "login": "reviewer",
            "id": 5,
            "node_id": "user",
            "avatar_url": url,
            "gravatar_id": "",
            "url": url,
            "html_url": url,
            "followers_url": url,
            "following_url": url,
            "gists_url": url,
            "starred_url": url,
            "subscriptions_url": url,
            "organizations_url": url,
            "repos_url": url,
            "events_url": url,
            "received_events_url": url,
            "type": "User",
            "site_admin": false
        }
    }))
    .unwrap();

    assert_event(
        Event::ConversationComment(&comment),
        "conversation_comment",
        &comment,
    );
}

#[test]
fn check_run_event_preserves_payload() {
    let check: CheckRun = serde_json::from_value(json!({
        "id": 6,
        "node_id": "check-run",
        "head_sha": "abc",
        "url": "https://api.github.com/repos/owner/repo/check-runs/6",
        "name": "cargo test",
        "conclusion": "failure",
        "output": {
            "title": "Tests failed",
            "summary": "One failure\nSee logs",
            "annotations_count": 0,
            "annotations_url": "https://api.github.com/repos/owner/repo/check-runs/6/annotations"
        },
        "pull_requests": []
    }))
    .unwrap();

    assert_event(Event::CheckRun(&check), "check_run", &check);
}

#[test]
fn head_commit_event_preserves_payload() {
    let url = "https://api.github.com/repos/owner/repo/git/commits/abc";
    let commit: RepoCommit = serde_json::from_value(json!({
        "sha": "abc",
        "node_id": "commit",
        "url": url,
        "html_url": "https://github.com/owner/repo/commit/abc",
        "comments_url": "https://api.github.com/repos/owner/repo/commits/abc/comments",
        "author": null,
        "committer": null,
        "parents": [{"sha": "def"}],
        "commit": {
            "url": url,
            "message": "Add event wrappers\n\nPreserve existing payloads.",
            "comment_count": 0,
            "tree": {"sha": "tree", "url": url}
        }
    }))
    .unwrap();

    assert_event(Event::HeadCommit(&commit), "head_commit", &commit);
}
