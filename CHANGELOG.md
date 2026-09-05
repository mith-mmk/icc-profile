# Changelog

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
