# Contributing

Thanks for working on `audiowaveform`.

The project is a Rust workspace centered on the `audiowaveform` library crate,
with `audiowaveform-cli` as a thin adapter. Reusable logic belongs in the
library. CLI-only argument parsing, user-facing logging, and exit handling
belong in the binary crate.

## Before You Start

- Discuss substantial feature work before implementing it.
- Work from an up-to-date branch.
- Keep changes focused; avoid mixing refactors, behavior changes, and fixture updates unless they are directly related.

## Development Expectations

- Follow the existing Rust style and module boundaries.
- Keep the public API narrow and ergonomic.
- Every public struct, enum, function, and public method must remain documented.
- Update rustdoc examples and README examples when public behavior changes.
- Add or update tests for behavior changes. Prefer unit tests for internal logic and integration tests for end-to-end workflows.

## Local Checks

Run these before sending a change for review:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

If you touch feature-gated code in the library, also run the feature matrix used in CI:

```sh
cargo test -p audiowaveform --no-default-features
cargo test -p audiowaveform --no-default-features --features decode
cargo test -p audiowaveform --no-default-features --features render
cargo test -p audiowaveform --no-default-features --features wav
```

## Fixtures and Goldens

- Shared fixtures and goldens live in `fixtures/`.
- If you intentionally change rendered output or decoder behavior, update the relevant golden files and explain why in the change.

## Releases

Release preparation should be a dedicated change. For a release:

- Update `[workspace.package].version` in `Cargo.toml`.
- Update `CHANGELOG.md`.
- Regenerate `Cargo.lock` if dependency resolution changes.
- Tag the release as `X.Y.Z`.
