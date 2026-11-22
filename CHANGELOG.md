# Changelog

All notable changes to Koala are documented here.

## Unreleased

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
