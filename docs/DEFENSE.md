# Defense and high-assurance deployment notes

**Not approved for classified information, CUI, export-controlled data, or operational deployment.**
No FIPS, CMMC, FedRAMP, Common Criteria, or hardware accreditation claim is made.

## Cryptography and key custody

The project uses general-purpose Rust cryptographic libraries for hashes and integrity checks. It
has not been validated as a FIPS 140-2/140-3 cryptographic module. A SHA-256 checksum does not
establish FIPS validation. There is no TPM 2.0, PKCS#11, or HSM-backed key provider, no documented
key ceremony, and no external signing-key lifecycle. Do not market or deploy it as FIPS validated.

## Offline and supply-chain posture

A no-network build is available with --no-default-features; it excludes reqwest, ureq, and
Hugging Face Hub clients. Provision a local model with the network-enabled model fetch --to command,
set KOALA_MODEL_DIR, and use --offline at runtime. Bundled plugins remain available while remote
plugin installs are blocked. The release workflow generates a CycloneDX SBOM and keyless Sigstore
signature for the SHA-256 checksum manifest, and the remember/recall path has passed inside an
isolated unshare -Urn network namespace. SBOM coverage of bundled third-party runtime binaries and
reproducible builds still need independent verification. This does not provide FIPS validation or
hardware-backed key custody.

## Hardware and accreditation

TPM/HSM integration, certified hardware, CMMC evidence, facility controls, personnel controls,
accreditation, and prime/integrator partnerships are outside what this repository can establish by
itself. They require a system-level programme and independent assessment.

## Minimum gate before any pilot

- Written classification/CUI determination and approval from the system owner.
- FIPS-validated crypto module where required, with validated configuration evidence.
- External key custody, rotation, recovery, and incident procedures.
- Independently reviewed SBOM coverage (including bundled runtime binaries), signed release artifacts,
  vulnerability response, and reproducible build evidence.
- Identity, least privilege, audit, retention, secure deletion, and immutable backup controls.
- Independent penetration test and formal authorization/accreditation process.

Related engineering work: #17 plugin trust, #18 offline build, #21 quality, #24 threat model/SBOM,
#27 FIPS/TPM/HSM research.
