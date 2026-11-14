# Continuous integration

CI tests the library and repository-owned synthetic fixtures on Linux, Windows,
and macOS stable Rust, and Linux Rust 1.91.0 (the declared MSRV). Documentation
tests and the existing reader example are checked separately. No manufacturer
profile is downloaded, bundled, or silently skipped because it is absent.

External fixture tests are explicitly ignored with a reason. After obtaining
the named samples under their individual terms, run a selected test with
`cargo test --test icc_profile_load srgb_v4 -- --ignored`, for example. Missing
samples in an explicitly selected test remain errors. The official mAB test
also requires `ICC_V4_MAB_TEST_PROFILE`. Do not use `--include-ignored` in the
normal CI job: benchmarks and externally licensed data are opt-in.

The independently locked `tools/cms-compare` harness is a manual black-box
comparison tool, not a dependency of the library or normal PR tests. See
manufacturer-cms-comparison.md for versions, licensing, and reproduction.

A separate Linux stable job enforces `cargo fmt --all -- --check` and
`cargo clippy --all-targets -- -D warnings` for the library, reader example,
and standalone CLI. Legacy public constructor and inherent string-method
signatures have narrow, documented lint exceptions to preserve compatibility.
Manual comparison harnesses remain outside the normal CI gate.
No CI workflow grants write permissions or publishes packages. The standalone CLI package is tested with its committed lockfile in every matrix job. Cargo.lock is intentionally not tracked
for this library; CI exercises compatible dependency resolution on each toolchain.
