# ICC CMS 0.0.5 implementation and release plan

This document is the durable scope and release checklist for the `icc-profile`
0.0.5 Gray/RGB CMS work. It records implementation boundaries and acceptance gates, not
individual work logs. The normative reference is [ICC.1:2022](https://www.color.org/specifications/ICC.1-2022-05.pdf).

## Current checkpoint

The following work is complete and must remain API-compatible:

- Public profile/transform API foundation: `Profile`, `ParseLimits`,
  `ColorSpace`, `Pcs`, `RenderingIntent`, `TransformOptions`, immutable
  `Transform`, and `TransformWorker`.
- Checked header/tag bounds, declared-profile-length bounds, profile/tag/curve
  count limits, and explicit `UnsupportedProfileFeature` errors. Full checked
  allocation coverage remains pending.
- Gray and RGB matrix/TRC transforms, including D50 PCS handling,
  chromatic-adaptation validation, `curv`, and parametric curves 0--4.
- Transform module split, with compiled direction and inverse matrix data
  retained rather than recomputed per pixel. Scratch storage is owned by
  `TransformWorker`.
- `Profile` structural-parse-only separation and independent Curve forward /
  inverse compilation are not complete yet. They remain explicit follow-up
  work because the current profile path still performs matrix compilation and
  the current curve parser applies inverse-oriented monotonicity checks.
- Canonical Sharma CIEDE2000 and CIE76 implementations behind the existing
  `utils::delta_e76` and `utils::ciede2000` signatures.

The crate version is `0.0.5` on the release-preparation branch. Publication,
tagging, and merge to the release branch remain separate approval-gated steps.

The broader 0.1.0 checklist below is retained as historical follow-up
context. It is not a gate for this narrowed Gray/RGB 0.0.5 release.

## Historical 0.1.0 follow-up scope

### Parsing and compilation

- [ ] Keep `Profile` as a checked structural parser only; it must not select a
  rendering intent or silently construct a fallback transform.
- [ ] Use one checked reader for all integer reads, offsets, lengths,
  allocations, tag tables, and declared profile-length boundaries.
- [ ] Replace remaining unbounded or indirectly bounded vector allocations
  with checked reservations and make allocation failure/limit errors explicit.
- [ ] Preserve immutable, `Send + Sync` compiled transforms and per-worker
  scratch reuse for all new paths.
- [ ] Return `UnsupportedProfileFeature` for a valid but unsupported feature;
  malformed data and limit violations remain parse/limit errors. Never use a
  simplified RGB or CMYK fallback for either case.
- [ ] Complete legacy parser safety and malformed/truncated/oversized input
  handling, including overflow and allocation-failure paths.

### Curves, LUTs, and multiprocess elements

- [ ] Keep curve forward parsing independent from inverse compilation. Forward
  LUT curves may be constant or non-monotonic; only an inverse consumer may
  require a compilable monotonic mapping. The current parser still conflates
  these concerns and must be split before LUT completion.
- [ ] Implement `mft1` and `mft2` with the specified processing order:
  matrix, input tables, full N-dimensional CLUT, then output tables. N-D CLUT
  interpolation must cover all `2^N` corners and exact endpoints.
- [ ] If an `mft1` XYZ definition is ambiguous for the requested transform,
  reject it explicitly as unsupported rather than guessing.
- [ ] Implement `mAB` in A -> CLUT -> M -> matrix -> clipped B order, and
  `mBA` in B -> matrix -> clipped M -> CLUT -> A order.
- [ ] Implement optional A/M/B stage combinations and validate channel counts,
  matrix dimensions, table sizes, and stage ordering.
- [ ] Implement DToB/BToD and MPE float paths. Unknown MPE elements must be
  rejected as unsupported; they must not be skipped.

### PCS, intent, and color spaces

- [ ] Complete ICC D50 PCS XYZ and Lab conversion paths in both directions,
  including the required encoding ranges and chromatic adaptation behavior.
- [ ] Implement A2B0/A2B1/A2B2 and B2A0/B2A1/B2A2 selection according to the
  ICC intent-priority rules.
- [ ] Make rendering intent and black-point compensation behavior explicit in
  `TransformOptions`. Unsupported or unavailable requested options must return
  an error rather than be silently ignored.
- [x] Keep format-specific adapters outside the CMS; this release has no
  external format integration and no implicit CMYK convention.

## Historical verification gates

The implementation gates below are recorded for audit; only package and
publish dry-run items remain open on this branch.

- [ ] Preserve all existing tests and add isolated tests for each parser,
  curve, LUT, PCS, intent, and optional-stage combination.
- [ ] Verify matrix/TRC paths against finite reference vectors with absolute
  error at most `1e-5`.
- [ ] Verify LUT paths against a black-box CMS oracle: RGB `17^3`, Gray 4096
  points, and CMYK `9^4`, including endpoints. Record median, p95, and maximum
  Delta E00 limits of 0.1, 0.25, and 1.0 respectively.
- [ ] Verify U8/U16 wrappers against the f32 core for quantization consistency,
  and verify one-thread and concurrent use of the same `Transform` produce
  identical output.
- [x] Verify Gray/RGB, ICC-embedded, and PCS XYZ/Lab paths with synthetic
  vectors; CMYK, N-color, MPE, and BPC remain explicit unsupported paths.
- [ ] Exercise malformed ICC, truncated data, huge tags/CLUTs, integer
  overflow, and resource-limit cases; assert no panic, OOM-triggering
  unbounded allocation, or partial success.
- [ ] Run the full test and documentation suites, all-target checks, MSRV,
  clippy, rustdoc, Miri, Wasm, and 32-bit checks. Run feature combinations
  with defaults and optional SIMD disabled/enabled as applicable.
- [ ] Confirm the package contains all required source, tests, license, README,
  and documentation files, with no ignored fixture or machine-local path
  required for the normal build.

## Release sequence

The release order is fixed so downstream crates never resolve an unpublished
dependency:

1. Implement and review all CMS feature branches, then merge the feature PRs
   into `dev`.
2. Prepare a release PR with version `0.0.5`, changelog, README, MSRV, API and
   feature documentation only after every verification gate passes.
3. Merge the release PR into `master`, run the complete test suite on clean
   `master`, and verify the exact release commit.
4. Before tagging, pass package listing, locked-package, and publish dry-run
   checks from clean `master` and verify the package contents.
5. Create and push the annotated `0.0.5` tag.
6. Publish `icc-profile 0.0.5`; after registry propagation, build an empty
   consumer pinned to exactly `icc-profile = "=0.0.5"` and verify that no path
   or git dependency is selected.
7. Verify crate contents, docs, license, repository metadata, CI, and the
   corresponding GitHub release.
8. Downstream consumers, including WML2, switch to the registry dependency
   only after this consumer verification passes.

Tags are never moved after publication. Post-tag corrections use the
corresponding next patch release and planned downstream version; yanking is
reserved for security, package-integrity, license, or fatal dependency
failures.

## Source and oracle policy

The implementation is clean-room Pure Rust. The normative specification above
is the only implementation reference. External CMS implementations may be
used only as black-box oracles for comparison and benchmarking; their source
must not be viewed, copied, or ported. All unsupported combinations remain
explicit errors until their specification, implementation, and tests are
reviewed and accepted.
