# icc-profile

Pure Rust ICC profile parsing and explicit Gray/RGB color transforms.

The 0.0.5 release candidate provides a checked ICC v2/v4 CMS for Gray and
RGB profiles. The processing core is `f32` and supports D50 PCS, XYZ/Lab,
chromatic adaptation (`chad`), Gray TRC, RGB matrix/TRC, `curv`, parametric
curves 0-4, `mft1`, `mft2`, `mAB`, `mBA`, A2B/B2A intent selection, optional
stages, 1D interpolation, and tetrahedral 3D interpolation.

CMYK, N-color, MPE, black-point compensation, and other unsupported routes
return `UnsupportedProfileFeature`; there is no simplified color fallback.
Alpha is not passed to the CMS and must be handled by the caller.

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
