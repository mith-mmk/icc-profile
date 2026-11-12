# Relative CMYK input black-point compensation

This stage extends relative BPC to v4 printer CMYK profiles with an explicit
A2B1 lutAToBType and a same-profile explicit B2A0 lutBToAType. The RGB destination
must select matrix/TRC in both directions, with XYZ PCS. Both profiles require
D50 PCS headers. Missing, malformed or fallback routes are rejected. This does
not enable CMYK output BPC, v2 CMYK BPC, Gray BPC or non-relative intents.

## Basis and estimator

The public ICC-approved 2013 committee draft, section 4.2.3, specifies black
estimation for output-capable CMYK sources:
https://archive.color.org/files/BlackPointCompensation.pdf
This work does not claim conformance to the final published ISO 18619 standard.

Physical Lab (0,0,0), equivalent to physical XYZ zero, passes through the
source profile's perceptual B2A0. The resulting four-channel device black passes
through its relative A2B1. Lab LUT encoding must represent neutral a*/b*, not
raw normalized zero. There is no recursive BPC, reference-black lift, media
white ratio or extra chad adjustment within this auxiliary probe. Retain the
resulting luminance, cap L* above 50 and neutralize chroma as in the matrix BPC
foundation. Invalid probes are errors, not silently substituted zero black.

The destination uses the existing RGB endpoint estimator. The affine BPC stage
runs once after source PCS decoding and before destination matrix inversion.
Strict inverse-domain failures remain explicit. Atomic calls preserve caller
output on failure, including failures after an earlier valid pixel.

## Interpolation and resources

Both implemented 3D interpolation choices are estimated and validated before
a transform is returned. Selecting an AlgorithmProfile selects its matching
coefficients and diagnostics; earlier clones and workers retain their own
selection. CMYK forward interpolation remains 4D multilinear. No image-sized
scratch allocation or CMS-name compatibility promise is introduced.

The auxiliary reverse LUT is admitted with both main routes and BPC storage
before decoding. Its temporary ownership contributes to the construction peak;
after estimation it is dropped, rather than retained for ordinary execution.
Resident and construction-peak memory are distinct. Limits include auxiliary
curve/CLUT entries and decoded storage. Errors during construction do not return
a partially usable transform.

## Diagnostics compatibility

The pre-existing bpc_diagnostics accessor remains matrix-BPC specific; it does
not truncate CMYK into its three-component endpoint type. A CMYK-specific
accessor exposes the full four-component device black and auxiliary
route, active interpolation, PCS black estimate and affine coefficients. The
forward route remains available through Transform::input_route_info().

## Validation

Synthetic contract tests, licensed real-profile measurements and their exact
results are recorded when completed. No unverified numerical agreement or
final-standard conformance is implied by this design description.

## Recovered verification, 2026-11-12

Each external measurement covers 6,561 CMYK grid colors, u16 RGB output,
relative intent, BPC enabled, LittleCMS 2.19 with NOOPTIMIZE and NOCACHE.
The unchanged X-Rite CGATS21_CRPC6 -> sRGB2014 comparison has maximum 96/65535
and mean 6.58 code differences for either selected 3D algorithm. No sample
exceeds 256 codes. This is not a DeltaE quality or universal compatibility claim.

The minimal original synthetic profiles intentionally contain only A2B1 and
B2A0. Their large differences (Lab trilinear max 1757; XYZ max 3620) are retained
in the comparison records, not counted as agreement. A public-API tag-admission
experiment isolates a material behavior difference: cmsDetectBlackPoint returns
success with XYZ zero for these minimal profiles. Adding A2B0 alone changes the
result to nonzero; adding B2A1 alone does not. A2B1/B2A0 bytes stay identical.
Adding all six intent tags and metadata does not further change the detected
black. This is an observed engine precondition, not a claim about its internal
implementation. The production estimator is not changed to mimic this behavior.

With complete synthetic tags, XYZ maximum difference is 7 and mean 4.71;
Lab trilinear maximum is 10 and mean 8.47. Lab tetrahedral remains maximum 218
and mean 196.86. Its bilinear cross-term reverse fixture deliberately makes
interpolation algorithms differ. The independent f64 contract oracle validates
both local algorithms; the external engine's interpolation is engine-selected.
Residual numeric differences are retained, not declared exact compatibility.
Heidelberg v2 remains an explicitly rejected BPC case, not a zero-error result.

Records: `cms-comparison-results/bpc-cmyk-*.json` and
`cms-comparison-results/bpc-tag-experiment-results.json`. The tag experiment uses
only public LittleCMS functions and original synthetic bytes; it never edits a
manufacturer profile. Run `cargo test --test bpc_cmyk_input
export_original_cmyk_bpc_fixtures -- --ignored`, then
`python tools/cms-compare/bpc_tag_completeness_experiment.py` in a fresh output
directory. The exporter uses `.test-cms-compare`; the experiment accepts that
directory as an optional argument and refuses existing experiment outputs.
Build the comparison harness as in manufacturer-cms-comparison.md, then use
`compare.py SOURCE DESTINATION FRESH_PREFIX 1 trilinear on` (or tetrahedral).

