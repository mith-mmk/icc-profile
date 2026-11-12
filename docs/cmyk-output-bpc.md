# Bounded relative CMYK output black-point compensation

This stage connects an RGB XYZ matrix/TRC source to a v4 printer CMYK destination
using explicit B2A1 (relative output), A2B1 (relative forward) and B2A0
(perceptual black probe), with D50 PCS headers. No selected-tag fallback is
accepted. CMYK source-to-CMYK output, v2 output BPC, non-relative BPC, Gray input,
and general LUT input BPC remain unsupported. This is not full ISO conformance.

## Algorithm and diagnostics

The implementation follows the published ICC-approved 2013 committee draft
section 4.2.5: https://archive.color.org/files/BlackPointCompensation.pdf .
It does not reproduce external CMS implementation source. Physical neutral
Lab black passes through B2A0 and A2B1 for the initial estimate; its chroma is
neutralized and L* capped at 50. A 256-point neutral Lab ramp passes through
B2A1 then A2B1. Backward minimum correction makes returned L* monotonic while
preserving the lightest sample. The relative midrange-straight test selects the
initial estimate; otherwise normalized shadow samples in [0.1, 0.5) are fitted
with a quadratic and its positive-gradient root is capped to [0, 50].

This subset rejects invalid, degenerate or underdetermined ramps explicitly
rather than silently substituting the draft's zero-black fallback. Consequently
some otherwise readable profiles can be unsupported with BPC enabled. All such
cases are construction errors, not a partially initialized transform.

The source retains the matrix endpoint estimator. One D50-preserving affine
correction runs in XYZ before encoding destination PCS. Both interpolation
choices have immutable estimates; AlgorithmProfile switching selects matching
diagnostics and coefficients. cmyk_output_bpc_diagnostics reports initial device
black, initial and final L*, selected estimator path, shadow sample count, source
black, interpolation and affine coefficients. Existing input/matrix diagnostic
APIs remain separate.

Both auxiliary LUTs and bounded ramp storage are admitted before decoded
allocation. Temporary construction storage contributes to peak memory but is
not retained for per-pixel execution. Atomic conversion continues to preserve
all caller output after any late failure. CMYK remains four normalized increasing
ink channels; this is not JPEG inverted CMYK or YCCK handling.

## External profile verification

Profiles remain local, unmodified and outside Git. Only original code, hashes
and measurement summaries are committed. See manufacturer-cms-comparison.md
for source links and file-specific usage/redistribution restrictions. The tested
manufacturer profiles are X-Rite CGATS21_CRPC6 and Heidelberg PSOcoated_v3;
these do not establish coverage of all manufacturers or hardware profiles.

LittleCMS 2.19 public API, NOOPTIMIZE|NOCACHE|BLACKPOINTCOMPENSATION, relative
intent, RGB16 17^3 = 4,913 input colors, CMYK16 output. Rust release build.
All numbers below are absolute 16-bit code differences, not DeltaE or a print
quality guarantee. LittleCMS interpolation is engine-selected. The implementation
default is unchanged tetrahedral; trilinear is an explicitly selected alternative.

- sRGB2014 -> X-Rite, explicit trilinear: maximum 43/65535, mean 1.4664665.
- The same profiles, default tetrahedral: maximum 2859, mean 54.355893;
  48 pixels exceed the existing 1024-code screening threshold, 610 exceed 256.
  This result is not called numerical agreement. Previous BPC-off output tests
  already identified interpolation-sensitive differences. A complete isolated
  attribution of every residual in this BPC run is not claimed.
- Heidelberg v2 is parsed but BPC construction is explicitly unsupported. The
  reference engine converts it; no local pixel-error metric is reported.
- moxcms is intentionally skipped: the harness lacks an equivalent public BPC
  setting. BPC-off output is not substituted as an equivalent comparison.

Independent Python implementation of the draft ramp algorithm using only
LittleCMS public Lab/CMYK transforms agrees with its public destination-black
API on both manufacturer samples. Both take the midrange-straight path. X-Rite
reference L*=10.8064391 (Y=0.0123408028); our estimate is L*=10.7975429
(Y=0.0123285204). The difference is retained rather than tuned away. These real
samples do not exercise quadratic fitting, so analytic synthetic tests cover it.

Reports are cms-comparison-results/output-bpc-*.json and include input hashes,
engine versions/flags, worst pixels and exceedance counts. Public-API probes:
tools/cms-compare/probe_output_bpc.py and probe_output_bpc_ramp.py. They use local
_test_samples, never modify profiles, and refuse existing report outputs.
Rebuild tools/cms-compare with its locked dependencies as described in
manufacturer-cms-comparison.md, then run compare.py with a fresh prefix:

    python tools/cms-compare/compare.py _test_samples/sRGB2014.icc _test_samples/CGATS21_CRPC6.icc .test-cms-compare/fresh-output-bpc 1 trilinear on

Use tetrahedral for the default comparison. Switching the destination to
PSOcoated_v3.icc demonstrates the explicit unsupported v2 case.

## Perceptual follow-up

The large CMYK code differences were subsequently evaluated in a common
profile-predicted Lab space, including all pixels of an actual photograph.
See [perceptual impact and replaceable algorithm controls](cmyk-output-perceptual-impact.md)
for DeltaE00 distributions, localized dark-violet cases and limitations.
