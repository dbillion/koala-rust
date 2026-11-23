# Security policy

## Reporting a vulnerability

Please report privately. Do not open a public issue for a security problem.

1. Open a [private security advisory](https://github.com/dbillion/koala-rust/security/advisories/new).
2. Include the version (`koala-rust --version`), your OS, steps to reproduce and what you expected.
3. Do not include real personal, patient or financial data. Use made-up samples.

This is a small open-source project run by volunteers. We read every report and will answer as
soon as we can, but we cannot promise a fixed response time.

## Supported versions

Fixes land in the latest release. Please test against it before reporting.

## What to know about the current state

koala-rust is pre-1.0. It is good for personal and team agent memory and for pilots, and it is being
hardened step by step. Known gaps are public and tracked as issues, for example:

- verified deletion of every version of a fact ([#11](https://github.com/dbillion/koala-rust/issues/11))
- encryption at rest ([#14](https://github.com/dbillion/koala-rust/issues/14))
- tamper-evident audit log ([#15](https://github.com/dbillion/koala-rust/issues/15))
- access control ([#16](https://github.com/dbillion/koala-rust/issues/16))
- plugin trust and sandbox limits ([#17](https://github.com/dbillion/koala-rust/issues/17))

Please do not store secrets or regulated data (health, financial or classified) in it yet.
Reports about these known gaps are welcome as comments on the issues. Public discussion is fine,
because they are already public. For a new problem, use the private advisory.

## Dependencies

Dependabot checks Cargo and GitHub Actions dependencies weekly. Patch updates merge automatically
after CI passes. Security updates are not delayed.
