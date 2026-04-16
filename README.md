# ICC Profile Reader

## Example

```rust
pub fn main() -> std::io::Result<()> {
    let mut is_fast = true;
    for argument in env::args() {
        if is_fast {
            is_fast = false;
            continue
        }
        println!("{}",argument);
        let icc_profile = icc_profile::utils::load(argument)?;
        let decoded = DecodedICCProfile::new(&icc_profile.data)?;
        println!("{}",decoded_print(&decoded, 0)?);
    }
    Ok(())
}
```

## Testing

The project includes comprehensive integration tests for ICC profile handling and color space conversions.

### Running Unit Tests (No Samples Required)

```bash
cargo test --lib
```

This runs 14 unit tests covering color space math and gamma curves with no external dependencies.

### Running Full Test Suite (Requires ICC Profiles)

For the 96 integration tests that use ICC profile samples:

1. **Prepare test samples** by creating `_test_samples/` directory:

```bash
mkdir -p _test_samples
```

2. **Add ICC profile files** from trusted sources:
   - **sRGB**: `sRGB_v4_ICC_preference.icc` (or similar standard sRGB profile)
     - Source: Windows/macOS system profiles, or Adobe RGB profile
   - **Printer (CMYK)**: `JapanColor2011Coated.icc`, `ycck.icc`
     - Source: Printing vendor profiles (Agfa, EFI, Heidelberg, etc.)
   - **Monitor**: `asus_rog_strix_xg309cm.icm`
     - Source: Monitor manufacturer profile
   - **Reference**: `Spec400_10_700-IllumA-Abs_2deg.icc`, `sample1.icc`, `sample2.icc`
     - Source: Color reference database or spectrophotometer software

3. **Run all tests**:

```bash
cargo test
```

### Test Coverage

- **Unit tests** (14): Color space conversions, gamma curves, white point math
- **Integration tests** (96): 
  - ICC profile loading and parsing
  - CMYK→RGB pipeline transformations
  - Color difference metrics (ΔE76, CIEDE2000)
  - LUT8/LUT16 profile handling
  - Pipeline roundtrip validation

**Total: 110 tests** ✅

### Supported Profile Formats

- Display profiles (RGB, Monitor)
- Printer profiles (CMYK, YCCK)
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

## Todo

- ICC Profile 4.x, 5.x tags full support
- RGB→CMYK inverse pipeline (complete LUT inversion)
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
