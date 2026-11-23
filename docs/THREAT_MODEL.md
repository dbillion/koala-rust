# Threat model

**Status:** engineering baseline, not a penetration-test report or certification assessment.

## System and assets

koala-rust is a Rust process exposing an MCP stdio interface and a CLI. It stores facts, historical
versions, embeddings, entities, relations, tasks, sync metadata, and plugin metadata in a local
SQLite database. It may download an embedding model and plugin artifacts. The user controls the
state directory and host operating system.

Assets to protect:

- Fact text, historical versions, names, metadata, and derived embeddings.
- Plugin binaries, manifests, registry metadata, and execution state.
- Model files, local database backups, and release artifacts.
- Integrity and availability of the database and the process.

## Trust boundaries

1. **MCP client to process:** stdio JSON-RPC input is untrusted. Handlers validate required
   arguments and use parameterized SQL for database values. A compromised client can still issue
   every operation exposed to that process; there is no per-client identity or authorization layer.
2. **Process to SQLite:** the database is local to the configured state directory. OS file
   permissions and host encryption are part of the trusted computing base. Other local processes
   running as the same user can read or modify the database.
3. **Registry/model downloads to local storage:** downloaded content crosses a network trust
   boundary. Plugin artifacts require a pinned SHA-256 digest in the current branch; a checksum
   provides integrity only if the registry/digest source is trusted. Model-download authenticity
   and offline first-run provisioning need separate review.
4. **Plugin runtime:** WASM modules are instantiated by Wasmtime with a deliberately small linker
   and no WASI filesystem/network imports. Native dynamic libraries, where enabled, execute with
   the process's full OS privileges and are fully trusted code.
5. **Backup and release distribution:** backups contain the same sensitive data as the source DB.
   Release checksums are public integrity metadata, not confidentiality controls or signatures.

## Adversaries and assumptions

Consider malformed MCP input, a malicious or compromised plugin registry, a malicious plugin,
other processes running under the same OS account, stolen backup media, and a compromised host.
We assume the host kernel, filesystem, Rust toolchain, and cryptographic libraries are not already
compromised. A local administrator/root attacker is outside the protection offered by this process.

## Existing mitigations

- Rust memory safety for ordinary application code; remaining `unsafe` and native plugin surfaces
  still require review.
- Parameterized SQL for user-provided database values.
- WASM execution without WASI imports by default; no automatic grant of host filesystem or network
  access.
- Plugin artifact SHA-256 verification is mandatory in the current implementation branch.
- SQLite `secure_delete` and incremental vacuum are enabled for the fact-purge path in this branch.
- SQLite backup uses a consistent `VACUUM INTO` snapshot and restore validates `integrity_check`.

These controls are not a claim that all plugin execution, deletion, migration, or backup scenarios
have passed independent security testing. See the linked issues and CI evidence.

## Important gaps

| Area | Current limitation | Required follow-up |
|---|---|---|
| Confidentiality | No database-level encryption or key rotation | #14; host full-disk encryption is the interim control |
| Authorization | No caller identity, clearance, labels, or tenant isolation | #16 |
| Audit | No append-only signed event chain | #15 |
| Erasure | Historical rows and SQLite pages require purge verification; remote replicas/backups need separate lifecycle controls | #11, #12 |
| Retention | No per-fact TTL policy yet | #12 |
| Historical recall | Point-in-time search is not a compliance-grade audit record | #13 |
| Plugins | SHA-256 is not a signature; native code remains privileged; resource limits and signature trust need review | #17, #29 |
| Operations | Fuzzing, full unsafe audit, concurrency stress, and SBOM/signing workflow need completion | #21, #24, #27 |
| Compliance | No independent assessment or certification | Organisation-specific work; not a binary feature |

## Deployment guidance

Use a dedicated OS account and restrictive state-directory permissions. Enable full-disk encryption
on laptops, servers, and removable backup media. Do not expose the state directory to shared users.
Install only trusted plugins; avoid native plugins unless their source and build are reviewed. Store
backups under the same access controls as the primary database. Do not place PHI, MNPI, classified,
or other regulated data in production until the relevant security controls and legal reviews are
complete.
