# Baseline spikes and known limitations

These findings were observed against the unmodified v0.3.3 application while building the Phase 0 contract snapshots. They are recorded rather than silently changing behavior in the safety-net phase.

## CLI model state is process-local

- `model-init` exits successfully, but each CLI invocation starts a new `Koala` instance.
- A following `model-train` or `model-predict` invocation exits with `no model loaded (model_init or model_load first)`; `model-save` exits with `no model loaded`.
- This means the documented multi-command CLI flow cannot share an initialized in-memory model. A later model/persistence phase should either persist initialization state or provide a single-session/combined command flow.
- The baseline snapshots preserve these results until the behavior is deliberately corrected.

## Built-in WASM plugin metadata (fixed in the current issue branch)

- The registry contains the `hash-tools` manifest and tool definition.
- Generic startup discovery originally loaded installed WASM files with `None` metadata, leaving an empty manifest.
- The current issue branch reattaches registry metadata for installed builtins on startup, validates the generated WASM bytes against a build-time SHA-256 digest, and removes temporary debug output.
- A regression test installs the builtin, exercises generic discovery, restores its manifest, and verifies SHA-256(`hello`). The WASM module is a metadata carrier; builtin hash computation is implemented in the host runtime. Arbitrary external WASM tool execution is still not implemented.

## Registry update is not a deterministic snapshot operation

- `plugin-update-registry` contacts the public plugin registry and can change as remote data changes.
- Its CLI help is snapshotted; the live registry mutation is intentionally excluded from the fixed-database output snapshot.

## Snapshot baseline observations

- The current MCP `tools/list` response contains 60 tool definitions (the original plan estimated 53); all names and input schemas are captured.
- The CLI snapshot captures deterministic command results in JSON plus representative text-format outputs, and all CLI/context help entries.
- `embed-text` and `embed-batch` are not executed in snapshots because they require a separately provisioned pretrained Model2Vec model. Their CLI help is captured; embedding behavior is exercised by the E2E/model tests.
- `plugin-update-registry` is not executed because it mutates against live external registry data. Its CLI help is captured.
- The snapshot and E2E harnesses set `KOALA_STATE_DIR` to temporary directories; they do not use or modify the user's real `~/.koala` state.
