# Profile-free conversions

These new functions do not load profiles or replace ICC transform errors. They
cover explicitly encoded YCbCr, linear RGB/XYZ matrix operations and XYZ/Lab.
All scalar pixels and packed buffer pixels contain exactly three components;
there is no alpha, row stride or padding.

## Full-range byte YCbCr

```rust
use icc_profile::simple::{rgb8_to_ycbcr8_full_buffer, YcbcrCoefficients};
let ycbcr = rgb8_to_ycbcr8_full_buffer(
    &[255, 0, 0, 0, 0, 255], YcbcrCoefficients::Bt601,
)?;
assert_eq!(ycbcr, [76, 85, 255, 29, 255, 107]);
# Ok::<(), icc_profile::simple::SimpleError>(())
```

`rgb8_to_ycbcr8_full` and `ycbcr8_full_to_rgb8` are scalar counterparts;
`ycbcr8_full_to_rgb8_buffer` converts complete buffers in reverse. `Bt601`
and `Bt709` select luma/chroma coefficients. Codes are full-range `0..=255`,
chroma center 128, rounded/clipped to bytes using the established helper math.
No transfer function or studio-range encoding is implied, and rounding makes
byte round trips lossy. PAL/custom coefficients retain their legacy entrypoints.

## Linear RGB and XYZ

```rust
use icc_profile::simple::{linear_rgb_f64_to_xyz_f64, ColorMatrix3D};
let matrix = ColorMatrix3D::d65_rgb_to_xyz();
let xyz = linear_rgb_f64_to_xyz_f64([1.0, 0.0, 0.0], &matrix)?;
assert!(xyz.iter().all(|value| value.is_finite()));
# Ok::<(), icc_profile::simple::SimpleError>(())
```

Use `xyz_f64_to_linear_rgb_f64` with an **XYZ-to-RGB matrix**, not the forward
matrix. Both names also have `*_buffer` variants. Matrices are row-major;
`ColorMatrix3D` is the existing type with presets and explicit checked inverse.
No inverse, transfer curve or chromatic adaptation is inserted automatically.
RGB must already be linear. XYZ is relative/physical tristimulus data on the
matrix's scale (normally white Y=1), not ICC fixed-point or normalized LUT codes.
Finite values outside nominal ranges remain unclipped, including negative RGB.

## XYZ and Lab

```rust
use icc_profile::simple::{lab_f64_to_xyz_f64, xyz_f64_to_lab_f64, WhitePoint};
let white = WhitePoint::d50();
let xyz = [white.x, white.y, white.z];
let lab = xyz_f64_to_lab_f64(xyz, &white)?;
assert!((lab[0] - 100.0).abs() < 1e-9);
let roundtrip = lab_f64_to_xyz_f64(lab, &white)?;
assert!((roundtrip[1] - xyz[1]).abs() < 1e-12);
# Ok::<(), icc_profile::simple::SimpleError>(())
```

Both names have `*_buffer` variants. XYZ and the reference white must use the
same scale. White components must be finite and strictly positive. Lab is
physical L* (nominally 0–100), signed a* and b*, not Lab8/Lab16 encoding. Inputs
and outputs are finite but not clipped to nominal ranges. `WhitePoint` reexports
the existing type with its legacy methods, including unchecked methods.
Likewise `ColorMatrix3D` retains unchecked methods; checked guarantees here refer
to the new named conversion functions. The whitepoint rounded CIE presets and encoded ICC PCS D50 preset are
not interchangeable precision promises. No whitepoint adaptation occurs.

## Validation and compatibility

Buffers process every sample and reject lengths not divisible by three. Empty
buffers succeed after validating any supplied matrix/whitepoint. Nonfinite
inputs and arithmetic overflow are errors. Allocation uses `try_reserve_exact`;
no partial Vec is returned. Unlike ICC execution helpers, these functions have
no configured 64 MiB output cap; callers should bound untrusted image sizes.
`SimpleError` and `YcbcrCoefficients` are non-exhaustive for future additions.

Legacy count-taking APIs retain their accepted surplus tails and individual
RGB/RGBA layouts. Gamma byte-domain behavior, unusual byte XYZ/Lab encodings,
inverted YMCK, raw LUTs and profile approximations are not relabeled as these
new functions. See [the migration map](api-migration.md) and
[legacy contracts](standalone-color-contracts.md).
