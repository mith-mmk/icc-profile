# Restricted absolute RGB to CMYK output

This stage extends relative output with a deliberately narrow absolute route:

- Source is an RGB XYZ matrix/TRC monitor, with exact ICC fixed-point D50 wtpt.
- Destination is a CMYK printer, v2 mft2 or v4 mBA, with explicit B2A1.
- Both header illuminants are encoded D50. Both media whites are checked,
  finite and strictly positive. Printer media white may be non-D50.
- No B2A0 fallback, numerical A2B inversion, source LUT fallback, BPC or
  standalone absolute CompiledProfile stage. Perceptual/saturation remain
  separate work; relative output is unchanged.

The existing physical XYZ bridge multiplies each component by
source_wtpt / destination_wtpt once before destination PCS encoding.
wtpt is already represented in the adopted PCS-white coordinates, so chad is
not applied again. This is the inverse direction of the restricted absolute
CMYK display path in cmyk-absolute.md and shares its normative references:
ICC v2 section 6.4.25 / Annex A and ICC.1:2022 sections 6.3.2.2 / 9.2.36.
It is not general partial-adaptation simulation. Rejected routes remain errors.

## Black-box evidence

Same unmodified licensed profiles and fixed engines as cmyk-output.md;
no profile binaries are committed. sRGB2014 to each printer, 4913 RGB16 grid
points, absolute intent, BPC off. LCMS 2.19 interpolation is engine-selected
with NOOPTIMIZE|NOCACHE. moxcms 0.9.2 requests tetrahedral; local interpolation
is explicitly recorded rather than presumed equivalent across engines.

| Printer | Local default tetrahedral max / mean vs LCMS | Local trilinear max / mean vs LCMS |
| --- | --- | --- |
| X-Rite CGATS21_CRPC6 | 2881 / 49.965042 | 31 / 0.930694 |
| Heidelberg PSOcoated_v3 | 1623 / 46.809027 | 23 / 0.774018 |

All engines converted every sample. Default tetrahedral had 44 / 17 pixels
over the 1024-code screening threshold; explicit trilinear had zero.
The default remains unchanged. moxcms differences were respectively max 6666 /
8961 and mean 1184.752290 / 1303.738805. These observations are not attributed
to a defect or evidence of universal engine equivalence/superiority. No DeltaE
or print-quality acceptance is claimed. All external comparison is black-box.

Raw output-absolute-*.json records settings, hashes, per-channel maxima and
worst samples. Reproduce with the existing harness using final arguments
`3 trilinear` or `3 tetrahedral` and a fresh prefix. Contracts and fixed-size
four-channel atomic scratch are unchanged from cmyk-output.md.
