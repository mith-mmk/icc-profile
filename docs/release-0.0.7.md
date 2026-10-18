# icc-profile 0.0.7 release

This patch release contains the independently reviewed Lab PCS LUT connection
fix from `897f27d6f5383921b7bdccd36ca7aac72031bf18`, plus version and release
documentation changes. The correction passes the physical PCS expected by a
destination LUT while retaining XYZ for matrix destinations. It does not add
color models, change public APIs or dependencies, or enable implicit CMS in
downstream decoders.

## Validation boundary

- The fixture-independent library and integration selection covers 161 tests,
  with two existing ignored tests. It includes an independent analytical
  matrix of 200 source/destination PCS cases, Gray matrix controls, malformed
  input/resource controls, and a frozen real-profile oracle at u8/u16/f32.
- The retained Lab-PCS profile is a synthetic calibration serialized by
  LittleCMS 2.16, SHA256
  `1b7dea170895aacd22a8c76344b06c54ccf18882a573698bb8e8a07f6cfec2b4`.
  Reference generation and tolerances are documented in
  `tests/fixtures/README.md`. Ordinary tests do not depend on LittleCMS.
- Local WML2 linkage through a scratch Cargo path patch changes the failing
  mid-gray result from `[458,182,294]` to `[32744,32772,32778]`; the independent
  LittleCMS reference is `[32753,32772,32778]`. Existing WML2 highres controls
  pass without a downstream dependency change.
- The full legacy CMS suite is not recorded as passing: 23 tests require four
  missing external ICC files. Full profile-load coverage additionally needs
  the documented reference/generic samples. No replacement files are invented.
- Strict Clippy reports pre-existing legacy style debt (208 diagnostics matching
  the baseline); full-tree formatting also has existing unrelated differences.
  Changed Rust files and staged diffs are checked separately.
- This repository has no GitHub Actions workflow or required CI check at the
  release baseline. Local release checks are therefore explicit gates.

## Publication procedure

Use a clean release commit containing the reviewed correction. Run the
available fixture-independent tests and doc tests, then `cargo package --locked`
and `cargo publish --dry-run --locked --registry crates-io`. Preserve the legacy
sample and style-check qualifications above.

Integrate into main using a normal fast-forward push, consistent with the
previous release. Respect any server-side branch protections; do not force push
or use administrative bypass. Use an annotated `0.0.7` tag, matching the
repository's existing unprefixed version-tag convention, and verify its remote
commit before publication. Publish with Cargo's existing standard credentials,
without printing or configuring authentication secrets, then create the GitHub
Release for the verified tag.

Verify the registry version/checksum and downloaded crate against the local
package, `.cargo_vcs_info.json`, original packaged source and tag commit.

WML2 and JPEG XL dependency changes, merges and publications are outside this
release. Registry consumers need a separately requested downstream update or
resolution before they use the corrected library.
