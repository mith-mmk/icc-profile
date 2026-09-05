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
- [x] `cargo package --locked` on a clean release commit.
- [x] `cargo publish --dry-run --locked` on a clean release commit.
- [x] crates.io publication and registry verification.
- [x] WML2 updated to the published version and integration tests passed.

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

## Publication and downstream verification

- Release commit: `bd5a65e19b26826aa5f031cd6a354e5d38e77de9`, pushed to
  `origin/main` before publication. Package and dry-run used a clean detached
  checkout of this exact commit, excluding the local uncommitted .gitignore.
- `cargo publish --locked --registry crates-io` published 0.0.6;
  `cargo info icc-profile@0.0.6 --registry crates-io` downloaded and confirmed it.
- Package: 67 files, 572.6 KiB before compression. Registry checksum:
  `fb960e791dd103a3e696377c2b1e68da54215d68e43698960217666f8a53ff87`.
- WML2 and wml2-test now request registry 0.0.6. Workspace tests: 136 passed;
  combined AVIF/ICC/PNG/PSD/TIFF/JPEG tests: 103 passed; Rust 1.91 ICC tests:
  38 passed. All had zero failures. Workspace examples, WML2 Clippy, Wasm
  and 32-bit ICC checks passed. WML2 itself was not published in this step.
