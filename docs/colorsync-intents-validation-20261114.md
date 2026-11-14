# ColorSync intent discrepancy: controlled reproduction

Baseline: `df5ff0dde65144104a5a469595e3e36c4bd24907`.
This investigation changes the validation harness and documentation, not the
library, version, APIs, unsupported-route policy or downstream dependencies.
No release or merge is performed.

## Finding

The previously reported **48/255** perceptual/saturation discrepancy was an
**intent-mismatched comparison through sips**, not evidence of a 48-code error
in the ICC transform. Direct ColorSync API conversion with matched intent and
BPC=false gives **maximum 2/255** on both 625 and 6561 CMYK sample grids.
No library correction is justified by these results.

Confirmed facts:

1. Fresh sips conversions reproduce all historical results, including identical
   outputs for its perceptual, relative, saturation and absolute command flags.
   The RGB hash is `3f003bb170bf302e175f5fd08c764dce57a511ea1728436f6221f638e2f584ca`,
   matching the old oracle. Perceptual/saturation versus ICC again gives max 48,
   mean 15.812267 codes, while relative gives max 1, mean 0.371200.
2. The source is v2.1 printer CMYK/Lab with header intent **1 (relative)**.
   Its A2B0 and A2B2 tags share offset 432, size 41478 and identical `mft2`
   content. A2B1 is different, at offset 41912. ICC's route diagnostics select
   A2B0/A2B1/A2B2 respectively, with no source fallback. The v2.1 sRGB monitor
   destination uses a matrix/TRC route, not an intent-specific B2A LUT.
3. Direct ColorSync selects the requested intent on *both* profile-sequence
   entries, explicitly disables BPC and uses BestQuality. Perceptual/saturation
   outputs now differ from relative and closely match ICC's same-intent output.
4. A header-only control changes source bytes 64..68 in temporary copies, leaving
   all tables unchanged. For **each** header value 0,1,2,3, all four sips command
   intents still produce identical outputs to one another. Changing the header
   from 1 to 0/2 changes output by up to 48; changing it to 3 changes up to 69.
   Header 0/2 output is within 3 codes of ICC perceptual/saturation; header 1
   within 1 of ICC relative; header 3 within 1 of direct ColorSync absolute.

Thus, **this sips pipeline's output follows the embedded source header intent,
not its command-line intent override**. That is the controlled observable cause
of this comparison mismatch. Whether the override is lost in ImageIO decoding,
sips processing, or another internal stage is **not established**. No private
Apple implementation was inspected. This is not a claim about every sips
operation, OS version or profile.

## Profiles and aligned settings

- SWOP source SHA-256:
  `4855b8fabb96bdc6495d45d089bb8c8efb1ae18389e0dc9e75a5f701a9c0b662`
- macOS `sRGB Profile.icc` SHA-256:
  `2b3aa1645779a9e634744faf9b01e9102b0c9b88fd6deced7934df86b949af7e`
- Same raw CMYK8 samples, C fastest then M/Y/K, to RGB8 without alpha.
- ICC: explicit intent, BPC=false, default interpolation, no profile mutation.
- ColorSync: public `ColorSyncTransformCreate` and `ColorSyncTransformConvert`,
  source `DeviceToPCS`, destination `PCSToDevice`, same explicit intent on both,
  `kColorSyncBlackPointCompensation=false` on both, `kColorSyncBestQuality`.
  System-default CMM on macOS 27.0.1 arm64; it is not separately version-pinned.
  ColorSync's internal interpolation/rounding is not assumed identical to ICC's.
- sips is isolated as a diagnostic image pipeline, not the raw API oracle.
  It does not expose the equivalent explicit BPC control in this command.

## Raw API results

Absolute per-channel difference in 8-bit codes, ICC versus explicit ColorSync
BPC=false. This is not a DeltaE criterion or universal acceptance threshold.

| Samples | Intent | Max | Mean | Channels above 2 |
| --- | --- | ---: | ---: | ---: |
| 625 | perceptual | 2 | 0.665067 | 0 |
| 625 | relative | 2 | 0.815467 | 0 |
| 625 | saturation | 2 | 0.665067 | 0 |
| 6561 | perceptual | 2 | 0.686938 | 0 |
| 6561 | relative | 2 | 0.818625 | 0 |
| 6561 | saturation | 2 | 0.686938 | 0 |

The remaining 2-code raw-API difference is measured, not diagnosed as a specific
rounding or interpolation bug. ICC absolute for this pair remains explicitly
unsupported; ColorSync absolute is diagnostic only and is **not** an ICC pass.

BPC controls matter: changing only ColorSync BPC false→true yields max 31 codes
for perceptual/saturation and 51 relative on 625 samples (39 and 53 on 6561).
Those runs are not mixed into the BPC=false accuracy comparison. Absolute output
is unchanged by this control.

For a concrete old outlier, CMYK `[63,255,0,255]` gave sips `[48,26,38]` versus
ICC perceptual `[0,0,0]`. This compared different source intent tables, rather
than measuring a faithful perceptual reference output.

## Reproduce and verify

Use [the opt-in harness](../tools/colorsync-compare/README.md). It generates all
inputs, embedded TIFFs and header controls, runs both raw APIs and sips, and
writes auditable statuses, hashes, routes, comparison statistics and worst
samples. The original profile files are never changed. Original/private profiles
are not vendored. Full structured results are in
[the JSON report](colorsync-intents-validation-20261114.json).

Checks on this host:

- C harness: `clang -Wall -Wextra -Werror`, passed.
- Standalone Rust harness: release build, passed; focused rustfmt check passed.
- Python full reproduction: both grids, BPC controls and all 16 header/command
  combinations completed; expected unsupported ICC absolute recorded.
- `cargo test --offline`: 365 passed, 0 failures, 70 ignored. Existing external
  fixture tests can self-skip; this is not exhaustive real-profile coverage.
- `git diff --check`: passed.

This does not replace cross-platform/MSRV/wasm checks, real-profile coverage,
release checklist review or a release decision. The engine has not been tuned
to match sips's header-dependent behavior. The original inverse-TRC optimization
and its compatibility contract remain unchanged.
