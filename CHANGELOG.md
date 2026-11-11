# Changelog

## 0.0.8 - 2026-11-11

### Dependencies

- Update the private `bin-rs` dependency from 0.0.10 to 0.1.0, retaining its
  `util` feature. No downstream API migration is required for this update.

### Fixed

- Return errors before legacy ASCII reads on truncated `sig ` / `desc` tags,
  undersized declared tag lengths, and short `data` headers. Check the actual
  shifted range in the legacy extended-description branch before reading it.
  These malformed inputs could panic after the dependency range correction.
- Preserve successful decoding, including existing description offsets and
  version selection. This is not a comprehensive legacy-parser safety audit.

### Compatibility

- Public APIs are unchanged; no ICC specification or transform algorithm is
  redesigned. Narrow legacy input guards accompany the dependency update.
- The dependency update includes the earlier `bin-rs` ASCII range correction:
  legacy tag text now honors the requested offset and length instead of reading
  the entire buffer. Legacy text output can therefore differ from 0.0.7.
- Legacy profile IDs also inherit the corrected 128-bit read: the low 64 bits
  now come from the second half of the ID rather than repeating the first half.
- The ICC specification and API are still evolving. This dependency migration
  does not declare ICC API stability or establish an ICC minimum Rust version.

## 0.0.7 - 2026-10-18

### Fixed

- Connect a source LUT to the destination LUT's physical PCS before encoding
  its input. Lab-PCS destinations previously received XYZ numbers as Lab,
  producing severely incorrect colors. Matrix destinations retain XYZ.
- Preserve the existing single XYZ media-white bridge for absolute intent.

### Tests

- Add matrix/modern-LUT/legacy-LUT XYZ/Lab reference combinations, including
  Gray matrix controls, relative and absolute intents.
- Add a frozen LittleCMS-generated Lab-PCS profile and recorded independent
  references for u8, u16 and f32. LittleCMS is optional fixture-generation
  tooling only; no runtime, build or dev dependency is added.

### Compatibility

- Public APIs and dependencies are unchanged. CMYK, N-color, MPE and BPC
  remain outside the checked transform capability.
- Legacy external-profile tests still require the documented local sample
  files; these samples are not bundled or silently substituted.

## 0.0.6 - 2026-09-05

### Maintenance

- Integrate the Gray/RGB CMS implementation history into `main`.
- Refresh release documentation after the 0.0.5 publication and prepare the
  patch release for WML2 integration.
- Preserve the 0.0.5 public APIs and transform behavior; this release does
  not change the color conversion algorithms.
- Replace a redundant unsigned `<= 0` check with `== 0` so current Clippy
  accepts the legacy profile reader without changing its behavior.

## 0.0.5 - 2026-08-31

### Added

- Checked Gray/RGB ICC v2/v4 profile and transform APIs.
- D50 PCS XYZ/Lab conversion, chromatic adaptation, TRCs, and parametric
  curves 0-4.
- A2B/B2A intent selection with mft1, mft2, mAB, and mBA processing stages.
- 1D and tetrahedral 3D interpolation with explicit transform resource
  limits.
- `transform_f32` as the processing core plus U8/U16 quantization wrappers.

### Compatibility

- Unsupported CMYK, N-color, MPE, and black-point-compensation routes return
  explicit errors instead of using an approximate fallback.
- Legacy profile inspection and CMS modules remain available.
