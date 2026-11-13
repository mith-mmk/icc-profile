# Formatting and strict Clippy cleanup

Base: `a2e6801cd34b882573b8f0789e8dc6a2ad93a2d6` (0.1.0 preparation).
This is an independent cleanup branch, not a merge or publication.

## Changes and compatibility

- Apply rustfmt to the library, tests, examples and standalone CLI.
- Remove redundant borrows/field names/casts and unnecessary formatting or
  allocations; use equivalent standard-library helpers.
- Rewrite flagged loops without changing sample order or arithmetic order.
- Preserve the existing public `&Vec<u8>` constructor signatures and inherent
  string methods with narrow documented exceptions; compile-time signature
  checks and exact string assertions cover the compatibility decision.
- Add `MClut::is_empty` and tests for both CLUT storage precisions.
- Clean test initializers and macro array borrows. The shortened f32 literal
  represents the same f32 value, not a precision change in production math.
- Add a Linux stable CI quality job for root, reader and CLI formatting and
  strict all-target Clippy. Existing cross-platform/MSRV tests remain intact.

## Local checks (macOS arm64)

- Original library strict Clippy: 97 errors (100 in the lib-test build).
- Final stable Rust 1.97.0 strict all-target Clippy: passed, including reader
  and locked CLI packages. All three format checks passed.
- Stable and MSRV Rust 1.91.0: each 364 all-target tests passed, 62 explicitly
  ignored; each 3 doc tests passed, 8 ignored. No failed tests.
- Locked standalone CLI: 8 tests passed.
- MSRV Clippy was not run because that toolchain has no Clippy component;
  the enforced Clippy gate uses stable. MSRV compilation/tests passed.

Ignored external fixtures and opt-in comparisons were not represented as
coverage. Package verification and the exact pushed-SHA Actions outcome must
be checked independently; no command in this cleanup publishes the crate.
