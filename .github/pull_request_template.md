## What and why

<!-- One or two sentences. Link the issue: Closes #123 -->

## Checks

- [ ] `cargo fmt --check --all` and `cargo clippy --locked --all-targets --all-features -- -D warnings` pass
- [ ] Tests added or updated for behavior changes
- [ ] `scripts/check_loc.sh` passes (no new file over 99 lines, nothing added to the allowlist)
- [ ] Docs or README updated if behavior changed
- [ ] No secrets and no real personal, patient or financial data

## Notes for the reviewer

<!-- Anything tricky, trade-offs, or follow-ups -->
