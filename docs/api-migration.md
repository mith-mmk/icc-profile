# Additive API migration

The migration separates two existing use cases without removing old entrypoints.
`simple` is profile-free math with explicit units; `icc` is an additive namespace
for the existing checked ICC engine. `icc::Profile` and root `Profile` are the
same type, not conversion wrappers. Legacy root/module paths and signatures
remain supported. Migration is recommended only; no deprecation attributes or
silent behavioral substitution are introduced.

## Practical old and new calls

For a full-range BT.601 byte scalar, the existing call remains valid:

```rust
use icc_profile::cms::transration::rgb_to_yuv;
let old = rgb_to_yuv(255, 0, 0);
use icc_profile::simple::{rgb8_to_ycbcr8_full, YcbcrCoefficients};
let new = rgb8_to_ycbcr8_full([255, 0, 0], YcbcrCoefficients::Bt601);
assert_eq!([old.0, old.1, old.2], new);
```

The new buffer APIs infer their pixel count from complete packed triples, have
no alpha channel, and consume the entire slice. Existing `entries` APIs retain
explicit counts, their individual RGB/RGBA layouts and accepted surplus tails.
Do not rename a call without first adapting its layout and sample encoding.

For real ICC processing, keep root imports or switch to `icc`:

```rust
use icc_profile::icc::{Profile, Transform, TransformError, TransformOptions};
fn convert(source: &[u8], destination: &[u8], input: &[f32])
    -> Result<Vec<f32>, TransformError>
{
    let source = Profile::parse(source)?;
    let destination = Profile::parse(destination)?;
    Transform::new(&source, &destination, TransformOptions::default())?
        .transform_f32_vec(input)
}
```

`Profile::parse` checks the container without compiling every direction.
`Profile::new` and `from_bytes` keep their existing eager semantic validation;
changing to `parse` intentionally moves unsupported-stage errors to compilation.
For example, `new` / `from_bytes` can reject a constant TRC that is usable in the
forward direction but cannot be inverted; `parse` allows selecting and compiling
only the supported direction. Parsing is not a complete semantic validator.
The legacy `DecodedICCProfile` is not a checked `Profile`, so migrate from bytes
rather than treating those representations as interchangeable.

## Migration coverage

| Existing category | Recommended destination or explicit retained scope |
| --- | --- |
| Checked `Profile`, `Transform`, worker, limits, route/BPC diagnostics | Same types under `icc`; old root and `transform` paths remain. |
| RGB8 ↔ full-range BT.601/BT.709 YUV/YCbCr scalars | `simple::rgb8_to_ycbcr8_full` / `ycbcr8_full_to_rgb8`, explicit `YcbcrCoefficients`. |
| Corresponding packed RGB/YUV entry buffers | New `*_buffer` functions consume complete triples; adapt count, alpha and padding explicitly. |
| PAL or arbitrary custom YUV matrices/coefficient objects | Keep the legacy coefficient APIs; not claimed as implemented by the new two-choice enum. |
| Floating linear RGB ↔ XYZ, explicit matrix | `simple::linear_rgb_f64_to_xyz_f64` / `xyz_f64_to_linear_rgb_f64` and `*_buffer`; caller supplies the correct matrix direction. |
| Matrix presets, construction, evaluation, inverse | Reachable through the same `simple::ColorMatrix3D` type; existing constructors/inverse errors retain their contract. |
| Floating XYZ ↔ Lab, explicit reference white | `simple::xyz_f64_to_lab_f64` / `lab_f64_to_xyz_f64` and `*_buffer`, with `simple::WhitePoint`. |
| Whitepoint presets and legacy profile-derived whitepoint helpers | Presets are reachable through `simple::WhitePoint`; profile-derived legacy behavior remains a compatibility helper, not a full ICC route. |
| Old byte XYZ/Lab encodings and byte-to-float variants | Keep legacy entrypoints and exact documented quantization/scales; not renamed into physical XYZ/Lab APIs. |
| RGB/RGBA helper buffers and output alpha synthesis | Keep legacy layouts, or strip/recombine alpha explicitly around new packed triples; new APIs do not silently synthesize alpha. |
| Legacy `transration_prametic_curve` gamma helper | Keep legacy raw-byte-domain behavior (`0..255`, not normalized transfer curves); unknown types pass through. No ambiguous new `gamma` API. |
| All eight raw LUT8/LUT16 scalar shapes (3→1, 3→3, 3→4, 4→3 for each format) | Existing additive `try_` helpers in `cms::transration`; see checked LUT map below. Not a new profile-free engine. |
| CMYK ↔ Lab and CMYK/RGB scalar LUT helpers | Existing matching `try_` helpers for checked payload errors; original panic-oriented scalar signatures remain. |
| Profile-based CMYK/RGB approximation helpers | Existing `try_` variants only as limited compatibility helpers; use `icc::Profile` / `Transform` for profile-defined conversion and route validation. |
| Inverted YMCK conversions | Keep explicitly legacy behavior; do not substitute standard increasing-ink CMYK or rename as generic CMYK. |
| ICC decoding, record/tag inspection, fixed-point formatting | Existing parser/utility APIs and bounded inspection helpers; use checked `Profile::parse` for new transform workflows. |
| `ColorEntries::to_rgb`, `to_lab`, `to_lab24`, `to_lab48`, `to_xyz`, `to_xyz24`, `to_ymck` | All seven remain unimplemented (`None`); no automatic migration exists. Select and call a concrete supported operation instead. |

The table intentionally distinguishes implemented migration destinations from
retained compatibility-only functions. Scope outside the new core is not
silently approximated or presented as newly implemented.

## Detailed legacy contracts

- [Checked raw LUT helpers, exact try_ names and route limits](legacy-lut-checked.md)
- [Limited profile-helper behavior and migration](legacy-profile-helpers.md)
- [Standalone helper units, layouts, matrices, gamma and whitepoints](standalone-color-contracts.md)
- [Scalar Lab and whitepoint corrections](scalar-lab-whitepoint-corrections.md)
- [Fixed-point values](legacy-fixed-point.md)
- [Tag parsing](legacy-tag-parser.md)
- [Bounded inspection](bounded-inspection.md)

## Errors, algorithms and resource contracts

`SimpleError` is a new non-exhaustive error type. It reports complete-pixel
length errors, invalid/nonfinite inputs, invalid matrix/whitepoint parameters,
nonfinite results and fallible allocation errors. New simple buffers return no
partial result. Reexported `ColorMatrix3D` and `WhitePoint` retain their legacy methods, including
unchecked methods; checked-input guarantees describe the new named conversion
functions, not every method on those utility types.
Floating operations do not clip nominal ranges or add transfer
curves; caller-selected XYZ scales and matrix/whitepoint compatibility matter.

The existing exhaustive `TransformError` enum is unchanged. In particular,
malformed input does not always yield `MalformedProfile`: existing validators
also use `InvalidProfile` or other applicable variants. This migration does not
reclassify old errors or stabilize diagnostic text as an error taxonomy. A valid
profile can also yield `MalformedProfile` when a device sample is outside its
allowed domain with `clamp=false`; the variant alone does not prove corruption.

`AlgorithmProfile` chooses built-in methods in the same checked ICC engine.
It is not a plugin/callback API. ICC tag fallback, reported by `RouteInfo`, is
separate from cross-layer fallback: no ICC error invokes `simple` implicitly.

See the README for device versus physical PCS units, exact buffer/alpha rules,
`clamp=false` limitations, output transactions and their cost, the default ICC
64 MiB Vec cap, version boundaries and the BPC route support table.
