# Contributing to koala-rust

Thanks for helping. People of every experience level are welcome here.

## Find something to work on

- [`good first issue`](https://github.com/dbillion/koala-rust/labels/good%20first%20issue): small, well-scoped tasks.
- [`help wanted`](https://github.com/dbillion/koala-rust/labels/help%20wanted): bigger pieces where we want a hand.
- Not sure? Comment on an issue and say what you would like to try. We will help you scope it.
- Have a new idea? Open an issue first so we can agree on the shape before you write code.

## Set up in five minutes

You need Rust 1.94 or newer.

```sh
git clone https://github.com/dbillion/koala-rust
cd koala-rust
cargo build
cargo test
```

Try your build without touching your real data:

```sh
export KOALA_STATE_DIR=$(mktemp -d)
sh examples/cli_demo.sh ./target/debug/koala-rust
python3 examples/demo.py ./target/debug/koala-rust
```

## Before you open a pull request

CI runs these, so run them locally first:

```sh
cargo fmt --check --all
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo check --locked --all-targets
scripts/check_loc.sh
```

- **99-line rule.** Rust files should stay under 100 lines. `scripts/check_loc.sh` enforces it, and
  `scripts/loc_allowlist.txt` lists the files still waiting to be split. Never add a file to the list.
  Taking one off the list by splitting it is a great contribution.
- **Tests.** Add or update tests for behavior you change. CI also runs `tests/snapshot_baseline.py`,
  which pins the CLI, MCP and SQLite behavior. If a snapshot change is intended, explain why in the PR.
- **Commit messages.** Short and in the style `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
- **No secrets, no real personal or patient data** in code, tests, issues or screenshots.
- **One change per pull request.** Smaller pull requests get reviewed faster.

## Pull request flow

1. Fork, then create a branch from `master`.
2. Make the change, run the checks above.
3. Open a pull request and fill in the template. Link the issue it closes.
4. A maintainer reviews. Dependabot patch updates merge on their own after CI passes.

## Reporting security problems

Please do not open a public issue. See [SECURITY.md](SECURITY.md).

## Conduct

Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

By contributing you agree that your work is released under the project's [MIT license](LICENSE).
