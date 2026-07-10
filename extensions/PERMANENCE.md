# GASP Permanence Extension — Durable, Retrievable, Owned Agent State

**Status:** DRAFT — an exploratory extension to GASP (Git Agent State Protocol). Optional; core conformance does not require permanence. Shared as open thinking; feedback welcome.
**Scope note:** §4–5 summarize the anchor layer; a companion deep dive (per-chain cost survey, why OpenTimestamps beats direct-chain anchoring) may follow as a separate doc. This doc is self-contained.
**Costs herein:** SOL ≈ $81, BTC ≈ $60k, ETH ≈ $1,700, Arweave ≈ $0.015/MB, as of July 2026. Prices move; the ordering does not.

---

## 1. Goal: an agent that outlives every system holding it

"Digital immortality" of an agent means three properties at once:

- **Durable.** The state survives the death of any single host, remote, company, or chain.
- **Retrievable.** Anyone with the right to it can fetch and reconstruct the agent, forever, without a specific server being alive.
- **Owned.** It stays private and under the holder's control, not world-readable by default.

**What must perdure** is the small payload, not everything: the semantic event log (`state/events.jsonl`, GASP's source of truth) plus `identity/` and `skills/`. Raw transcripts stay cold and prunable and are out of scope. This is a few megabytes for a mature agent, and that smallness is what makes permanence cheap.

The key reframe: **immortality is a redundancy-and-retrievability property, not a storage-location property.** A thing is immortal when it survives as N independent, content-addressed copies, not because the one copy sits on a famous chain. This shapes the entire design below.

## 2. Why "just put it on Solana / Bitcoin" does not work

"The chain is forever" guarantees the *ledger* persists. It guarantees *your data* only if your data is in the ledger, which general-purpose chains price out on purpose, because every validator must carry your bytes in live state forever. That cost is the mechanism that keeps them decentralized. So the property you want (cheap + permanent) is exactly the one those chains are engineered not to sell. Each chain uses a *different* pricing mechanism, which is why the numbers look so different, but all of them punish payload-on-L1. Sizing the ~5 MB payload:

| Where | Pricing mechanism | Cost for ~5 MB | Verdict |
|---|---|---|---|
| Bitcoin (Ordinals/inscription) | fee per vByte | **~$3,000–6,000** at realistic fee rates, spikes far higher | Most expensive; fee-driven, volatile |
| Ethereum L1 (contract storage) | gas per byte (625 gas/byte via SSTORE) | thousands per MB on quiet days, tens of thousands when busy; the full 5 MB can crest six figures | Only ever used for KB-scale generative art |
| Solana (account storage) | **rent deposit**, ~7 SOL/MB | ~$2,700–2,800 locked | Refundable only if you delete the data |
| Base / L2 (blobs) | per-byte, **pruned ~18 days** | cheap but **not permanent** | Disqualified: blobs expire by design |
| **Arweave (pay-once)** | one-time endowment | **~$0.05, forever** | The only cheap-and-permanent option |

Three corrections worth stating precisely, because they are the common challenges:

- **Bitcoin.** The Ordinals handbook's own floor is $50 per 1M bytes at 1 sat/vByte and BTC $20k. Real inscriptions run 4–8 sat/vByte (spiking to 50+), and BTC is ~$60k now, so ~5 MB realistically lands in the low thousands, not hundreds. It is the most expensive option, by a wide margin, and volatile with congestion.
- **Solana is rent, not fees.** On-chain storage is not priced by transaction fee; you deposit ~2 years of rent to make an account rent-exempt, scaling at ~7 SOL/MB, because the data sits in validator live state (RAM-tier), not cold archive. The deposit never leaves your ownership (it sits inside the account, like a security deposit) and is recovered *only by closing the account and deleting the data*, which is the opposite of permanence. So for an immortal agent it is a real, effectively non-recoverable ~$2,700.
- **"But there are tons of on-chain NFTs on Ethereum/Base."** True, and not a counterexample — we verified both patterns directly against contracts on Base. Almost all collections are the *pointer* pattern (`tokenURI` resolves to IPFS/Arweave), so the artwork is not on-chain at all; one checked collection's contract returned an `ipfs://` URI. The minority that are genuinely on-chain are **KB-scale generative art**: a second checked collection returned a `data:application/json;base64,…` URI of ~11 KB with the SVG embedded — real on-chain storage, affordable precisely because it is tiny. Neither case is "5 MB of payload on the chain." Nobody puts real payloads on an L1 in any ecosystem.

The universal takeaway: **every ecosystem avoids payload-on-L1** and converges on a tiny on-chain pointer plus off-chain content-addressed storage (Arweave/IPFS). GASP's permanence layer adopts that converged pattern, with one change: because an agent's state is intimate, **the stored payload is encrypted** (Section 6).

## 3. The three-layer stack (each orthogonal, each pluggable)

Permanence is not one product. It is an assembly of three independent guarantees, each with a swappable backend, exactly like GASP's `EventStore` and anchor backends.

| Layer | Job it does | Reference backend | Alternatives |
|---|---|---|---|
| **Substrate** | working state, fork, merge, restore | git | (none; this is GASP core) |
| **Redundancy** | host-independent availability | **Radicle** (git-native P2P) | IPFS (cache; not permanent) |
| **Permanence** | pay-once forever floor | **Arweave** (encrypted payload) | Irys (future; see §8) |
| **Provenance** | proof a state existed at time T, un-rewritten | **OpenTimestamps** on Bitcoin (free) | direct-chain anchor |

The reference stack is **Radicle + Arweave + OpenTimestamps**: the most decentralized, host-independent combination available today, with no company in any loop. Redundancy answers "where does it live so no host can kill it." Permanence answers "and it is still fetchable in ten years." Provenance answers "and here is Bitcoin-grade proof of when each state existed." None overlaps the others.

## 4. The single-hash pivot (why there is only ever one OTS record)

The three systems do not hold three different states needing three timestamps. **Local git, Radicle, and OTS all pivot on one value: the git commit SHA.**

- A git commit hash is already a Merkle root over the entire history.
- Radicle is a peer-to-peer transport on top of the same git object store. When you push commit `abc123`, every peer replicates `abc123` byte-for-byte, same SHA. Radicle adds signed refs and a repo identity *around* your commits; the commit objects are unchanged.
- OpenTimestamps stamps a hash.

So there is one hash in the system that matters, all three layers already agree on it, and therefore **one OTS stamp per run boundary, no duplication.**

**Stamp the commit, not the Radicle signed ref.** Radicle also signs its own object ("peer P says the canonical head is abc123"), which is a different byte string. Stamping that would be a second, unnecessary record. The commit carries the agent; the Radicle signature is only distribution consensus (which peer's head to trust). For provenance of the agent's state and history, the commit SHA is the complete and correct target. Only stamp the Radicle ref if you specifically need to prove Radicle-level canonicality at a time, which GASP does not require.

## 5. The anchoring flow (per run boundary)

1. Run closes (GASP's one-closing-commit-per-run rule). Take the head commit SHA.
2. OTS-stamp the SHA: submit to calendar servers, receive a pending `.ots` proof.
3. Commit the proof into the repo at `anchors/<sha>.ots` and append a permanence record (Section 7).
4. Push to Radicle; upload the encrypted payload to Arweave. The `.ots` travels *inside* the repo, so every Radicle peer and the Arweave copy carry the proof automatically.

**Pending then upgrade.** The `.ots` from step 2 is incomplete until Bitcoin confirms the calendar's batch (up to a few hours). An `upgrade` step then fills in the complete Merkle path to the Bitcoin block, after which the proof is self-contained forever and the calendar servers can disappear. In an append-only repo this is clean: the upgraded proof lands in a later commit on top. Still one logical OTS record per run; it just finalizes one commit after creation.

**Verification, by anyone, without trusting the source.** Given the payload plus the `.ots`, a verifier recomputes the hash, walks the Merkle path to the on-chain root, and reads the Bitcoin block timestamp. It needs only the file, the proof, and a view of Bitcoin. Not the calendar servers, not the Radicle peer it fetched from. This is what makes peer-to-peer distribution safe: the OTS proof answers "can I trust this copy?" independently of who served it.

## 6. Encrypt-then-store, and the key-survival problem

Permanent plus public equals intimate agent state that is world-readable forever and impossible to delete. That is the opposite of the ownership goal. The resolution:

**Encrypt the payload before it goes to Arweave, IPFS, or public Radicle peers.** The ciphertext is what becomes immortal and public; the key is the ownership boundary. The commit SHA and the OTS proof stay public and leak nothing (a hash reveals nothing about content), so provenance remains publicly verifiable while the content stays private.

This converts digital immortality into a **key-survival problem**, which is the genuinely hard, partly-unsolved part: if the key dies, the agent is immortal *noise*. Key survival is its own design surface, options include Shamir/threshold split across custodians, social recovery, and sealed hardware handoff, and this doc flags it rather than pretending it is solved. Treat key management as a first-class requirement of the permanence layer, not an afterthought.

A consequence worth stating plainly: **crypto-shredding (destroying the key) is the only available "delete."** Once ciphertext is on Arweave it is there forever; the sole way to make it unrecoverable is to ensure the key no longer exists. Right-to-be-forgotten is therefore all-or-nothing per key, and per-item deletion is impossible. Design key scopes with that in mind.

**Public agents may skip encryption entirely.** "Owned" means the holder decides, and a deliberately transparent agent (yoyo is one: its state is already public) can store plaintext. For such agents the key-survival problem vanishes, at the price of a one-way door: public-permanent can never be retracted, so the upload step needs a redaction check before, not regret after.

## 7. The permanence record (retrievability manifest)

Append-only, one record per anchored run, committed into the repo at `permanence/manifest.jsonl`. This is the artifact that makes the state *retrievable*, not merely provable: given one record plus the decryption key, you can find, verify, and reconstruct the agent.

```json
{
  "commit": "abc123...",
  "run_id": "01J...",
  "ts": "2026-07-09T12:00:00Z",
  "payload": { "scope": ["state/events.jsonl", "identity/", "skills/"], "sha256": "…", "bytes": 4712345 },
  "encryption": { "algo": "age", "recipients": ["<key-id>"], "encrypted": true },
  "storage": [
    { "backend": "arweave",  "locator": "ar://<txid>",  "status": "permanent" },
    { "backend": "radicle",  "locator": "rad:<rid>",    "status": "seeded" },
    { "backend": "ipfs",     "locator": "ipfs://<cid>",  "status": "pinned" }
  ],
  "anchor": { "backend": "opentimestamps", "target": "commit", "proof": "anchors/abc123.ots", "status": "attested" }
}
```

`encryption` names the algorithm and key id, never the key. `storage` lists every place the encrypted payload can be fetched; more entries means more independent survivors.

**Restore-from-immortality** (the retrievability guarantee): given one manifest record and the key, fetch the encrypted payload from any listed `storage` backend, verify `payload.sha256`, decrypt, verify the commit and the OTS proof against Bitcoin, then fold the event log to reconstruct the agent. The bootstrap root is just the commit SHA: it is public (in the OTS proof, and in the on-chain registry if you run one), it resolves the repo from Radicle or Arweave, the repo carries the manifest, and the key unlocks the payload.

## 8. Pluggable backends, and Irys as the swappable future target

Every layer here is a named backend behind a stable record format, so any one can be swapped without touching GASP core or the other layers. This is deliberate, and it is the answer to "hope it is swappable."

**Irys is the documented migration target for the permanence layer.** Irys (evolved from Bundlr, the standard Arweave uploader) is a programmable datachain: genuine on-chain storage with USD-pegged predictable pricing, plus a VM that lets contracts read the stored data directly, which is shaped almost exactly like GASP (it could collapse the permanence layer and the on-chain pointer into one system, and is pitched at verifiable-AI workflows). It is the most GASP-shaped primitive available.

But it is **not yet load-bearing**: as of 2026 it is early (testnet, mainnet ahead, decentralization milestones in progress). Betting an immortality guarantee on a pre-mainnet L1 is self-defeating. So: adopt **Arweave now** as the proven permanence floor, keep the backend interface clean, and **migrate the permanence backend to Irys once it is mainnet-proven**. Because the layer is pluggable, that migration is a backend swap plus a re-upload, not a redesign. Watch Irys; do not depend on it yet.

## 9. Cost

Encrypted ~5 MB payload, per snapshot and per run:

- Arweave payload: ~$0.05 once, permanent.
- Radicle redundancy: free (peer seeding).
- IPFS pin: free to a few cents per month, optional cache.
- OpenTimestamps anchor: $0, at any cadence.

Total: pennies one-time plus $0 per run. Contrast the direct-on-chain payload costs from Section 2 (Solana ~$2,700 locked, Bitcoin ~$3,000–6,000 per snapshot, Ethereum L1 up to six figures). The immortal agent costs cents, because the design puts only a 32-byte hash on the expensive-forever chain and the encrypted payload on the cheap-forever one.

## 10. Conformance additions

A permanence-conformant repo satisfies, on top of GASP core:

1. **Proof validity.** Every `manifest.jsonl` record's OTS proof verifies against its `commit` on Bitcoin, and anchored commits are monotonic in first-parent history order.
2. **Retrievability.** For each record, at least one `storage` locator resolves, and the fetched-then-decrypted payload matches `payload.sha256`.
3. **Restore.** Given a manifest record and the key, a runtime reconstructs the agent (fetch, decrypt, verify, fold) without access to any original host.

## 11. Honest limits

- **No single system is all of git-native, on-chain, permanent, private, and mature.** Radicle is git-native but not permanent (peers must seed). Arweave is permanent but not git and not private. Irys is on-chain-and-programmable but not mature. Bitcoin is permanent-and-mature but cannot hold the payload. Immortality is therefore an *assembly* of layers, not one purchase, and this doc assembles it.
- **Redundancy is not permanence.** Radicle and IPFS survive only while peers seed or pins hold. The permanent floor is Arweave (or later Irys); the P2P layer is availability and speed, not the forever guarantee.
- **Permanence is still a bet.** Arweave's multi-century retention is a claim about one network's economics and longevity, strong but not a law of physics.
- **Key survival is unsolved here.** Encryption relocates immortality to the key. If the key dies, the agent is immortal noise. This is the real remaining hard problem, and it is called out, not solved.
- **Deletion is all-or-nothing.** Crypto-shredding a key is the only "delete." Per-item forgetting is impossible once ciphertext is permanent. Scope keys accordingly.
