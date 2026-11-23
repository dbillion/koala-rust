# Product claims and evidence

This table distinguishes implemented behavior from aspirations. A claim is considered verified only
when its named test or workflow is present and passing on the relevant commit.

| Claim | Evidence | Status / boundary |
|---|---|---|
| Local SQLite fact storage | `tests/e2e.py` fact write/read and raw SQL checks | Implemented; filesystem and OS account are trusted |
| Versioned facts | `read_fact_versions` and E2E update test | Implemented; not a tamper-proof history |
| Semantic recall | E2E semantic recall and vector tests | Implemented when the embedding model is available |
| Point-in-time recall | `facts_as_of` and `recall_as_of` regression tests | Implemented in this branch; not a signed audit record |
| Permanent fact purge | Purge unit/E2E tests, database-file plaintext check, vector-row check | Implemented in this branch; backups and remote copies have separate retention |
| SQLite migration/backup | Migration version tests, backup snapshot test, restore integrity check | Implemented in this branch; restore is destructive to the active DB after a validated backup |
| Built-in hash plugin | Plugin install/discovery/hash regression test | Implemented in this branch for built-in hash operations |
| Plugin artifact integrity and authenticity | Mandatory SHA-256 plus Ed25519 verification for remote WASM installs; build-time hashes for builtins | Implemented in this branch when trusted publisher keys are configured; third-party WASM tool execution is not implemented |
| WASM sandbox | Wasmtime fuel, memory/table limits, no WASI imports in linker; native plugins are opt-in | Limited sandbox; third-party WASM tool execution is not implemented, and enabling native plugins removes the WASM isolation boundary |
| Offline operation | E2E exercises a shared Hugging Face cache; it does not deny network access | Not a true air-gap build: model and remote plugin fetching still require network; no network-free Cargo feature is implemented |
| Encryption at rest | Opt-in `sqlcipher` feature and wrong-key/plaintext integration test | Experimental only: key via `KOALA_DB_KEY`; full app/vector, backup/restore, rekey, sync-blob and external key-provider coverage remain incomplete. Use host full-disk encryption as interim protection |
| Access control / tenant isolation | No authenticated caller identity or policy gate | Not implemented |
| Tamper-evident audit / WORM | No signed append-only audit chain or WORM storage integration | Not implemented; do not claim SEC 17a-4 compliance |
| FIPS / HIPAA / SOC 2 / FedRAMP / CMMC | No validation or certification evidence | Not certified; no compliance claim |

The tests referenced above must be run on the commit under review. A source-level implementation or
passing unit test does not replace cross-platform CI, release smoke tests, independent security
review, or organisational compliance work.
