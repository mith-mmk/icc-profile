# Restricted absolute-colorimetric CMYK display

This feature targets print-profile soft-proof input rather than a general
adaptation-state simulation. Its implementation boundary is deliberately narrower
than the complete ICC specification:

- CMYK printer source, v2 mft2 or v4 mAB, explicit A2B1. No absolute A2B0 fallback.
- RGB display matrix/TRC destination with ICC fixed-point D50 media white.
- Both profile header illuminants must be encoded D50. Both media-white tags
  must be valid, finite, and positive.
- Absolute correction belongs to the composed Transform, not standalone
  CompiledProfile CMYK stages. Other unsupported combinations return an error.

## Specification and mathematics

Normative references: [ICC v2](https://archive.color.org/files/ICC_Minor_Revision_for_Web.pdf)
section 6.4.25 and Annex A; [ICC.1:2022](https://www.color.org/specifications/ICC.1-2022-05.pdf)
sections 6.3.2.2, 9.2.15 and 9.2.36.

After decoding source LUT PCS, convert Lab to physical XYZ if needed. Convert
source media-relative XYZ into the destination media-relative XYZ as:

`XYZ_out[i] = XYZ_in[i] * source_wtpt[i] / destination_wtpt[i]`.

Both media whites and LUT PCS data already use the PCS adopted-white convention.
Do not apply chad again to these values. The existing destination matrix/TRC
then converts physical XYZ to RGB. This does not estimate partial adaptation,
black-point compensation, or paper simulation outside the admitted scope.

The existing perceptual/relative/saturation paths and numerical algorithms are
unchanged. Relative CMYK output is documented in cmyk-output.md; BPC remains separate work. Finite positive media
whites alone do not authorize broader old-display or mixed-reference-medium
behavior; rejected cases remain errors instead of guessed conversions.

## Black-box measurements

With the same pinned LittleCMS 2.19 / moxcms 0.9.2 engines and sample licenses
as manufacturer-cms-comparison.md, each source converted all 6561 CMYK u16
samples to sRGB2014. Absolute intent, BPC off, LCMS NOOPTIMIZE|NOCACHE, and
requested tetrahedral 3D interpolation were identical settings.

| Source | icc-profile max / mean absolute u16-code difference from LCMS | moxcms max / mean |
| --- | --- | --- |
| X-Rite CGATS21_CRPC6 | 95 / 5.395824 | 6408 / 1114.055733 |
| Heidelberg PSOcoated_v3 | 74 / 9.435452 | 7736 / 1048.111721 |

Local output had zero samples above the existing 1024-code screening threshold.
moxcms had 5191 / 5256 samples above it respectively. This difference is recorded,
not attributed to an implementation defect or explained by inspecting its source.
No DeltaE acceptance or universal superiority/equivalence is claimed. These two
profile pairs do not establish other profiles or adaptation policies.

Raw hashes, fixed options, status and aggregates are recorded in
cms-comparison-results/absolute-*.json. Reproduction uses the existing harness
with final argument `3` and a new output prefix:

```powershell
python tools/cms-compare/compare.py _test_samples/CGATS21_CRPC6.icc _test_samples/sRGB2014.icc .test-cms-compare/fresh-absolute 3
```

No profile binaries are included in this change. The original license restrictions
and clean-room black-box policy remain unchanged.

## Regression coverage

Synthetic v2/v4 source profiles cover XYZ and Lab PCS, paper white and
non-neutral samples against independent f64 Lab/XYZ equations. Tests exercise
the source/destination version combinations, nonidentity chromatic adaptation
tags without double application, f32/integer workers and transactional output.
Fail-closed cases include missing/malformed A2B1, attempted A2B0 fallback,
invalid or missing whites, non-D50 headers/display white, wrong classes,
selected destination LUTs, and standalone CMYK absolute compilation.
