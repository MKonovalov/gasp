use conformance_check::*;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixture")
}

fn fixture_lines() -> Vec<String> {
    read_log_lines(&fixture_dir()).expect("fixture log readable")
}

#[test]
fn fixture_passes_all_checks() {
    let reports = run_all(&fixture_dir(), true).expect("fixture parses");
    for report in &reports {
        assert!(
            report.passed(),
            "check {} ({}) failed: {:?}",
            report.number,
            report.name,
            report.failures
        );
    }
}

#[test]
fn envelope_rejects_unknown_field() {
    let mut lines = fixture_lines();
    lines[0] = lines[0].replace("\"causation_id\"", "\"extra\":1,\"causation_id\"");
    assert!(!check_envelope(&lines).passed());
}

#[test]
fn envelope_rejects_missing_field() {
    let lines = vec![r#"{"id":"event_x","kind":"goal.created","payload":{}}"#.to_string()];
    assert!(!check_envelope(&lines).passed());
}

#[test]
fn vocabulary_rejects_undeclared_kind() {
    let mut lines = fixture_lines();
    lines[1] = lines[1].replace("\"kind\":\"goal\"", "\"kind\":\"vibe\"");
    let events = parse_events(&lines).unwrap();
    let report = check_vocabulary(&fixture_dir(), &events);
    assert!(!report.passed(), "expected undeclared kind `vibe` to fail");
}

#[test]
fn causation_rejects_dangling_reference() {
    let mut lines = fixture_lines();
    lines[9] = lines[9].replace("\"causation_id\":\"event_09\"", "\"causation_id\":\"event_99\"");
    let events = parse_events(&lines).unwrap();
    assert!(!check_causation(&events).passed());
}

#[test]
fn causation_rejects_bad_root() {
    let mut lines = fixture_lines();
    // make eval.finished a root
    lines[6] = lines[6].replace("\"causation_id\":\"event_06\"", "\"causation_id\":null");
    let events = parse_events(&lines).unwrap();
    assert!(!check_causation(&events).passed());
}

#[test]
fn pairing_rejects_missing_ops_event() {
    let mut lines = fixture_lines();
    lines.remove(1); // drop goal.created's paired ops event
    let events = parse_events(&lines).unwrap();
    let report = check_pairing(&events);
    assert!(!report.passed(), "goal.created without paired ops must fail");
}

#[test]
fn pairing_rejects_status_contradiction() {
    let mut lines = fixture_lines();
    // decision.created says Approved; make its ops event create it Rejected
    lines[9] = lines[9].replace(
        "{\"CreateNode\":{\"id\":\"decision_3\",\"kind\":\"decision\",\"props\":{\"status\":\"Approved\"}}}",
        "{\"CreateNode\":{\"id\":\"decision_3\",\"kind\":\"decision\",\"props\":{\"status\":\"Rejected\"}}}",
    );
    let events = parse_events(&lines).unwrap();
    let report = check_pairing(&events);
    assert!(!report.passed(), "status contradiction must fail");
}

#[test]
fn restore_asserts_fixture_facts() {
    let mut lines = fixture_lines();
    // break the promotion: drop the UpdateNode from the final ops event
    lines[9] = lines[9].replace(
        ",{\"UpdateNode\":{\"id\":\"patch_9\",\"props\":{\"status\":\"Promoted\"}}}",
        "",
    );
    let events = parse_events(&lines).unwrap();
    let report = check_restore(&fixture_dir(), &events, true);
    assert!(!report.passed(), "unpromoted patch_9 must fail fixture facts");
}

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?} failed");
}

#[test]
fn append_only_rejects_history_rewrite() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    std::fs::create_dir_all(repo.join("state")).unwrap();
    let log = repo.join("state/events.jsonl");

    git(repo, &["init", "-q"]);
    std::fs::write(&log, "line one\nline two\n").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-qm", "one"]);

    // append-only extension is fine
    std::fs::write(&log, "line one\nline two\nline three\n").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-qm", "two"]);
    assert!(check_append_only(repo).passed());

    // in-place edit of an existing line must fail
    std::fs::write(&log, "line ONE\nline two\nline three\n").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-qm", "three"]);
    assert!(!check_append_only(repo).passed());
}
