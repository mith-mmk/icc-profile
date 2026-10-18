# Paired Lab-PCS LUT regression fixture

`lab-pcs-rgb.icc` is a synthetic RGB monitor calibration serialized by LittleCMS
2.16, not a manufacturer profile. It has v4.3 header, Lab PCS, A2B0 `mAB ` and
B2A0 `mBA ` stages. Each direction has identity curves and a 17×17×17 CLUT
sampled from independent LittleCMS sRGB↔Lab transforms. The creation date is
fixed to 2026-10-18 00:00:00 so regeneration produces stable fixture bytes.

SHA256: `1b7dea170895aacd22a8c76344b06c54ccf18882a573698bb8e8a07f6cfec2b4`.

`lab-pcs-rgb.csv` records RGB16 input/output from transforming through the exact
saved profile in both directions with LittleCMS 2.16 (`cmsGetEncodedCMMversion`
2160). The oracle reopens the serialized file before constructing the transform.
Settings are relative colorimetric intent 1, `TYPE_RGB_16`, flags `0x0140`
(`cmsFLAGS_NOOPTIMIZE | cmsFLAGS_NOCACHE`), BPC off, no alpha. These constants are
from the [official lcms2.16 header](https://github.com/mm2/Little-CMS/blob/lcms2.16/include/lcms2.h).

Regenerate using the system LittleCMS library and Python's standard library:

```sh
python tests/generate_pcs_lut_oracle.py /tmp/.test-pcs-oracle
```

Only regeneration needs LittleCMS; ordinary Rust tests use the retained profile
and CSV. This adds no Rust runtime, build or dev dependency.

`pcs_lut_oracle.rs` allows 1024/65535 normalized error (5 u8 codes) to account for
finite-grid interpolation differences near gamut boundaries and u8 input
quantization. It catches the original paired-stage error by a large margin:
mid-gray expected `[32753,32772,32778]` became `[458,182,294]`, and white expected
`[65209,65448,65292]` became `[1299,626,637]`.

The separate `physical_pcs_bridge_covers_matrix_and_xyz_lab_luts` regression
uses a tighter normalized tolerance of 1e-4 with fixed CIE D50 reference vectors
from LittleCMS 2.16's public `cmsLab2XYZ` and `cmsXYZ2Lab` functions. It covers
matrix, modern/legacy XYZ LUT and modern/legacy Lab LUT routes in both directions
for black, gray, chromatic Lab and white, under relative and absolute intents.
The absolute reference scales physical XYZ by 0.8 before converting back to Lab;
fixed s15Fixed16 media-white encoding explains the small allowed tolerance.
