# Standalone legacy color helper contracts

These contracts describe existing `cms::transration` helpers, not a replacement
for checked `Profile`/`Transform` processing or a claim of ICC-conformant results.

- **RGB and XYZ:** Matrix operations use linear RGB. `SrgbD65` chooses a matrix;
  it does not apply an sRGB EOTF/OETF, TRC or chromatic adaptation. Decode/encode
  transfer functions separately when working with encoded image samples.
- **XYZ bytes:** A simplified 0..1 XYZ scale maps to 0..255. Values above 1,
  including D65 white Z, clip in byte output. This is not ICC PCS XYZ encoding.
  Float XYZ can exceed 1. Matrix byte batches truncate after finite clipping;
  scalar float-to-byte paths retain their existing rounding.
- **Lab bytes and words:** XYZ-to-Lab byte helpers map L to 0..255 and offset
  a/b by 128. Word output maps L by 65535/100 and a/b by `(component + 128) * 255`,
  with truncation and clipping. These legacy encodings must not be assumed to
  implement ICC v2 or v4 Lab PCS encoding.
- **Whitepoint:** `WhitePoint::from_profile` reads the header illuminant, not
  `wtpt`. Supplying a whitepoint does not itself perform chromatic adaptation.
- **Bare CMYK helper:** `cmyk_to_rgb(y, m, c, k)` takes legacy **Y/M/C/K**
  arguments and uses inverted/additive byte components, not standard ink CMYK:
  RGB is `clamp(c + k - 255, m + k - 255, y + k - 255)`. All zero maps to
  black; all 255 maps to white. No profile is required. Do not pass conventional
  C/M/Y/K ink values without an explicit caller-side interpretation.
- **RGBA to YCbCr:** `rgba_to_yuv_entries` reads RGBA quads, discards alpha and
  returns Y/Cb/Cr triplets. The old reversed name
  `yuv_to_rgba_entries_from_rgb` remains a compatibility alias with identical
  behavior.
- **YCbCr to RGBA:** `yuv_to_rgba_entries_from_yuv` always writes alpha 255,
  independently of luma. BT.601/709 helpers use full-range centered chroma,
  not studio-range scaling or subsampled chroma. The PAL preset is an analog
  coefficient approximation represented in centered bytes.
- **XYZx to RGBA:** `xyz_to_rgba_entries` reads four bytes per pixel
  `[X, Y, Z, ignored]`. It drops the fourth byte and writes alpha 255. The
  three-byte input API `xyz_to_rgb_entries` outputs RGB rather than RGBA.
- **ColorEntries:** All seven `to_*` methods are unimplemented and return `None`.
  A generic implementation would require whitepoint, encoding and profile
  choices that this container currently does not provide.

These clarifications and the RGBA naming alias do not change old signatures,
channel interpretation, RGB transfer functions or finite clipping behavior.
