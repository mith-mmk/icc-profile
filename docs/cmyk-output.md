# Relative RGB / PCS to CMYK output

This stage adds explicit printer B2A1 execution: v2 mft2 and v4 mBA,
three PCS channels to four device channels. Composite transforms require an
RGB source. Standalone PcsToDevice accepts physical D50 XYZ, including for
Lab-profile PCS. The selected B2A1 is executed; no A2B numerical inversion is used.

This first stage supports relative colorimetric only. Missing B2A1, B2A0
fallback, other intents, Gray/CMYK composite sources, non-printer output,
mft1, wrong-direction mAB and BPC remain errors. This is an implementation
boundary, not a claim that ICC forbids those routes.

Normative references: [ICC v2](https://archive.color.org/files/ICC_Minor_Revision_for_Web.pdf)
and [ICC.1:2022](https://www.color.org/specifications/ICC.1-2022-05.pdf),
intent tag selection, lut16Type and lutBToAType definitions. B2A1 carries
relative PCS-to-device mapping; mBA runs B curves, matrix, M curves, CLUT,
then A curves. PCS and matrices remain three-channel; the CLUT and final
curves/tables retain all four output channels, including K.

## Buffer and resource contracts

Existing public signatures and defaults are preserved. Single-pixel atomic
execution uses four stack floats (16 bytes). Image f32 atomic scratch stays
192 floats; integer execution uses two 192-float buffers. Four-output paths
process at most 48 pixels per chunk. Validation precedes publication, so
all four caller output components remain bitwise unchanged on errors.
CLUT-entry and compiled-byte budgets include all four components.

Synthetic tests cover distinct C/M/Y/K, an independent K curve/table,
nonlinear interpolation, mBA stage order, RGB matrix/LUT sources, standalone
PCS output, 48-pixel boundaries, late invalid input and exact resource limits.

## Black-box comparison

Existing licensed, unmodified printer profiles were used locally only; see
manufacturer-cms-comparison.md for license sources. No profile binaries are
included. Each comparison uses 4913 points of a 17-cubed sRGB2014 RGB16 grid,
relative intent, BPC off, LittleCMS 2.19 NOOPTIMIZE|NOCACHE and moxcms 0.9.2.
External CMS implementation sources were not inspected.

| Printer | Default tetrahedral max / mean code difference from LCMS | Explicit trilinear max / mean |
| --- | --- | --- |
| X-Rite CGATS21_CRPC6 | 4455 / 52.344342 | 36 / 0.937767 |
| Heidelberg PSOcoated_v3 | 1455 / 46.112609 | 23 / 0.793558 |

All three engines converted all samples without errors. Default tetrahedral
had 43 and 18 pixels respectively over the existing 1024-code screening
threshold; explicit trilinear had zero. Independent f64 evaluation of the
X-Rite tables reproduced the tetrahedral result and localized the large
difference to interpolation. The library default remains tetrahedral.
Use AlgorithmProfile::with_clut_3d(ClutInterpolation3D::Trilinear) explicitly
when that algorithm is desired. This is not a named CMS compatibility mode.

LCMS interpolation is engine-selected, not forced by this harness. moxcms
continues to request tetrahedral; its observed maximum differences from LCMS
are 580 and 547 codes. Its internal algorithm is not asserted. Consequently,
the trilinear experiment does not claim identical interpolation settings
across all engines. Code differences are not DeltaE or a printing-quality
acceptance threshold, and do not establish universal profile compatibility.

Raw reports include hashes, engine options, per-channel maxima and worst
samples. Reproduce from the repo root with a fresh output prefix:

```powershell
$env:CARGO_TARGET_DIR='D:\codex\icc-profile\target\cms-compare'
cargo build --offline --locked --release --manifest-path tools/cms-compare/Cargo.toml
python tools/cms-compare/compare.py _test_samples/sRGB2014.icc _test_samples/CGATS21_CRPC6.icc .test-cms-compare/fresh-output 1 trilinear
```

Use PSOcoated_v3.icc for the other printer, or tetrahedral for the historical
default. CMYK output intents beyond relative and general BPC are future stages.
