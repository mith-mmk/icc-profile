# Bounded v4 perceptual CMYK output

This stage evaluates explicitly authored perceptual tables with a common v4
perceptual reference-medium (PRM) basis. ICC.1:2022 section 6.3.3 defines that
basis; see https://www.color.org/specifications/ICC.1-2022-05.pdf .

Composite scope is RGB v4 actual LUT A2B0 to CMYK v4 printer mBA B2A0,
with explicit selected tags, no fallback, and encoded D50 header illuminants.
Standalone PcsToDevice B2A0 accepts physical XYZ expressed in perceptual PCS,
not arbitrary colorimetric XYZ. The existing PCS XYZ/Lab conversion connects
the two LUTs; no new gamut map, reference-black correction or chad operation
is guessed. Existing relative/absolute output contracts remain unchanged.

Matrix input, v2 or mixed-major input/output, missing/malformed selected tags
and saturation output remain rejected. Same-major v2 is not sufficient proof
of a common perceptual reference medium. Saturation rendering is vendor-defined
and is not silently treated as equivalent to perceptual. These are deliberate
initial implementation boundaries, not universal restrictions on ICC workflows.

## Measurements and limitations

Existing unmodified licensed ICC sRGB v4 preference -> X-Rite CGATS21_CRPC6
profiles were compared on 4913 RGB16 grid samples using LittleCMS 2.19 and
moxcms 0.9.2 through public APIs only. Both profile binaries remain outside git.
For license provenance and fixed options see manufacturer-cms-comparison.md.

All engines converted all samples. Local explicit trilinear interpolation had
max 8 / mean 0.808416 u16-code difference versus LCMS. The unchanged default
(tetrahedral) had max 1932 / mean 60.958885, 19 pixels over the 1024-code
screening threshold. moxcms had max 1191 / mean 60.428862, one pixel over it.
LCMS interpolation is engine-selected; moxcms requests tetrahedral. Settings
are not claimed algorithmically identical. No DeltaE, print-quality acceptance
or universal CMS compatibility is claimed. One real accepted profile pair is
not a comprehensive perceptual reference-medium conformance suite.

The same v4 source -> Heidelberg v2 PSOcoated_v3 pairing is explicitly rejected
by this stage, recorded in output-perceptual-mixed-rejected.json. External
engines converting it does not establish a supported local mixed-version route.

Raw reports include hashes, options, per-channel maxima and worst cases.
Reproduce with tools/cms-compare/compare.py using the above source/destination,
a fresh output prefix and final arguments `0 trilinear` or `0 tetrahedral`.

## Contract tests

Independent f64 fixtures cover XYZ/Lab cross-PCS conversion, distinct selected
A2B/B2A tags, asymmetric K, standalone execution, nonzero black without extra
mapping, wtpt/chad invariance, both interpolation modes and 48-pixel worker
boundaries. Missing/malformed routes, matrix/mixed-version sources, invalid
headers and unsupported saturation fail closed. No external CMS implementation
code was used to construct the oracle.
