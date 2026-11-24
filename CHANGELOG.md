# Changelog

All notable changes to Koala are documented here.

## Unreleased

### Build and release
- Upgrade SQLx and SQLite bindings to patched releases; raise MSRV to Rust 1.94.
- Upgrade bundled SQLCipher to 4.20.0 with the patched SQLite amalgamation.
- Remove unused legacy dependencies that pulled vulnerable crypto crates into the lockfile.
- Bind dynamic SQL values with QueryBuilder, restrict vector filters to literal keywords, and document the trusted sqlite-vec loading boundary.
- Bound SQL fuzz inputs and enforce network isolation in offline integration tests.
- Correct release verification to expect all 28 generated assets.

## v0.3.4 - 2026-11-22

### Fixed
- Bundle the matching sqlite-vec loadable library with each native release executable.
- Prefer the library beside the executable, retaining the Linux system-path fallback for manual installations.
- Replace the non-failing release `--help` check with a smoke test that executes `vec_version()` from the bundle.

### Security
- Upgrade Wasmtime to 36.0.23, which includes fixes for the reported Cranelift sandbox-escape vulnerabilities.
- Remove the unused `wasmtime-wasi` dependency.
- Raise the declared minimum supported Rust version to 1.91.

### Build and release
- Track `Cargo.lock` and use locked dependency resolution in CI and release builds.
- Serialize publishing per release tag.
- Upload releases as drafts and publish only after the complete asset set and SHA-256 checksums have been verified.

### Documentation and tests
- Document self-contained release archives and the manual extension override for older releases.
- Correct the E2E test's default executable path and update the embedding-dimension references to 256.
