# icc-profile

Pure Rust ICC profile parsing and explicit Gray/RGB color transforms.

Version 0.0.7 provides a checked ICC v2/v4 CMS for Gray and
RGB profiles. The processing core is `f32` and supports D50 PCS, XYZ/Lab,
chromatic adaptation (`chad`), Gray TRC, RGB matrix/TRC, `curv`, parametric
curves 0-4, `mft1`, `mft2`, `mAB`, `mBA`, A2B/B2A intent selection, optional
stages, 1D interpolation, and tetrahedral 3D interpolation.

Version 0.0.7 corrects the physical PCS connection between paired LUT stages
when the destination uses Lab PCS. Matrix destinations continue to use XYZ.

CMYK, N-color, MPE, black-point compensation, and other unsupported routes
return `UnsupportedProfileFeature`; there is no simplified color fallback.
Alpha is not passed to the CMS and must be handled by the caller.

## Interpolation and LittleCMS compatibility

The 3D CLUT evaluator uses tetrahedral interpolation. LittleCMS 2.16
[selects trilinear interpolation for Lab-PCS output (B2A) LUTs](https://github.com/mm2/Little-CMS/blob/lcms2.16/src/cmsio1.c#L621-L624),
so matching profiles and rendering intents need not produce identical values.

In a 10,010-color sRGB-to-synthetic-Lab-PCS RGB LUT comparison, the maximum
RGB8 channel difference was 18/255. Evaluating the same LUT with trilinear
interpolation reduced the observed difference to at most 1/255. The comparison
used relative colorimetric intent, BPC off, and LittleCMS
`NOOPTIMIZE | NOCACHE`; these figures are observations, not general error bounds.

Neither method is universally more accurate; results depend on the profile
and samples. This comparison does not establish an ICC specification violation,
and numerical comparisons with manufacturer Lab-PCS output profiles remain
unverified. The current interpolation behavior is unchanged.

## Example

```rust
use icc_profile::{Profile, RenderingIntent, Transform, TransformOptions};

fn convert(source_icc: &[u8], destination_icc: &[u8], input: &[f32])
    -> Result<Vec<f32>, Box<dyn std::error::Error>>
{
    let source = Profile::from_bytes(source_icc)?;
    let destination = Profile::from_bytes(destination_icc)?;
    let transform = Transform::new(
        &source,
        &destination,
        TransformOptions {
            rendering_intent: RenderingIntent::RelativeColorimetric,
            ..TransformOptions::default()
        },
    )?;
    let mut output = vec![0.0; input.len()];
    transform.transform_f32(input, &mut output)?;
    Ok(output)
}
```

## Testing

The project includes checked parser and transform tests for ICC profile
handling and color space conversions.

### Running Unit Tests (No Samples Required)

```bash
cargo test --lib
```

This runs the no-fixture unit and synthetic transform tests.

### Running Full Test Suite (Requires ICC Profiles)

External fixture tests are optional and are not required for packaging:

1. **Prepare test samples** by creating `_test_samples/` directory:

```bash
mkdir -p _test_samples
```

2. **Add ICC profile files** from trusted sources:
   - **sRGB**: `sRGB_v4_ICC_preference.icc` (or similar standard sRGB profile)
     - Source: Windows/macOS system profiles, or Adobe RGB profile
   - **Monitor**: `asus_rog_strix_xg309cm.icm`
     - Source: Monitor manufacturer profile
   - **Reference**: `Spec400_10_700-IllumA-Abs_2deg.icc`, `sample1.icc`, `sample2.icc`
     - Source: Color reference database or spectrophotometer software

3. **Run all tests**:

```bash
cargo test
```

### Test Coverage

- Gray/RGB matrix/TRC, D50 PCS, XYZ/Lab, and chromatic adaptation
- `curv`, parametric curves 0-4, LUT8/LUT16, mAB/mBA, and A2B/B2A intents
- 1D and tetrahedral interpolation, endpoints, malformed profiles, and limits

### Supported Profile Formats

- Display profiles (RGB, Monitor)
- Color space profiles (Lab)
- LUT types: Lut8, Lut16, LutAtoB, LutBtoA
- ICC v2.0 - v4.2

## Development

Trusted sources for ICC profiles:
- [Adobe Profiles](https://adobe.com) - sRGB, ColorMatch RGB
- [ColorThink](https://www.colorlogic.com) - Reference profiles
- [ICC Repository](https://www.color.org) - Standards and examples
- Monitor/Printer manufacturer websites

⚠️ **Do not use profiles from unknown or untrusted sources**

## Release boundary

This release is limited to explicit Gray/RGB transforms. Unsupported color
spaces and processing elements fail closed rather than being approximated.
## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

### Transactional output

Use `Transform::transform_f32_atomic`, `transform_u8_atomic`, or
`transform_u16_atomic` (also on `TransformWorker`) when the caller's output
must remain unchanged on error. Existing f32 execution stays one-pass and may
partially update on a late error; existing integer execution is already
transactional. The f32 atomic variant uses two evaluation passes and fixed
768-byte chunk scratch, not an image-sized temporary. Single-pixel
`CompiledProfile::transform_f32_atomic` uses one evaluation and 12-byte scratch.
These are output transactions, not synchronization or panic rollback.
See [the execution contract](docs/cms-0.1.0-plan.md#borrowed-output-transaction-contract)
for memory, compatibility and error guarantees.

## Initial CMYK display route

The checked transform API now accepts v2 CMYK `mft2` and v4 CMYK `mAB` A2B stages for relative
colorimetric display conversion to RGB (or a standalone DeviceToPcs stage).
Samples are C/M/Y/K, 0 = no ink and 1 = maximum ink; u8/u16 scale to that range.
Format-specific inversion and YCCK belong to callers. PCS remains D50 XYZ/Lab.
CMYK output, other intents and other CMYK tag types remain unsupported.

The 4D CLUT uses all 16 hypercube corners with multilinear interpolation.
Existing 1D/3D interpolation is unchanged. CMYK mft2 matrices must be identity.
Integer execution retains two 192-float buffers, using 48-pixel chunks for CMYK
instead of 64. Borrowed f32 atomic output scratch remains 192 floats; device
normalization/evaluator scratch grows to four components. Alpha is not added.

## Explicit numerical algorithm profiles

`TransformOptions` keeps its existing fields and struct-literal compatibility.
Use `AlgorithmProfile` separately to select implemented numerical methods as a
bundle, then apply it with `Transform::with_algorithm_profile` or
`CompiledProfile::with_algorithm_profile`. These consuming methods do not copy
compiled tables. Existing clones/workers remain unchanged; newly created
workers inherit the selected profile. Ordinary, integer, Vec and atomic paths
all use the same deterministic choices. Getters expose the effective profile.

```rust
use icc_profile::{AlgorithmProfile, ClutInterpolation3D, ClutInterpolation4D};
let algorithms = AlgorithmProfile::default()
    .with_clut_3d(ClutInterpolation3D::Trilinear)
    .with_clut_4d(ClutInterpolation4D::Multilinear);
// let transform = transform.with_algorithm_profile(algorithms);
```

Default is unchanged: 3D tetrahedral and 4D multilinear. 3D trilinear is the
additional implemented option; 4D currently has only multilinear. Settings
apply only to the matching CLUT dimensionality, not matrix or 1D paths. There
are no placeholder CMS modes or accepted-but-unimplemented algorithms. Future
presets can bundle additional validated choices without claiming exact CMS
compatibility. Intent, clipping, BPC, and atomicity are independent contracts.
