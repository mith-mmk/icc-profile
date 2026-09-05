# icc-profile 0.0.6 release checklist

This patch release integrates the existing Gray/RGB CMS history into `main`
without changing the 0.0.5 transform algorithms or public APIs. Main
integration and crates.io publication were requested on 2026-09-05.

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
- [ ] crates.io publication and registry verification.
- [ ] WML2 updated to the published version and integration tests passed.

Git tags and a GitHub Release are outside this release request. The existing
`master` branch and repository default-branch setting are preserved; `main`
contains the fast-forward integration of `codex/icc-profile-highres`.

Internal allocation accounting is limited to checked practical limits; a
byte-accurate peak-live-allocation proof is not an acceptance condition.

## Local validation, 2026-09-05

- `cargo test --locked`: 219 passed, 0 failed, 10 ignored, including doc tests.
- Current Clippy's redundant unsigned-comparison error was repaired without
  changing behavior. Existing style warnings remain in legacy modules.
- Strict `cargo clippy -- -D warnings` and full-tree `cargo fmt --check`
  still report pre-existing style/format debt; neither is recorded as passing.
  Release source changes are limited to the equivalent comparison above.
