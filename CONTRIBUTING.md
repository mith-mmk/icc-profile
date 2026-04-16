# Contributing to ICC Profile Reader

Thank you for your interest in contributing! This document provides guidance on development and testing.

## Development Setup

### Prerequisites

- Rust 1.56+ (stable)
- Cargo
- git

### Building

```bash
git clone https://github.com/your-username/icc_profile.git
cd icc_profile
cargo build
```

### Testing

#### Unit Tests (No Samples Required)

```bash
cargo test --lib
```

Runs 14 unit tests for color space math without external ICC profiles.

#### Full Integration Tests

The project includes 96 integration tests using real ICC profiles. To enable them:

1. **Create test sample directory**:

```bash
mkdir -p _test_samples
```

2. **Add ICC profiles** from **trusted sources only**:

| File | Purpose | Suggested Source |
|------|---------|------------------|
| `sRGB_v4_ICC_preference.icc` | Display profile (RGB) | Adobe, Microsoft, or system profile |
| `JapanColor2011Coated.icc` | Printer profile (CMYK) | Japan Color Association official |
| `ycck.icc` | YCCK color space profile | Adobe or CMYK printer vendor |
| `asus_rog_strix_xg309cm.icm` | Monitor profile | ASUS ROG Strix monitor (or similar) |
| `Spec400_*.icc` | Reference spectral profile | ColorThink or ICC.org |
| `sample1.icc`, `sample2.icc` | Generic test profiles | Any standard RGB/CMYK profile |

**⚠️ Security Note**: Only use ICC profiles from:
- Official manufacturer websites
- Adobe/Microsoft system folders
- ISO Color standards organizations
- Reputable printing or color management vendors

**Never use profiles from unknown or suspicious sources**.

3. **Run full test suite**:

```bash
cargo test
```

### Recommended Profile Sources

**Free/Official:**
- https://www.adobe.com/ - sRGB, ColorMatch RGB
- https://www.color.org - ICC standards and examples
- https://www.colorlogic.com - ColorThink, reference data
- System profiles: Windows (`%windir%\System32\spool\drivers\color`), macOS (`/System/Library/ColorSync/Profiles`)

**Commercial:**
- Printing vendor profiles (EFI, Agfa, Heidelberg, GMG)
- Monitor manufacturer profiles (Dell, ASUS, BenQ)
- Color reference databases (X-Rite, Konica Minolta)

### Code Organization

```
src/
├── lib.rs                    # Public API
├── utils.rs                  # Utility functions (loading, printing, color diff)
├── iccprofile.rs             # ICC profile structures
└── cms/
    └── transration/          # Color space conversions
        ├── cmykrgb.rs        # CMYK ↔ RGB
        ├── cmyklab.rs        # CMYK ↔ Lab
        ├── xyzrgb.rs         # XYZ ↔ RGB
        └── ...

tests/
├── cms.rs                    # Integration tests: CMS pipelines
├── color_space.rs            # Color space math validation
└── icc_profile_load.rs       # Profile loading & parsing
```

### Testing Specific Modules

```bash
# Color space tests only
cargo test --test color_space

# CMS integration tests
cargo test --test cms

# Specific test
cargo test japan_color_cmyk_to_lab_via_lut8_white -- --nocapture
```

### Debugging Tests

```bash
# Show println! output
cargo test -- --nocapture

# Run single thread (useful for debugging)
cargo test -- --test-threads=1

# Backtrace on panic
RUST_BACKTRACE=1 cargo test
```

## Code Quality

### Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy -- -D warnings
```

## Testing Guidelines

When adding new tests:

1. **Use clear test names**: `test_cmyk_to_lab_via_lut16_white()` not `test_lut()`
2. **Add documentation**: Explain what is being tested and why
3. **Check bounds**: Lab values should be within [-127, +127], L* in [0, 100]
4. **Use assertions**: Provide helpful error messages

Example:

```rust
#[test]
fn japan_color_cmyk_to_lab_range_check() {
    let decoded = load_decoded("JapanColor2011Coated.icc");
    let tag = decoded.tags.get("A2B0").expect("A2B0 must exist");
    if let Data::Lut8(lut8) = tag {
        let (l, a, b) = cmyk_to_lab_lut8(100, 50, 30, 10, lut8);
        
        // Validate color space ranges
        assert!(l >= 0.0 && l <= 100.0, "L* {} out of range [0,100]", l);
        assert!(a >= -127.0 && a <= 127.0, "a* {} out of range [-127,127]", a);
        assert!(b >= -127.0 && b <= 127.0, "b* {} out of range [-127,127]", b);
    }
}
```

## Color Difference Validation

Use ΔE metrics for profile comparison:

```rust
use icc_profile::utils::{delta_e76, ciede2000};

let lab1 = (50.0, 10.0, -20.0);
let lab2 = (52.0, 9.0, -18.0);

let de76 = delta_e76(&lab1, &lab2);      // Simple Euclidean
let de2000 = ciede2000(&lab1, &lab2);    // ICC standard

assert!(de2000 < 2.0, "Color difference acceptable");
```

**Tolerance Guidelines:**
- **< 1.0**: Imperceptible
- **1-2**: Acceptable
- **2-4**: Noticeable  
- **> 4**: Unacceptable

## Submitting Changes

1. Fork and create a feature branch: `git checkout -b feature/my-feature`
2. Make changes and test thoroughly
3. Format code: `cargo fmt`
4. Run linter: `cargo clippy`
5. Run full test suite: `cargo test`
6. Commit with clear messages
7. Push and create a Pull Request

### PR Checklist

- [ ] Tests pass: `cargo test`
- [ ] Code formatted: `cargo fmt`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] Doc tests pass: `cargo test --doc`
- [ ] New tests added for features
- [ ] Changes documented in code
- [ ] No unsafe code unless necessary

## License

All contributions are licensed under Apache 2.0 or MIT license (dual-licensed). By contributing, you agree to these terms.

## Questions?

- Open an issue for bugs
- Discuss features in discussions
- See README.md for usage examples
