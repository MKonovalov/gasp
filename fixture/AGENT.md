# AGENT — GASP manifest

```yaml
spec_version: 1
agent_id: gasp-fixture
identity_hash: 1acb93a8eac071f3ca7bdf6f229887f3711ce63b0ef9052b87162b33cc805c7f
executor: .agent/config.toml
```

The identity hash is SHA-256 over each `identity/` file's relative path followed by a
newline and its bytes, in sorted path order:

```
for f in $(find identity -type f | sort); do printf '%s\n' "$f"; cat "$f"; done | shasum -a 256
```

An identity change updates `identity/`, this hash, and appends a `decision` event —
all in one human-gated commit (Part I, commit rule 4).

## Locations

| GASP role | path |
|---|---|
| event log | `state/events.jsonl` |
| identity | `identity/` |
| skills | `skills/` |
