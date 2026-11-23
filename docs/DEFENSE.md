# Defense and high-assurance deployment notes

**Not approved for classified information, CUI, export-controlled data, or operational deployment.**
No FIPS, CMMC, FedRAMP, Common Criteria, or hardware accreditation claim is made.

## Cryptography and key custody

The project uses general-purpose Rust cryptographic libraries for hashes and integrity checks. It
has not been validated as a FIPS 140-2/140-3 cryptographic module. A SHA-256 checksum does not
establish FIPS validation. There is no TPM 2.0, PKCS#11, or HSM-backed key provider, no documented
key ceremony, and no external signing-key lifecycle. Do not market or deploy it as FIPS validated.

## Offline and supply-chain posture

A cached embedding model can be used without network access, but first-run model acquisition and
registry updates still use the network. A true air-gap build, reproducible supply chain, complete
SBOM, signed artifacts, and offline model/plugin provisioning require further work. Release checksums
are integrity metadata, not a signature or attestation.

## Hardware and accreditation

TPM/HSM integration, certified hardware, CMMC evidence, facility controls, personnel controls,
accreditation, and prime/integrator partnerships are outside what this repository can establish by
itself. They require a system-level programme and independent assessment.

## Minimum gate before any pilot

- Written classification/CUI determination and approval from the system owner.
- FIPS-validated crypto module where required, with validated configuration evidence.
- External key custody, rotation, recovery, and incident procedures.
- Complete SBOM, signed release artifacts, vulnerability response, and reproducible build evidence.
- Identity, least privilege, audit, retention, secure deletion, and immutable backup controls.
- Independent penetration test and formal authorization/accreditation process.

Related engineering work: #17 plugin trust, #18 offline build, #21 quality, #24 threat model/SBOM,
#27 FIPS/TPM/HSM research.
