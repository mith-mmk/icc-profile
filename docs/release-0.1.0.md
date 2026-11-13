# icc-profile 0.1.0 release checklist

Status: preparation only, not published or merged. Source candidate:
`3087ec3e370c8f82bc43e31aedf08e2f81d01c3d`, including inverse-TRC performance
validation and the controlled ColorSync intent investigation. The release branch
adds metadata/documentation and synchronizes standalone-tool lockfiles only.

## Scope

See [CHANGELOG](../CHANGELOG.md), [API migration](api-migration.md), and
[README](../README.md) for supported routes and compatibility. Unsupported routes
fail explicitly; there is no profile-free fallback or universal CMS accuracy
claim. Existing legacy format/strict-Clippy debt is not silently waived as passing.
Historical [0.0.6 checklist](release-checklist.md) and [0.0.7 notes](release-0.0.7.md)
are preserved and do not describe this release.

## Gates

- [x] Set library version to 0.1.0 and synchronize tracked tool lockfiles.
- [x] Update README, changelog and minimum-Rust contribution guidance.
- [ ] macOS stable: all-target tests, doc tests, reader example, locked CLI tests.
- [ ] macOS Rust 1.91.0: all-target and doc tests.
- [ ] Clean-commit `cargo package --locked` and `cargo publish --dry-run --locked`.
- [ ] Compare full-tree formatting/strict Clippy with the source candidate.
- [ ] Verify pushed branch SHA and cross-platform CI for that exact SHA.
- [ ] Owner-authorized merge/publication, registry version/checksum verification.
- [ ] Only after ICC publication: update and verify downstream WML2.

## Reproduction

```sh
cargo test --all-targets
cargo test --doc
cargo check --manifest-path examples/Cargo.toml
cargo test --locked --manifest-path tools/icc-cli/Cargo.toml
cargo +1.91.0 test --all-targets
cargo +1.91.0 test --doc
cargo package --locked
cargo publish --dry-run --locked
cargo fmt --check
cargo clippy --offline -- -D warnings
```

CI (`.github/workflows/ci.yml`) covers Linux/Windows/macOS stable and Linux
1.91.0, including the reader example and locked CLI. Work-branch pushes alone
do not trigger CI; use existing `workflow_dispatch` on the pushed release
branch (or an authorized pull request). Require the run's head SHA to match
the final pushed SHA. Dry-run never uploads the crate. External profiles and
opt-in comparisons are not rerun by these normal gates; refer to the existing
[TRC](inverse-trc-validation-20261114.md) and
[ColorSync](colorsync-intents-validation-20261114.md) evidence for their scope.
