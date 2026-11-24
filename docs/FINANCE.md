# Finance deployment guide

**Not a broker-dealer recordkeeping system, not SEC 17a-4/WORM compliant, and not legal advice.**
Do not store MNPI or regulated books-and-records data until compliance counsel and security owners
approve the architecture and controls.

## Information barriers

The current process has no authenticated caller identity, desk/team clearance, label-based access
policy, or tenant namespace. A single state directory must not be treated as an information barrier.
Do not rely on separate fact-name prefixes as access control. A deployment that needs Chinese walls
requires a policy gate covering read, list, recall, vector search, sync, export, and plugin-mediated
access (#16).

## Point-in-time recall and audit

Versioned facts can show application-level historical values, but this is not proof of what a user
saw or what an agent retrieved. Point-in-time recall (#13) is a query feature, not a signed audit
record. A tamper-evident log (#15) must record actor, action, timestamp, object, outcome, and a
verifiable signature with keys protected separately from the database.

A hash chain alone is not SEC Rule 17a-4 compliance and is not WORM storage: an administrator who
controls both the database and signing key may be able to rewrite history. Pair application logs
with independently controlled immutable storage that has been assessed for the applicable rule.

## Recordkeeping and deployment checklist

- Identify applicable SEC/FINRA/CFTC rules and retention schedules with counsel.
- Separate desks and clients at the identity and storage-policy layers; do not rely on naming
  conventions.
- Encrypt hosts, database backups, exports, and removable media.
- Define legal hold, deletion, retention, export, supervision, and audit review processes.
- Keep non-public information out of unapproved pilots; use synthetic or public sample data.
- Validate backup restore, incident response, access revocation, and disaster recovery.

Related engineering work: #13 point-in-time recall, #14 encryption, #15 audit, #16 information
barriers, #20 migrations/backup, #24 threat model and claims.
