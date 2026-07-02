//! GASP conformance checks (SPEC.md Part VI).
//!
//! Each check takes the parsed log (and/or the repo path) and returns a
//! `CheckReport`. `run_all` runs the seven checks in spec order.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use yoagent_state::{replay, Event, Graph, NodeId, Pack, StateOp};

pub const EVENTS_PATH: &str = "state/events.jsonl";

/// Append-only paths per Part I commit rule 1, checked when present.
pub const APPEND_ONLY_PATHS: &[&str] = &[
    "state/events.jsonl",
    "memory/facts.jsonl",
    "journal/JOURNAL.md",
    "JOURNAL.md",
];

const ENVELOPE_KEYS: &[&str] = &[
    "id",
    "schema_version",
    "ts_ms",
    "actor",
    "kind",
    "payload",
    "causation_id",
    "correlation_id",
];

const BASELINE_NODE_KINDS: &[&str] = &[
    "goal",
    "task",
    "run",
    "observation",
    "failure",
    "hypothesis",
    "patch",
    "eval",
    "decision",
    "model_call",
    "tool_call",
    "frame",
    "project_snapshot",
    "policy",
    "behavior",
];

const BASELINE_RELS: &[&str] = &[
    "serves",
    "blocks",
    "advances",
    "observes",
    "explains",
    "addresses",
    "modifies",
    "validated_by",
    "approved_by",
    "rejected_by",
    "produced_by",
    "derived_from",
    "depends_on",
    "supersedes",
    "contained_in_frame",
    "forked_from",
    "references",
];

/// Domain-event kinds that create an entity and therefore REQUIRE a paired
/// `state.ops_applied` (check 7), mapped to the node kind the pair must create.
const PAIRED_KINDS: &[(&str, &str)] = &[
    ("goal.created", "goal"),
    ("task.created", "task"),
    ("observation.created", "observation"),
    ("failure.observed", "failure"),
    ("hypothesis.created", "hypothesis"),
    ("patch.proposed", "patch"),
    ("eval.finished", "eval"),
    ("decision.created", "decision"),
    ("frame.created", "frame"),
];

#[derive(Debug)]
pub struct CheckReport {
    pub number: u8,
    pub name: &'static str,
    pub failures: Vec<String>,
    pub notes: Vec<String>,
}

impl CheckReport {
    fn new(number: u8, name: &'static str) -> Self {
        Self {
            number,
            name,
            failures: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

pub fn read_log_lines(repo: &Path) -> Result<Vec<String>, String> {
    let path = repo.join(EVENTS_PATH);
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    Ok(raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect())
}

pub fn parse_events(lines: &[String]) -> Result<Vec<Event>, String> {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            serde_json::from_str::<Event>(line).map_err(|e| format!("line {}: {e}", i + 1))
        })
        .collect()
}

fn ops_of(event: &Event) -> Option<Vec<StateOp>> {
    if event.kind != "state.ops_applied" {
        return None;
    }
    serde_json::from_value(event.payload.clone()).ok()
}

/// Check 1 — envelope round-trip: exact key set, parses to the envelope,
/// re-serializes structurally equal.
pub fn check_envelope(lines: &[String]) -> CheckReport {
    let mut report = CheckReport::new(1, "envelope round-trip");
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                report.failures.push(format!("line {n}: not JSON: {e}"));
                continue;
            }
        };
        let Some(obj) = value.as_object() else {
            report.failures.push(format!("line {n}: not a JSON object"));
            continue;
        };
        for key in ENVELOPE_KEYS {
            if !obj.contains_key(*key) {
                report
                    .failures
                    .push(format!("line {n}: missing envelope field `{key}`"));
            }
        }
        for key in obj.keys() {
            if !ENVELOPE_KEYS.contains(&key.as_str()) {
                report
                    .failures
                    .push(format!("line {n}: unknown top-level field `{key}`"));
            }
        }
        match serde_json::from_str::<Event>(line) {
            Ok(event) => {
                let reserialized = serde_json::to_value(&event).expect("event serializes");
                if reserialized != value {
                    report
                        .failures
                        .push(format!("line {n}: does not round-trip structurally"));
                }
            }
            Err(e) => report
                .failures
                .push(format!("line {n}: not a valid envelope: {e}")),
        }
    }
    report
}

/// Check 2 — replay determinism, plus snapshot integrity + snapshot-equivalence
/// when `snapshots/` exists.
pub fn check_replay(repo: &Path, events: &[Event], lines: &[String]) -> CheckReport {
    let mut report = CheckReport::new(2, "replay determinism");
    let first = replay(events);
    let second = replay(events);
    match (first, second) {
        (Ok(a), Ok(b)) => {
            if a != b {
                report
                    .failures
                    .push("folding the log twice yielded different graphs".into());
            }
        }
        (Err(e), _) | (_, Err(e)) => {
            report.failures.push(format!("fold failed: {e}"));
            return report;
        }
    }

    let snapshots = repo.join("snapshots");
    if !snapshots.is_dir() {
        report.notes.push("no snapshots/ — skipped seed check".into());
        return report;
    }
    let Ok(entries) = std::fs::read_dir(&snapshots) else {
        report.failures.push("cannot read snapshots/".into());
        return report;
    };
    for entry in entries.flatten() {
        let graph_path = entry.path().join("graph.json");
        if !graph_path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let raw = match std::fs::read_to_string(&graph_path) {
            Ok(r) => r,
            Err(e) => {
                report.failures.push(format!("snapshot {name}: unreadable: {e}"));
                continue;
            }
        };
        let value: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                report.failures.push(format!("snapshot {name}: not JSON: {e}"));
                continue;
            }
        };
        let (Some(graph_v), Some(integrity)) = (value.get("graph"), value.get("integrity")) else {
            report
                .failures
                .push(format!("snapshot {name}: missing `graph` or `integrity` record"));
            continue;
        };
        let line_count = integrity
            .get("line_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        let expected_sha = integrity
            .get("sha256")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if line_count == 0 || line_count > lines.len() {
            report
                .failures
                .push(format!("snapshot {name}: line_count {line_count} out of range"));
            continue;
        }
        let prefix = lines[..line_count].join("\n") + "\n";
        let actual_sha = hex(&Sha256::digest(prefix.as_bytes()));
        if actual_sha != expected_sha {
            report.failures.push(format!(
                "snapshot {name}: integrity hash mismatch (log prefix changed?)"
            ));
            continue;
        }
        let snapshot_graph: Graph = match serde_json::from_value(graph_v.clone()) {
            Ok(g) => g,
            Err(e) => {
                report
                    .failures
                    .push(format!("snapshot {name}: graph does not parse: {e}"));
                continue;
            }
        };
        let prefix_fold = match replay(&events[..line_count]) {
            Ok(g) => g,
            Err(e) => {
                report
                    .failures
                    .push(format!("snapshot {name}: prefix fold failed: {e}"));
                continue;
            }
        };
        if prefix_fold != snapshot_graph {
            report.failures.push(format!(
                "snapshot {name}: snapshot + tail differs from folding from scratch"
            ));
        }
    }
    report
}

fn load_packs(repo: &Path) -> Vec<Pack> {
    let dir = repo.join(".agent/packs");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|raw| serde_json::from_str::<Pack>(&raw).ok())
        .collect()
}

/// Check 3 — vocabulary: node kinds and relation kinds are baseline or declared
/// by a pack in `.agent/packs/*.json`.
pub fn check_vocabulary(repo: &Path, events: &[Event]) -> CheckReport {
    let mut report = CheckReport::new(3, "vocabulary");
    let packs = load_packs(repo);
    let mut kinds: HashSet<&str> = BASELINE_NODE_KINDS.iter().copied().collect();
    let mut rels: HashSet<&str> = BASELINE_RELS.iter().copied().collect();
    for pack in &packs {
        kinds.extend(pack.object_types.keys().map(String::as_str));
        rels.extend(pack.relation_types.keys().map(String::as_str));
    }
    for (i, event) in events.iter().enumerate() {
        let Some(ops) = ops_of(event) else { continue };
        for op in ops {
            match op {
                StateOp::CreateNode { kind, id, .. } if !kinds.contains(kind.as_str()) => {
                    report.failures.push(format!(
                        "line {}: CreateNode {} uses undeclared kind `{kind}`",
                        i + 1,
                        id.as_str()
                    ));
                }
                StateOp::CreateRelation { rel, from, .. } if !rels.contains(rel.as_str()) => {
                    report.failures.push(format!(
                        "line {}: CreateRelation from {} uses undeclared rel `{rel}`",
                        i + 1,
                        from.as_str()
                    ));
                }
                _ => {}
            }
        }
    }
    report
}

fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("git failed to spawn: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Check 4 — append-only-in-git: for every commit touching an append-only path,
/// the previous version must be a byte prefix of the next (additions at EOF
/// only). The working tree must extend the last committed version.
pub fn check_append_only(repo: &Path) -> CheckReport {
    let mut report = CheckReport::new(4, "append-only in git");
    if git(repo, &["rev-parse", "HEAD"]).is_err() {
        report
            .notes
            .push("no git history — nothing to violate yet".into());
        return report;
    }
    for path in APPEND_ONLY_PATHS {
        let Ok(shas) = git(repo, &["rev-list", "--reverse", "HEAD", "--", path]) else {
            continue;
        };
        let shas: Vec<&str> = shas.split_whitespace().collect();
        if shas.is_empty() {
            continue;
        }
        let spec = |sha: &str| format!("{sha}:./{path}");
        let mut prev: Option<String> = None;
        for sha in &shas {
            match git(repo, &["show", &spec(sha)]) {
                Ok(content) => {
                    if let Some(prev) = &prev {
                        if !content.starts_with(prev.as_str()) {
                            report.failures.push(format!(
                                "{path}: commit {} edits existing lines (not append-only)",
                                &sha[..7.min(sha.len())]
                            ));
                        }
                    }
                    prev = Some(content);
                }
                Err(_) => {
                    // Path touched but absent in this commit = deleted.
                    if prev.is_some() {
                        report.failures.push(format!(
                            "{path}: deleted in commit {}",
                            &sha[..7.min(sha.len())]
                        ));
                        prev = None;
                    }
                }
            }
        }
        if let (Some(prev), Ok(on_disk)) = (&prev, std::fs::read_to_string(repo.join(path))) {
            if !on_disk.starts_with(prev.as_str()) {
                report
                    .failures
                    .push(format!("{path}: working tree edits committed lines"));
            }
        }
    }
    report
}

/// Check 5 — causation integrity: every non-null causation_id references an
/// EARLIER event; roots (null causation) are `*.created` / `*.started`.
pub fn check_causation(events: &[Event]) -> CheckReport {
    let mut report = CheckReport::new(5, "causation integrity");
    let mut seen: HashSet<&str> = HashSet::new();
    for (i, event) in events.iter().enumerate() {
        let n = i + 1;
        match &event.causation_id {
            Some(cause) => {
                if !seen.contains(cause.as_str()) {
                    report.failures.push(format!(
                        "line {n}: causation_id {} does not reference an earlier event",
                        cause.as_str()
                    ));
                }
            }
            None => {
                if !(event.kind.ends_with(".created") || event.kind.ends_with(".started")) {
                    report.failures.push(format!(
                        "line {n}: root event (null causation) has kind `{}`, expected *.created / *.started",
                        event.kind
                    ));
                }
            }
        }
        seen.insert(event.id.as_str());
    }
    report
}

/// Check 6 — restore: manifest + identity present, the log folds. With
/// `fixture_facts`, additionally asserts the Part VI fixture graph.
pub fn check_restore(repo: &Path, events: &[Event], fixture_facts: bool) -> CheckReport {
    let mut report = CheckReport::new(6, "restore");
    if !repo.join("AGENT.md").is_file() {
        report.failures.push("AGENT.md manifest missing".into());
    }
    let identity_ok = repo.join("identity").is_dir() || repo.join("IDENTITY.md").is_file();
    if !identity_ok {
        report
            .failures
            .push("no identity/ directory or IDENTITY.md".into());
    }
    let graph = match replay(events) {
        Ok(g) => g,
        Err(e) => {
            report.failures.push(format!("fold failed: {e}"));
            return report;
        }
    };
    if !fixture_facts {
        return report;
    }

    let node = |id: &str| graph.get_node(&NodeId::new(id));
    let prop = |id: &str, key: &str| -> Option<Value> {
        node(id).and_then(|n| n.props.get(key)).cloned()
    };
    let edge = |from: &str, rel: &str, to: &str| -> bool {
        graph
            .outgoing(&NodeId::new(from), Some(rel))
            .iter()
            .any(|r| r.to.as_str() == to)
    };

    if node("goal_retry").is_none() {
        report.failures.push("fixture: goal_retry missing".into());
    }
    if !edge("patch_9", "advances", "goal_retry") {
        report
            .failures
            .push("fixture: patch_9 --advances--> goal_retry missing".into());
    }
    if prop("patch_9", "status") != Some(Value::from("Promoted")) {
        report.failures.push("fixture: patch_9.status != Promoted".into());
    }
    if prop("patch_9", "references_commit") != Some(Value::from("abc1234")) {
        report
            .failures
            .push("fixture: patch_9.references_commit != abc1234".into());
    }
    if !edge("patch_9", "validated_by", "eval_5")
        || prop("eval_5", "status") != Some(Value::from("Passed"))
    {
        report
            .failures
            .push("fixture: patch_9 validated_by passed eval_5 missing".into());
    }
    if !edge("patch_9", "approved_by", "decision_3")
        || prop("decision_3", "status") != Some(Value::from("Approved"))
    {
        report
            .failures
            .push("fixture: patch_9 approved_by approved decision_3 missing".into());
    }
    report
}

/// Check 7 — domain↔ops consistency (the pairing rule): every entity-creating
/// domain event has a paired ops event (causation = domain id) whose CreateNode
/// matches the payload's id, kind, and status.
pub fn check_pairing(events: &[Event]) -> CheckReport {
    let mut report = CheckReport::new(7, "domain↔ops consistency");
    let paired: HashMap<&str, &str> = PAIRED_KINDS.iter().copied().collect();
    let by_id: HashMap<&str, &Event> = events.iter().map(|e| (e.id.as_str(), e)).collect();

    // ops events indexed by the domain event they claim to apply
    let mut ops_for: HashMap<&str, &Event> = HashMap::new();
    for event in events {
        if event.kind == "state.ops_applied" {
            if let Some(cause) = &event.causation_id {
                ops_for.insert(cause.as_str(), event);
            }
        }
    }

    for (i, event) in events.iter().enumerate() {
        let n = i + 1;
        let Some(expected_kind) = paired.get(event.kind.as_str()) else {
            continue;
        };
        let Some(entity_id) = event.payload.get("id").and_then(Value::as_str) else {
            report.failures.push(format!(
                "line {n}: {} payload has no `id`",
                event.kind
            ));
            continue;
        };
        let Some(ops_event) = ops_for.get(event.id.as_str()) else {
            report.failures.push(format!(
                "line {n}: {} `{entity_id}` has no paired state.ops_applied",
                event.kind
            ));
            continue;
        };
        let Some(ops) = ops_of(ops_event) else {
            report.failures.push(format!(
                "line {n}: paired ops event {} has malformed payload",
                ops_event.id.as_str()
            ));
            continue;
        };
        let create = ops.iter().find_map(|op| match op {
            StateOp::CreateNode { id, kind, props } if id.as_str() == entity_id => {
                Some((kind.clone(), props.clone()))
            }
            _ => None,
        });
        let Some((kind, props)) = create else {
            report.failures.push(format!(
                "line {n}: paired ops for {} do not create node `{entity_id}`",
                event.kind
            ));
            continue;
        };
        if kind != *expected_kind {
            report.failures.push(format!(
                "line {n}: `{entity_id}` created as kind `{kind}`, domain event implies `{expected_kind}`"
            ));
        }
        if let (Some(evt_status), Some(node_status)) =
            (event.payload.get("status"), props.get("status"))
        {
            if evt_status != node_status {
                report.failures.push(format!(
                    "line {n}: `{entity_id}` status contradicts domain event ({evt_status} vs {node_status})"
                ));
            }
        }
    }

    // The reverse direction: an ops event claiming causation from a paired
    // domain event must not create a node whose id is that event's entity with
    // a different kind (contradiction), which the loop above already covers.
    // Here we only verify claimed causations exist as domain events.
    for event in events {
        if event.kind != "state.ops_applied" {
            continue;
        }
        if let Some(cause) = &event.causation_id {
            if let Some(domain) = by_id.get(cause.as_str()) {
                if domain.kind == "state.ops_applied" {
                    report.failures.push(format!(
                        "ops event {} chained to another ops event, not a domain event",
                        event.id.as_str()
                    ));
                }
            }
        }
    }
    report
}

pub fn run_all(repo: &Path, fixture_facts: bool) -> Result<Vec<CheckReport>, String> {
    let lines = read_log_lines(repo)?;
    let events = parse_events(&lines)?;
    Ok(vec![
        check_envelope(&lines),
        check_replay(repo, &events, &lines),
        check_vocabulary(repo, &events),
        check_append_only(repo),
        check_causation(&events),
        check_restore(repo, &events, fixture_facts),
        check_pairing(&events),
    ])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
