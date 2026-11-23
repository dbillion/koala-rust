# Tamper-evident audit log

Schema migration v4 adds `audit_log` and a single signed `audit_head`. Each entry
records an actor, action, subject, timestamp, JSON metadata, previous hash and
SHA-256 entry hash. Hash fields are length-delimited before hashing. SQLite
triggers reject ordinary `UPDATE` and `DELETE` operations on the log; verification
also recomputes the full chain so dropping a trigger does not hide row tampering.

## Signing and trust

Set `KOALA_AUDIT_SIGNING_KEY` to a base64-encoded 32-byte Ed25519 seed held by an
external secret manager. The seed is never written to SQLite. Optionally set
`KOALA_AUDIT_KEY_ID` to identify the signer and pin the corresponding base64
public key in `KOALA_AUDIT_TRUSTED_PUBLIC_KEY` on verification hosts.

- `KOALA_AUDIT_REQUIRE_SIGNATURE=1` rejects audited writes without a signing key
  and makes verification fail when the head is unsigned.
- `KOALA_AUDIT_REQUIRE_TRUSTED_KEY=1` requires the database's public key to match
  `KOALA_AUDIT_TRUSTED_PUBLIC_KEY`; this is the trust-anchor check.
- `KOALA_AUDIT_ACTOR` sets the actor recorded in new entries (default `local`).

Without a signing key, the hash chain is still recorded but the head is unsigned.
Without a pinned public key, a valid signature proves consistency with the public
key stored in the same database, not the identity of the signer. For production,
configure both the signing key and an independently pinned public key.

## Commands

- `koala-rust audit verify` checks the complete chain and head signature.
- `koala-rust audit export` emits JSON containing all records and the verification
  metadata, including the public key and signed head for external verification.

## Coverage boundary

The current implementation audits fact writes, deletes, prefix deletes and TTL
expiry; entity creation/observation additions; task creation/updates; relation
creation; metadata changes; model snapshots; feedback; thought/conclusion writes
and thought clearing; and schema migration. The in-memory sync manager does not
persist transport state in this database. Plugin administration and any future
mutation path must be explicitly added to the audit boundary before being treated
as covered. The explicit raw-SQL interface and direct database-file edits can
bypass application audit events; derived vector-index maintenance is not logged
as a separate business event. Restrict raw-SQL access and filesystem access
accordingly. This is not WORM storage, a legal hold system, or a compliance
certification. Keep independent backups and preserve the signing key and pinned
public key outside the database host.
