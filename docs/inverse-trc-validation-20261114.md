# Inverse parametric TRC performance validation

Base: `5d145f60365c731b18c39469a182825f4d4808cd` (the 0.1.0 candidate
branch, still identifying itself as version 0.0.8). This is an optimization
and validation change, not a release or downstream dependency upgrade.

## Algorithm and compatibility

The endpoint policy determines whether the old search returns a lower or upper
bound. Search only that bound, and stop after updating a bound when f32 rounding
makes the midpoint equal an endpoint. Keep the original 32-iteration cap.

For increasing, positive type-3 parametric curves, use an analytic inverse only
to propose a cell on the exact 2^-20 dyadic grid. Validate the original search
predicate at both cell endpoints. Under the checked monotonicity conditions,
these are the exact bounds the original search would have reached after 20
iterations. Continue the remaining 12 original iterations, including their
rounding, endpoint and plateau policy. If any precondition or bracket check
fails, start at [0,1] as before. No approximate inverse is returned, no inverse
LUT is stored, and no public API or output-write path changes.

In particular, decreasing curves and other function types retain bisection;
plateau and signed-zero behavior is tested against the original algorithm.
The integer API still evaluates privately before writing, preserving atomic
output on errors. This change does not remove that duplicate evaluation.

## Verification

Host: macOS arm64, Rust 1.97.0. No Windows, Linux, wasm or MSRV claim.

- `cargo test --offline`: 365 passed, zero failures, 70 ignored. Some legacy
  external-fixture tests self-skip when their files are absent; this is not
  exhaustive real-profile coverage.
- Both new inverse-search tests also pass in release mode. Frozen old-search
  bit comparisons cover all five functions, increasing/decreasing curves,
  lower/upper plateaus, discontinuities, out-of-range and tiny finite targets,
  signed zero, fixed-point-quantized coefficients, dyadic boundaries and adjacent
  f32 values, plus deterministic random bit-pattern targets. The seed test
  directly proves a successful bracket equals the first 20 original steps and
  checks rejected seeds.
- Seven macOS RGB profiles and a real SWOP CMYK profile, as sources against the
  seven RGB destinations, across all four intents: 224 combinations attempted.
  156 supported combinations, 4096 varied samples each, tested with u8/u16/f32.
  All 468 output files match the old version byte-for-byte; the 68 rejected
  combinations and their errors are unchanged. Profile hashes are in the JSON.
  Unsupported combinations are not counted as color-conversion passes.
- WML2 release-mode lib/integration regression suite with the candidate explicitly
  selected: 373 passed, zero failures, nine ignored.
- Five real JPEG/TIFF/PSD decode outputs match the old candidate PNGs byte-for-byte.
  The ordinary PSD is compatibility coverage, not a CMYK PSD color oracle.
- A separate 625-swatch SWOP test against previously generated macOS ColorSync
  reference data is unchanged: relative-colorimetric max difference 1/255,
  mean 0.3712/255. Perceptual/saturation retain an existing max 48/255 difference,
  mean 15.8123/255; they are **not** accuracy passes. Absolute is unsupported for
  that profile pair. ColorSync itself was not rerun in this change.
- `rustfmt --check --edition 2021 src/transform/curve.rs` and `git diff --check`
  pass. Repository-wide `cargo fmt --check` already fails in unrelated files.
  `cargo clippy --offline -- -D warnings` fails with the same 97 existing errors
  on both the base and changed versions. Those failures are not claimed fixed.

## Release timing

Same-host sequential baseline/optimized runs, separate Cargo target directories,
11-decode medians and 7-run stage medians. Times are microseconds, not CI gates.
The WML2 source is the validation snapshot described in its
`docs/cmyk-validation-20261114.md`; only a command-line Cargo path patch selects
ICC. WML2 manifests and committed lockfile are not updated.

| Operation | Base | Optimized |
| --- | ---: | ---: |
| 500x333 ICC JPEG decode | 598138 | 143840 |
| u8 transform, 166500 varied CMYK pixels | 585753 | 135864 |
| f32 transform, same samples | 298537 | 67574 |
| u8, 256-pixel calls | 584455 | 135214 |
| f32, 256-pixel calls | 290337 | 67428 |
| Profile parse | 17 | 17 |
| Transform construction | 58 | 51 |
| Identity-TRC f32 diagnostic only | 15510 | 15513 |
| 256x256 CMYK text JPEG | 486 | 483 |
| 157x151 CMYK TIFF | 2089 | 2079 |
| 640x426 ordinary PSD | 1953 | 1935 |
| 625x1 SWOP TIFF | 1153 | 344 |

The tagged JPEG is 4.16x faster (76% less time), and the u8 stage is 4.31x
faster. Reported resident transform memory remains 642196 bytes. This is not a
peak-RSS measurement. About 144ms remains material for interactive large-image
use; this does not claim viewer performance is fully solved.

## Reproduction and limits

Run the repository tests above, then the WML2 opt-in
`cmyk_external_validation` tests with `--release -- --ignored --nocapture
--test-threads=1`, `WML2_TEST_CMYK_ICC=transform`, a fresh target directory and
`--config 'patch.crates-io.icc-profile.path="/absolute/ICC/path"'`.
Set `WML2_CMYK_PROFILE_JPEG` to the tagged CMYK JPEG, and supply
`WML2_CMYK_SAMPLE_FILES`, `WML2_CMYK_OUTPUT_DIR`, and
`WML2_CMYK_BENCH_REPEATS=11` as documented by WML2. Restore any temporary Cargo
lockfile change. Compare baseline/optimized decoded PNG bytes.

Private/local profile samples are not vendored. Local probe source, logs and
small comparison summaries are retained beside the validation checkout;
unneeded targets and bulk comparison binaries are moved to Trash. This is not
exhaustive fuzzing, all-profile accuracy certification, or a new CMS oracle.
No merge, publication, release, or WML2 dependency update is performed.
