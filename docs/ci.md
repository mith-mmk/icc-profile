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

Formatting and strict Clippy are not merge gates yet: existing legacy files
have unrelated formatting/lint debt. Adding them requires a separate cleanup.
No CI workflow grants write permissions or publishes packages. CLI checks can
be added when the CLI package exists. Cargo.lock is intentionally not tracked
for this library; CI exercises compatible dependency resolution on each toolchain.
