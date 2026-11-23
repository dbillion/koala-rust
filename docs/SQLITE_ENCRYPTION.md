# SQLite encryption at rest: current position

## Finding

The current build uses the standard SQLite library linked through `sqlx` and the `sqlite-vec`
extension. It does not build or link SQLCipher, does not accept a database encryption key, and does
not provide `db encrypt` or `db rekey`. SQLCipher compatibility with the current `sqlx`/SQLite
binding and `sqlite-vec` extension has not yet been established. Do not assume the database is
encrypted because the host filesystem or transport is encrypted.

## Interim deployment control

The only currently documented at-rest protection is host-managed full-disk or volume encryption
(e.g. FileVault, BitLocker, or LUKS) combined with restrictive OS permissions on the state
 directory and backups. This protects powered-off media; it does not protect an unlocked/running
host, a process with filesystem access, or plaintext exported to another location.

## Decision and follow-up

- SQLCipher remains an unverified integration spike; no compatibility claim is made.
- Do not store regulated data based on this fallback alone.
- A future SQLCipher implementation must test fresh and migrated databases, wrong-key behavior,
  rekey, backup/restore, vector virtual-table compatibility, and absence of plaintext facts in the
  database file.
- Key material must come from a protected file, OS keyring, or external key provider, never a
  command-line argument or committed configuration.
