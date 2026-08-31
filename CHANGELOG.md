# Changelog

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
