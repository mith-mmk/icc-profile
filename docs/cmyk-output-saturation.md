# Explicit v4 saturation LUT connection

This stage connects RGB v4 A2B2 to CMYK v4 printer mBA B2A2. Both tables must
be explicitly present and selected without fallback; both headers must encode
D50. Matrix sources, v2/mixed-major profiles, missing/malformed selected tags
and BPC remain rejected. Relative, restricted absolute and perceptual behavior
are unchanged. Standalone B2A2 accepts intent-specific PCS values expressed in
physical XYZ units, not arbitrary measured colorimetric XYZ.

ICC.1:2022 sections 6.2.5 and 9.2.48 define saturation rendering and its optional
reference-gamut tag: https://www.color.org/specifications/ICC.1-2022-05.pdf .
Saturation rendering is vendor-specific. This is execution and PCS connection
of author-provided tables, not a claim of common perceptual reference-medium
quality. rig2 is optional; its absence is not a reason for fallback or rejection.
The implementation adds no black-point, gamut or chad adjustment. It preserves
floating-point PCS between stages, with ordinary XYZ/Lab representation conversion.

## Reproducible black-box check

The available real sRGB profiles do not have A2B2. We therefore generated an
original synthetic v4 RGB signal -> encoded-Lab cube with explicit identity
A2B2 and a different gamma-2 A2B0. It is not a calibrated RGB space, real image
or production profile. tools/cms-compare/make_saturation_fixture.py generates it
without overwriting an existing file. No vendor profile is modified.

With the existing licensed X-Rite CGATS21_CRPC6 output profile and 4913 RGB16
signal samples, all three engines converted every sample. Fixed LittleCMS2.19
NOOPTIMIZE|NOCACHE and moxcms0.9.2 were used through public APIs only; BPC off.
Local explicit trilinear vs LCMS: max3 / mean0.362609 codes. Unchanged local
default tetrahedral: max886 / mean11.762620. moxcms: max1697 / mean79.964380,
26 pixels above the 1024-code screening threshold. Local runs had zero above it.
LCMS interpolation is engine-selected and moxcms requests tetrahedral; algorithms
are not asserted equivalent. This verifies table execution for a synthetic
input, not photographic color quality, DeltaE acceptance or universal compatibility.

Rejected cases are also recorded: the actual ICC sRGB v4 preference source
lacks A2B2, so its A2B0 fallback is refused; Heidelberg PSOcoated_v3 is v2 and
its B2A2 is mft1, outside this stage. External engines accepting these inputs
does not make them locally supported. Manufacturer originals remain unmodified
and excluded from git; license sources are in manufacturer-cms-comparison.md.

## Reproduction

From the repository root, with fresh paths:

```powershell
python tools/cms-compare/make_saturation_fixture.py .test-cms-compare/fresh-saturation.icc
python tools/cms-compare/compare.py .test-cms-compare/fresh-saturation.icc _test_samples/CGATS21_CRPC6.icc .test-cms-compare/fresh-saturation-result 2 trilinear
```

Build the pinned harness first as described in cmyk-output.md. Raw reports
record source/destination hashes, settings, per-channel maxima and worst samples.

Independent synthetic contract tests cover XYZ/Lab, distinct intent0/1/2 tables,
K, both interpolation choices, standalone, optional metadata independence,
48-pixel integer/atomic boundaries and unsupported-route errors.
