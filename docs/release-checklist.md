# icc-profile 0.0.5 release checklist

This is the pre-publication checklist for the narrowed Gray/RGB CMS release.
It does not publish, tag, or merge branches.

- [x] ICC v2/v4 checked profile parsing with offset, length, tag, curve, and
      CLUT limits.
- [x] Gray/RGB matrix/TRC, D50 PCS, XYZ/Lab, chad, curv, and parametric curves
      0-4.
- [x] mft1, mft2, mAB, mBA, A2B/B2A, four rendering intents, optional stages,
      1D interpolation, and tetrahedral interpolation.
- [x] `transform_f32` core with U8/U16 quantization wrappers.
- [x] Explicit unsupported errors for CMYK, N-color, MPE, and BPC paths.
- [x] Synthetic parser, transform, LUT, interpolation, limit, and malformed
      input tests.
- [x] Package metadata, README, changelog, license, and documentation are
      independent of machine-local fixtures.
- [ ] `cargo package --locked` on a clean release commit.
- [ ] `cargo publish --dry-run --locked` on a clean release commit.
- [ ] Actual tag, publication, registry verification, and GitHub release.

Internal allocation accounting is limited to checked practical limits; a
byte-accurate peak-live-allocation proof is not an acceptance condition.
