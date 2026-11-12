# Relative matrix black-point compensation foundation

This deliberately bounded first stage enables the existing
TransformOptions.black_point_compensation flag for relative RGB matrix/TRC
source and destination only. Both profiles must use XYZ PCS and exact encoded
D50 headers; existing v2/v4 and class checks still apply. Destination forward
relative routing must also select a matrix, so an authoritative A2B1/A2B0 LUT
is never silently ignored. Invalid selected tags propagate errors.

LUT/CMYK, Gray, absolute/perceptual/saturation BPC and other unproved routes
remain unsupported. This is not the full CMYK black-estimation algorithm.
BPC-off behavior and its compiled-memory charges are unchanged.

## Algorithm and provenance

The basis is the ICC-approved 2013 committee draft, sections 4.1–4.2.7:
https://archive.color.org/files/BlackPointCompensation.pdf . This is not the
published final ISO 18619:2015 standard and no full ISO conformance is claimed.
Practical background: https://archive.color.org/files/WP40-Black_Point_Compensation_2010-07-27.pdf .

For each retained forward matrix/TRC, evaluate RGB000 and RGB111, choose the
lowest Y (000 on a tie), convert to L*, cap at 50 and neutralize chroma. Reject
nonfinite values, negative Y and curve endpoint values outside [0,1], rather
than guessing a black or silently clamping the estimator. Zero Y is valid.
Use effective neutral black luminances Bs and Bd:

    scale = (1 - Bd) / (1 - Bs)
    XYZout = scale * XYZin + (1 - scale) * D50

Calculations use f64; coefficients must be finite and scale positive. Expansion
(scale > 1) is valid. Equal effective black uses an exact identity shortcut.
The stage runs once between source matrix output and destination inverse.
Neither wtpt, bkpt nor chad supplies a black estimate or an extra correction.
An arbitrary chromatic endpoint is not promised to map to device zero.
Destination inverse-domain failures remain explicit errors, especially with
clamp=false; atomic APIs still preserve caller output on any error.

## Diagnostics, limits and algorithm switching

Transform::bpc_diagnostics() returns None when disabled. Otherwise read-only
BpcDiagnostics/BpcBlackEstimate getters expose estimation method, chosen device
endpoint and XYZ, raw/capped L*, cap flag, effective black Y, scale and offset.
No unchecked caller overrides or new TransformOptions fields are introduced.

The immutable BpcStage is Arc-owned. Its payload size is admitted together with
both routes before materialization, committed transactionally and counted once
in memory_usage. Existing accounting convention excludes Arc control blocks.
Endpoint work is fixed: two endpoints per profile, fixed stack storage, no
extra decoded curves or image-sized allocations. Failure does not yield a
partially constructed transform.

Current estimates depend only on matrix/TRC data, so the existing infallible
with_algorithm_profile remains valid without re-estimation; clones/workers
share the immutable stage. Future LUT BPC must account for interpolation when
estimating black, rather than retaining stale values across algorithm changes.
The current scope explicitly rejects such LUT routes.

## Black-box evidence

Original synthetic matrix fixtures have different neutral black codes1311 and
5243 (u16), plus unmodified licensed ICC sRGB2014 controls. They are not calibrated
hardware profiles. Generator make_bpc_fixture.py never overwrites an existing
file and never modifies vendor data. Existing sRGB terms are recorded in
manufacturer-cms-comparison.md. Profile binaries are not included in git.

Relative BPC was enabled in both local icc-profile and LittleCMS2.19, with
NOOPTIMIZE|NOCACHE|BLACKPOINTCOMPENSATION. Each pair used4913 RGB16 samples:

| Pair | max / mean absolute code difference versus LCMS |
| --- | --- |
| Synthetic black1311 -> black5243 | 4 / 0.549020 |
| Synthetic black5243 -> black1311 | 2 / 0.411765 |
| sRGB2014 -> itself | 3 / 0.647059 |
| sRGB2014 -> synthetic black5243 | 4 / 0.378520 |

Every local/LCMS sample converted successfully. moxcms was explicitly skipped:
the harness does not expose a comparable public BPC option, and disabled-BPC
results must not be presented as equivalent. No external CMS source code was
read. These code differences do not establish DeltaE, photographic quality,
all-profile accuracy or full-standard conformance. Ordinary zero-black RGB
profiles often make BPC identity; practical CMYK work remains a later stage.

Build the fixed harness as in cmyk-output.md, generate the synthetic profiles
with tools/cms-compare/make_bpc_fixture.py, then call compare.py with final
arguments `1 tetrahedral on` and a fresh output prefix. Raw bpc-matrix-*.json
contains hashes, versions, flags and worst samples.

## Contract regressions

Seven independent integration tests cover unequal/zero black, compression and
expansion, v2/v4 pairs, reversed endpoints, deterministic ties, cap50, chromatic
black Y-only neutralization, metadata independence, strict midtones, algorithm
switching/clones/workers and exact/one-byte-under budgets. A valid late device
sample that becomes unreachable in the destination inverse preserves atomic
output. Negative-Y estimator failure after ordinary compilation, followed by
successful exact-budget retries, checks construction cleanup behavior.
