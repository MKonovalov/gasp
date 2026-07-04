# Design brief — GASP homepage

Paste this into claude.ai/design as the prompt.

---

Design a homepage for **GASP — the Git Agent State Protocol** (github.com/yologdev/gasp), a git-native open standard for portable AI agent state.

## The one idea (hero)

**The repo is the agent.** An agent is stateless; its durable state is an
append-only event log that folds into a queryable graph of goals, patches,
evals, and decisions — committed into a git repo. Clone the repo, fold the
log, and the same agent resumes anywhere, on any model. Swap the executor
freely; the state survives it.

Tagline candidates (pick or riff):
- "The repo is the agent."
- "Portable, durable state for AI agents — in plain git."
- "Your agent's memory shouldn't die with its runtime."

## Audience & tone

Developers building AI agents (Rust/TS/Python), skeptical of frameworks and
vendor lock-in. Tone: precise, engineering-grade, a little austere — a
*protocol*, not a product. Think RFC meets git: monospace accents, log lines,
commit graphs. Confidence through specificity, zero marketing fluff. Dark
theme fits the terminal-native audience but is not mandatory.

## Story arc (sections in order)

1. **Hero** — the one idea above + two CTAs: "Read the spec" (SPEC.md) and
   "Run the checker" (`conformance-check <agent-repo>`).

2. **The problem** — agent state today is trapped: transcripts rot in vendor
   folders, memory dies with the runtime, and nothing explains *why* a change
   exists. Swapping models means amnesia.

3. **How it works** — three beats, ideally as a visual:
   - **Append** — every semantic moment is one JSONL line: goals, runs, tool
     calls, evals, decisions. A real log line to show (monospace):
     `{"kind":"eval.finished","payload":{"command":"cargo test retry","status":"Passed"},...}`
   - **Fold** — the log folds into a typed graph. The causal spine:
     `goal → run → patch → eval → decision → promotion`
   - **Ship** — git commits it, git ships it. Restore = `clone + replay`.

4. **Conformance is a test, not a claim** — five rules, seven mechanical
   checks, one canonical fixture. `conformance-check` exits non-zero on any
   violation: envelope round-trip, replay, vocabulary, append-only-in-git,
   causation integrity, restore, domain↔ops pairing. A terminal block showing
   the checker's real output would land well:
   ```
   [PASS] check 1 — envelope round-trip
   [PASS] check 2 — replay
   ...
   [PASS] check 7 — domain↔ops consistency
   conformant: all checks passed
   ```

5. **Proof: yoyo lives here** — yoyo, a self-evolving agent with 100k+ lines
   of self-written Rust (github.com/yologdev/yoyo-evolve), keeps its identity,
   skills, memory, and lineage in a GASP repo (github.com/yologdev/yoyo-gasp).
   Its state repo passes all 7 checks. "Clone yoyo-gasp and yoyo wakes up
   knowing who it is."

6. **The ecosystem** (cards/links):
   - **SPEC.md** — the normative protocol (Part I)
   - **conformance-check** — fixture + 7-check CLI, in this repo
   - **yoagent-state** — reference runtime, Rust, on crates.io
     (crates.io/crates/yoagent-state) — GitEventStore, lineage, replay, fork
   - **yoyo-gasp** — the first living agent repo
   - Adapters for closed agents (Claude Code, Codex) — spec'd, coming

7. **Footer** — MIT. Credit line: "The 'log is the agent' idea descends from
   Yohei Nakajima's ActiveGraph; yoagent-state is an independent Rust
   implementation. GASP binds it to git as the interchange substrate."

## Visual raw material (use any)

- The ten-line worked example from the spec (goal → patch → eval → decision,
  real JSONL) — great for a scrolling/typing log animation.
- The repo layout tree (identity/, skills/, state/events.jsonl, memory/,
  journal/, snapshots/, .agent/) — the anatomy of an agent.
- Commit-graph / DAG motifs: events as nodes, causation as edges.
- The motto layering: "yoagent executes. yoagent-state remembers.
  yoyo evolve improves. git ships."

## Avoid

- Generic AI imagery (robots, brains, sparkles), purple-gradient SaaS look.
- Overpromising: GASP is v1 and honest about sharp edges — the spec has a
  whole section of caveats. Confidence, not hype.
