# Checked legacy LUT conversions

The legacy `cms::transration` module now offers 21 additive `try_` scalar functions.
Every raw `d4_to_d3`, `d3_to_d4`, `d3_to_d3`, and `d3_to_d1` LUT8/LUT16 helper,
CMYK-to-Lab and Lab-to-CMYK scalar variant (including float Lab and `_u8`), and
LUT-based CMYK-to-RGB scalar helper has a checked counterpart with identical
arguments and `std::io::Result<OriginalOutput>` return type.

```rust
use icc_profile::{Mft2, cms::transration::try_cmyk_to_lab_lut16};
fn convert(lut: &Mft2) -> std::io::Result<(f64, f64, f64)> {
    let lab = try_cmyk_to_lab_lut16(0, 0, 0, 0, lut)?;
    Ok(lab)
}
```

Migrate `name(...)` to `try_name(...)?` inside a Result-returning caller, or match
on the result. No panic-catching wrapper or fabricated fallback color is needed.
The original signatures remain available and produce the same valid outputs,
but **explicitly panic on invalid inputs**. They are not panic-free APIs.

Checked calls validate the mutable LUT on every call: matching input/output
channel counts, at least two grid points and table entries, sufficient input,
CLUT, and output storage, and nine matrix coefficients for three-channel inputs.
Trailing storage remains accepted for compatibility. Four-channel helpers do
not use the matrix. Checked size arithmetic and batch allocation failures return
`InvalidInput`. Float Lab must be finite; finite out-of-range Lab retains the
legacy saturation/quantization behavior. White-point components must be finite
and positive. Invalid input returns `InvalidInput`, never a substitute color.

All eight Lab/CMYK batch wrappers call the checked scalar path and reject short
buffers and overflowing entry counts. Empty batches continue to succeed without
validating the unused LUT. Non-finite float Lab now returns an error in batches.

## Units and limits

- Raw scalar inputs remain encoded bytes, including LUT16 inputs.
- Raw LUT16 outputs remain 0..65535; raw LUT8 outputs remain bytes.
- Lab-to-CMYK floating outputs remain percentages, and `_u8` outputs remain bytes.
- Existing Lab encodings, rounding, and three-channel matrix semantics are unchanged.
- These are supplied-LUT helpers, not complete ICC profile transforms. They do not
  infer PCS metadata, choose intent tags, apply profile adaptation, or validate
  that a caller's LUT actually represents the named color conversion.
- Full-profile `*_from_profile` functions and their nearest-neighbor behavior are
  outside this change. The simple inverted-CMYK function and gamma byte domain
  are also unchanged. Use the compiled transform API for its supported routes.

Synthetic tests cover all checked scalar APIs without external profile files.
