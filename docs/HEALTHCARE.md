# Healthcare deployment guide

**Not HIPAA certified. Not legal advice. Do not use for production PHI until your organisation's
privacy, security, legal, and compliance owners approve the complete deployment.**

## HIPAA Security Rule discussion

| Safeguard area | koala-rust today | What the covered entity/business associate still needs |
|---|---|---|
| Risk analysis and management | No organisation-wide risk analysis | Document threats, assets, likelihood/impact, mitigations, and periodic review |
| Workforce security / training | No workforce roles or training | Assign authorised operators, train them, revoke access promptly |
| Access control (164.312(a)) | No caller identity, role, or minimum-necessary policy | Enforce unique identities, least privilege, and tenant/namespace boundaries before PHI use (#16) |
| Audit controls (164.312(b)) | No append-only signed audit record | Record access, changes, exports, admin actions; protect and review logs (#15) |
| Integrity (164.312(c)) | SQLite transactions and checksums exist, but no signed audit chain | Define integrity controls and evidence; a checksum alone is not an audit trail |
| Person/entity authentication (164.312(d)) | No application authentication | Authenticate users and services at the deployment boundary |
| Transmission security (164.312(e)) | Local stdio; sync transport is not a production secure transport | Protect any remote transport and peer links with authenticated encryption |
| Contingency / backup | SQLite backup/restore is available in this branch | Encrypt backups, test restores, define RTO/RPO and retention |
| Device/media controls | Relies on host OS | Full-disk encryption, inventory, secure disposal, access-controlled backups |
| Deletion / amendment | Application fact purge and vector deletion are implemented; replicas/backups have separate lifecycles | Define retention, deletion propagation, legal holds, backup expiry (#11, #12) |

## BAA and hosting question

Whether a business associate agreement is required depends on the actual service relationship,
including who hosts the system, who can access ePHI, and whether the vendor creates, receives,
maintains, or transmits it on behalf of a covered entity. Self-hosting does not by itself establish
that every deployment is outside BAA requirements. Have counsel assess the facts and contracts.

## Deployment checklist

- Use a dedicated host/account and encrypted storage; restrict state and backup directory access.
- Use synthetic or de-identified data until access control, audit, deletion, and retention are
  independently verified.
- Keep model and plugin downloads controlled; pin plugin digests and review plugin code.
- Test backup restore, deletion, retention, incident response, and access revocation.
- Complete a risk analysis and vendor/security review; do not represent the binary as HIPAA compliant.

Related engineering work: #11 deletion, #12 retention, #14 encryption, #15 audit, #16 access control,
#24 threat model and claims.
