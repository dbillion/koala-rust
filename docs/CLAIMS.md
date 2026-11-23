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
| Plugin artifact integrity | Mandatory SHA-256 match at install and on already-installed files | Implemented in this branch; SHA-256 is not a publisher signature |
| WASM sandbox | Wasmtime instantiation with no WASI imports in linker | Limited sandbox; resource limits/signatures/native plugin policy still require work |
| Offline operation | E2E with cached model under network isolation | Verified only after model cache is populated; first run still needs model download |
| Encryption at rest | No application-level encryption | Not implemented; use host full-disk encryption as interim protection |
| Access control / tenant isolation | No authenticated caller identity or policy gate | Not implemented |
| Tamper-evident audit / WORM | No signed append-only audit chain or WORM storage integration | Not implemented; do not claim SEC 17a-4 compliance |
| FIPS / HIPAA / SOC 2 / FedRAMP / CMMC | No validation or certification evidence | Not certified; no compliance claim |

The tests referenced above must be run on the commit under review. A source-level implementation or
passing unit test does not replace cross-platform CI, release smoke tests, independent security
review, or organisational compliance work.
