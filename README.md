# GASP — The Graph Agent State Protocol

A git-native standard for portable agent state: an agent's append-only event log
folds into a typed graph of goals, patches, evals, and decisions; the repo — not
the model or the runtime — is the agent. Point any GASP-conformant runtime at the
repo URL and the same agent resumes anywhere, on any model.

- **[SPEC.md](SPEC.md)** — the protocol (Part I, normative), reference runtime
  ([`yoagent-state`](https://github.com/yologdev/yoagent-state)), adapters for
  closed agents, the reference agent
  ([`yoyo-evolve`](https://github.com/yologdev/yoyo-evolve)), sharp edges, and
  the conformance kit.
- **[fixture/](fixture)** — the canonical agent-repo every conformant runtime
  must restore.
- **[conformance-check/](conformance-check)** — the checker every emitter must
  pass.

```sh
cargo run -q -- fixture --fixture   # verify the fixture (all 7 checks)
cargo run -q -- path/to/agent-repo  # verify any emitted repo
cargo test                          # kit self-tests (incl. corrupted-fixture negatives)
```

Credit: the "log is the agent" idea descends from
[Yohei Nakajima's ActiveGraph](https://github.com/yoheinakajima/activegraph);
`yoagent-state` is an independent Rust implementation of it, and GASP binds that
idea to git as the interchange substrate.
