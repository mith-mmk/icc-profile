# icc-profile

Pure Rust ICC profile parsing and explicit Gray/RGB and limited CMYK color transforms.

The checked engine provides a checked ICC v2/v4 CMS for Gray and
RGB profiles. The processing core is `f32` and supports D50 PCS, XYZ/Lab,
chromatic adaptation (`chad`), Gray TRC, RGB matrix/TRC, `curv`, parametric
curves 0-4, `mft1`, `mft2`, `mAB`, `mBA`, A2B/B2A intent selection, optional
stages, 1D interpolation, and tetrahedral 3D interpolation.

Version 0.0.7 corrects the physical PCS connection between paired LUT stages
when the destination uses Lab PCS. Matrix destinations continue to use XYZ.

The development branch adds explicitly bounded CMYK output and relative matrix
black-point compensation, described below. N-color, MPE and other unsupported
routes return `UnsupportedProfileFeature`; there is no simplified color fallback.
Alpha is not passed to the CMS and must be handled by the caller.

## Choose the API layer

- `simple`: profile-free, explicitly named color math with checked inputs and
  strict packed-buffer lengths. Reexported matrix/whitepoint types retain their
  legacy methods, including unchecked ones. Use the named conversion functions when you already know the matrix,
  reference white or YCbCr coefficients. It does not read ICC profiles.
- `icc`: the existing checked `Profile` / `Transform` engine, reexported without
  wrappers. Use it when the profile defines color behavior. ICC failures are
  returned to the caller, never converted through `simple` implicitly.
- Existing root exports, `transform`, `cms::transration`, `iccprofile` and `utils`
  stay available with their names, signatures and compatibility contracts.
  Migration is recommended, not enforced: no `#[deprecated]` attributes or
  removal are introduced. See [the migration map](docs/api-migration.md).

This integration targets the `0.1.0` development line. `Cargo.toml` still declares
package version `0.0.8`; the new namespaces described here are development
changes, not a claim that a new crates.io release has been published.

## Interpolation and LittleCMS compatibility

The 3D CLUT evaluator defaults to tetrahedral interpolation; AlgorithmProfile can select trilinear interpolation. LittleCMS 2.16
[selects trilinear interpolation for Lab-PCS output (B2A) LUTs](https://github.com/mm2/Little-CMS/blob/lcms2.16/src/cmsio1.c#L621-L624),
so matching profiles and rendering intents need not produce identical values.

In a 10,010-color sRGB-to-synthetic-Lab-PCS RGB LUT comparison, the maximum
RGB8 channel difference was 18/255. Evaluating the same LUT with trilinear
interpolation reduced the observed difference to at most 1/255. The comparison
used relative colorimetric intent, BPC off, and LittleCMS
`NOOPTIMIZE | NOCACHE`; these figures are observations, not general error bounds.

Neither method is universally more accurate; results depend on the profile
and samples. This comparison does not establish an ICC specification violation,
and does not establish universal numerical agreement. Recorded manufacturer
CMYK output measurements are limited to their stated profiles, intents and
algorithms: see [CMYK output](docs/cmyk-output.md),
[absolute output](docs/cmyk-output-absolute.md), and
[output BPC](docs/cmyk-output-bpc.md). The default interpolation behavior is unchanged.

## Checked ICC quickstart

```rust
use icc_profile::icc::{Profile, RenderingIntent, Transform, TransformOptions};

fn convert(source_icc: &[u8], destination_icc: &[u8], input: &[f32])
    -> Result<Vec<f32>, Box<dyn std::error::Error>>
{
    let source = Profile::parse(source_icc)?;
    let destination = Profile::parse(destination_icc)?;
    let transform = Transform::new(
        &source,
        &destination,
        TransformOptions {
            rendering_intent: RenderingIntent::RelativeColorimetric,
            ..TransformOptions::default()
        },
    )?;
    // Input and output channel counts can differ (for example RGB to CMYK).
    // The allocating helper validates complete pixels and sizes output correctly.
    Ok(transform.transform_f32_vec(input)?)
}
```

## Profile-free quickstart

```rust
use icc_profile::simple::{
    rgb8_to_ycbcr8_full_buffer, xyz_f64_to_lab_f64, WhitePoint, YcbcrCoefficients,
};

let ycbcr = rgb8_to_ycbcr8_full_buffer(&[255, 0, 0], YcbcrCoefficients::Bt601)?;
assert_eq!(ycbcr.len(), 3); // packed Y, Cb, Cr; no alpha
let white = WhitePoint::d50();
let lab = xyz_f64_to_lab_f64([white.x, white.y, white.z], &white)?;
assert!((lab[0] - 100.0).abs() < 1e-9);
# Ok::<(), icc_profile::simple::SimpleError>(())
```

Byte YCbCr is full range with neutral chroma 128; BT.601/BT.709 select
coefficients, not transfer functions or studio-range video. Floating conversions
are finite, unclipped `f64`: RGB must already be linear, XYZ uses the scale of
the supplied matrix/white (normally Y=1), and Lab uses L* and signed a*/b*.
There is no implicit adaptation, transfer curve, matrix inversion or alpha.
New buffers consume **all** samples as packed triples and reject incomplete
pixels, unlike legacy `entries` functions that may accept a surplus tail or use
RGBA layouts. Their Vec allocation is fallible; they do not use the ICC Vec cap.
See [profile-free examples and compatibility](docs/simple-conversions.md).

## ICC execution contract

`Profile::parse` validates and owns the container bytes; direction-specific
compilation validates the selected stages. A parsed profile need not support
both directions. Existing `Profile::new` / `from_bytes` retain their earlier
semantic validation behavior.

- Pair transforms use packed/interleaved Gray, RGB or CMYK samples, no alpha:
  `f32` device values are `0..=1`; `u8` and `u16` represent their full code ranges.
  CMYK is increasing ink, not JPEG inverted CMYK or YCCK.
- A standalone `CompiledProfile` exchanges **physical D50 XYZ** on the PCS
  side, including when the ICC payload uses Lab. XYZ is not normalized LUT
  encoding and is not restricted to `0..=1`.
- `clamp=false` does not promise HDR or arbitrary out-of-range processing.
  Curve inverse domains and LUT/route restrictions still apply.
- Borrowed ordinary `f32` output can be partly written before a late error.
  Choose the atomic methods for unchanged output on error (cost below). Input
  must contain whole pixels and output length must match the route exactly.
- Vec helpers default to a **64 MiB output allocation cap**. Select an explicit
  `ExecutionLimits` bound when necessary; parser and compiler limits are separate.
- `RouteInfo::used_fallback` reports legal ICC tag/model selection within the
  checked engine. It never means falling back to profile-free color math.
- `TransformError` remains source-compatible and exhaustive. Malformed inputs
  are not uniformly classified as `MalformedProfile`; some established checks
  return `InvalidProfile` or another applicable error. Even a valid profile can
  produce `MalformedProfile` for an out-of-range device sample with `clamp=false`;
  do not infer profile corruption from that variant or parse message text.
- `AlgorithmProfile` selects implemented numerical algorithms only, not arbitrary
  external engines or callbacks. `ColorEntries::to_xyz` / `to_lab` remain
  unimplemented legacy entrypoints and are not migration destinations.

### Black-point compensation route support

| Pair route | Accepted BPC subset |
| --- | --- |
| RGB matrix/TRC → RGB matrix/TRC | Relative intent; XYZ PCS and encoded D50 headers; both selected forward routes must be matrix. |
| v4 printer CMYK → RGB matrix/TRC | Relative intent; explicit source A2B1 + same-profile B2A0; XYZ matrix destination and D50 headers. |
| RGB matrix/TRC → v4 printer CMYK | Relative intent; explicit destination B2A1 + A2B1 + B2A0; XYZ matrix source and D50 headers. |
| Gray, general LUT/RGB, v2 CMYK, CMYK → CMYK, non-relative intent | Not supported with BPC. |

Additional route/estimator validation can reject an otherwise readable profile.
CMYK BPC does not accept selected-tag fallback; invalid/degenerate estimators
fail explicitly. See [matrix BPC](docs/relative-matrix-bpc.md),
[CMYK input BPC](docs/cmyk-input-bpc.md) and
[CMYK output BPC](docs/cmyk-output-bpc.md). This is a bounded implementation,
not a claim of full ISO 18619 conformance.

## Testing

The project includes checked parser and transform tests for ICC profile
handling and color space conversions.

### Running Unit Tests (No Samples Required)

```bash
cargo test --all-targets
cargo test --doc
```

This runs the no-fixture unit and synthetic integration tests. External data tests are explicitly ignored, not silently treated as passed. See [CI and opt-in test instructions](docs/ci.md). Rust 1.91 is the supported minimum.

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

3. **Run a selected external test explicitly** (missing required files are errors):

```bash
cargo test --test icc_profile_load srgb_v4 -- --ignored
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

The 0.1.0 development branch adds perceptual, relative-colorimetric, and saturation CMYK to RGB display through v2 mft2 and v4 mAB source profiles. Published 0.0.8 remains unchanged. Unsupported color
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
`CompiledProfile::transform_f32_atomic` uses one evaluation and 16-byte scratch
(up to four output components).
These are output transactions, not synchronization or panic rollback.
See [the execution contract](docs/cms-0.1.0-plan.md#borrowed-output-transaction-contract)
for memory, compatibility and error guarantees.

## Initial CMYK display route

The checked transform API now accepts v2 CMYK `mft2` and v4 CMYK `mAB` A2B stages for perceptual, relative
colorimetric, and saturation display conversion to RGB (or a standalone DeviceToPcs stage).
Samples are C/M/Y/K, 0 = no ink and 1 = maximum ink; u8/u16 scale to that range.
Format-specific inversion and YCCK belong to callers. PCS remains D50 XYZ/Lab.
CMYK output is supported only by the bounded B2A routes described below; other
CMYK tag types remain unsupported. Absolute display is limited to the separately
documented printer-to-D50-matrix route.

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

## Command-line inspection and sample conversion

The standalone, non-published [icc-cli](tools/icc-cli/README.md) provides
`inspect`, `validate`, and numeric-sample `convert` commands through the checked
API. It does not decode images, modify profiles, or claim full ICC conformance.

```sh
cargo run --manifest-path tools/icc-cli/Cargo.toml -- --help
```

### CMYK rendering intents

CMYK input now selects A2B0 (perceptual), A2B1 (relative colorimetric), or
A2B2 (saturation) for the supported v2 mft2 / v4 mAB display paths. Missing
A2B1/A2B2 may use A2B0; RouteInfo reports this fallback. A malformed selected
tag is never replaced by another tag. This executes profile-provided tables,
not a newly invented gamut mapper or a guarantee of cross-CMS equivalence.
CMYK output and limited matrix, CMYK-input and CMYK-output BPC are described below. Absolute CMYK display has the narrower scope below.
Existing non-relative matrix-route media-white requirements remain in force.
See [intent validation](docs/cmyk-intents.md) for scope and black-box results.

### Absolute CMYK display

Absolute intent is supported only by a composed Transform from a CMYK printer
with explicit A2B1 (v2 mft2 or v4 mAB) to a D50-white RGB matrix/TRC monitor.
Both header illuminants must be encoded D50 and both media whites valid.
A2B0 fallback, LUT output profiles, non-D50 display whites, and standalone
CMYK absolute CompiledProfile are rejected. Adapted PCS/white values are not
passed through chad again. See [scope, specification and measurements](docs/cmyk-absolute.md).

### Bounded CMYK output and BPC

CMYK output uses explicit printer B2A tags, without numerical A2B inversion:
[relative RGB/PCS output](docs/cmyk-output.md),
[restricted absolute output](docs/cmyk-output-absolute.md),
[v4 perceptual LUT connection](docs/cmyk-output-perceptual.md) and
[v4 saturation LUT connection](docs/cmyk-output-saturation.md).
Each stage documents its source/version/intent restrictions and rejected paths.

The existing black_point_compensation option supports relative
RGB XYZ matrix-to-matrix routes with D50 headers and a verified matrix forward
route for the destination. [BPC scope and diagnostics](docs/relative-matrix-bpc.md)
describe endpoint estimates, memory budgets and black-box comparisons.
An additional [CMYK-input BPC stage](docs/cmyk-input-bpc.md) accepts only v4
printer CMYK with explicit A2B1 mAB and B2A0 mBA, to an RGB matrix destination.
It validates black estimates for both interpolation choices and accounts for
auxiliary compilation peak memory. Outside the bounded output extension below, v2 CMYK, general LUT and
non-relative BPC remain explicit errors; no full ISO conformance is claimed.


### Bounded CMYK output black-point compensation

Relative BPC also supports RGB XYZ matrix sources to v4 printer CMYK destinations
with explicit B2A1, A2B1 and B2A0 routes and D50 PCS headers. The destination
estimate uses a 256-sample neutral round trip, midrange test and quadratic shadow
fit from the public ICC-approved 2013 committee draft, not the media-black tag.
Invalid or degenerate ramps are explicitly unsupported in this bounded subset;
this is not a claim of final ISO 18619 conformance. Auxiliary LUTs and fixed ramp
storage count toward construction admission and are discarded after estimation.
`cmyk_output_bpc_diagnostics()` exposes the selected interpolation's immutable
estimate. See [CMYK output BPC](docs/cmyk-output-bpc.md) for scope and validation.

## Checked legacy LUT helpers

Legacy scalar LUT helpers have additive `try_` variants returning `std::io::Result`.
Use these for malformed-payload error handling; original scalar signatures retain
explicit panic behavior. See [migration, units, and route limits](docs/legacy-lut-checked.md).
## Standalone legacy color helpers

The profile-free helpers under `cms::transration` are separate from the checked
`Transform` API above. RGB/XYZ helpers are matrix-only linear-light operations,
not sRGB transfer functions or a full ICC pipeline. Their byte encodings, RGBA
layouts and compatibility limits are described in
[the standalone helper contracts](docs/standalone-color-contracts.md).
