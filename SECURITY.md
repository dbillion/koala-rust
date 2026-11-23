# Security policy

## Supported security posture

koala-rust is a local-first memory server, not a certified security boundary. The current release
is suitable for development, evaluation, and appropriately controlled non-sensitive workloads.
Do not use it as the sole control for regulated, classified, or otherwise high-impact data.

The database and plugin files are protected by the operating-system account and filesystem
permissions. The application does not currently provide database encryption, user authentication,
role-based access control, tenant isolation, or a tamper-evident audit trail. Native plugins execute
in the host process and must be treated as trusted code. WASM plugins are instantiated without WASI
capabilities by default; this is not a substitute for a complete plugin security review.

## Reporting a vulnerability

Please do not open a public issue containing exploit details, credentials, personal data, or a
reproducible attack against a live system. Use GitHub's **Report a vulnerability** action on the
repository Security tab to submit a private advisory. Include the affected version, platform,
impact, and a minimal reproduction. We will coordinate a fix and disclosure through the advisory.

## Release verification

Release archives publish `SHA256SUMS.txt`. Verify it before installing:

```sh
sha256sum -c SHA256SUMS.txt
cosign verify-blob --bundle SHA256SUMS.txt.sigstore.json \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com \
  --certificate-identity-regexp '^https://github.com/dbillion/koala-rust/.github/workflows/release.yml@refs/tags/v[0-9.]+$' \
  SHA256SUMS.txt
```

The keyless Sigstore bundle authenticates the checksum manifest to the GitHub Actions release
workflow identity. The CycloneDX SBOM is published as `koala-rust.cdx.json`. These supply-chain
artifacts do not replace source review, reproducible builds, or a security assessment.

## Known gaps

- No built-in encryption at rest or key-management integration.
- No authenticated identities, access labels, or tenant namespaces.
- No append-only signed audit log or WORM retention guarantee.
- No FIPS 140-validated cryptographic module claim.
- No TPM 2.0, PKCS#11, or HSM key provider.
- No CMMC, FedRAMP, HIPAA, SOC 2, or other certification claim.

See [Threat Model](docs/THREAT_MODEL.md), [Claims](docs/CLAIMS.md), and the sector-specific guides.
