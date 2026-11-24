# SQLite encryption at rest: current position

## Experimental SQLCipher support

This branch includes an opt-in `sqlcipher` Cargo feature backed by the bundled
SQLCipher build of `libsqlite3-sys`. When built with `--features sqlcipher`, the
application requires a non-empty `KOALA_DB_KEY` environment variable and applies
the key before other SQLite pragmas. The key is not accepted on the command line.

The current integration test exercises SQLCipher through `sqlx`: it checks that
stored fact-like text is absent from the database file, that a wrong key cannot
read the table, and that the correct key can. This is an experimental integration
spike, not a production-ready encryption feature. The test does not yet establish
compatibility of the complete application with `sqlite-vec`, migrations, online
backup/restore, sync blobs, or key rotation.

There are no `db encrypt` or `db rekey` commands. Existing plaintext databases
are not transparently encrypted by enabling the feature; users must not point an
encrypted build at a plaintext database and assume it has been converted. The
key currently comes only from the environment; protected key-file, OS keyring,
and external key-provider integrations are not implemented.

## Interim deployment control

Until the full application path and lifecycle tests are complete, the supported
at-rest deployment control remains host-managed full-disk or volume encryption
(e.g. FileVault, BitLocker, or LUKS), combined with restrictive OS permissions
on the state directory and backups. This protects powered-off media; it does not
protect an unlocked/running host, a process with filesystem access, or plaintext
exported to another location.

## Required before production use

- Test the application `Database` path with SQLCipher and `sqlite-vec` together.
- Test fresh and migrated databases, wrong-key failure, backup/restore, and rekey.
- Ensure sync payloads, temporary files, and backups do not expose plaintext.
- Add a protected key-file, OS keyring, or external key provider; document rotation
  and recovery without putting key material in logs, arguments, or source control.
- Verify deletion and secure-vacuum behavior on encrypted databases.

Do not treat the experimental feature or host-level encryption as a compliance
certification. See the threat model and healthcare/finance guides for limitations.
