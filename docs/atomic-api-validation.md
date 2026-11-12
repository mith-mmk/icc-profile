# Atomic API validation, 2026-11-12

Base implementation: `3bc2d5dac41c925ed0c72774671876be9de74b8e`.
No crate dependency or production algorithm changes were made for this validation.

## Official external profile

- Source and terms: https://registry.color.org/rgb-registry/srgbprofiles
- Download: https://registry.color.org/rgb-registry/profiles/sRGB_v4_ICC_preference.icc
- SHA256: `83174717332326ddc198d9df188a4daec27b8979ba152cebbfc470c793d0bb11`.
- Terms checked before use: no warranty, unchanged file including copyright
  tag, no use of ICC name for advertising without permission. Used unchanged
  for local testing only; not committed or redistributed.
- Seven `icc_profile_load` sRGB-v4 tests passed.
- The opt-in official mAB test passed for perceptual and relative colorimetric
  intents. It now also checks f32/u8/u16 atomic results against ordinary APIs,
  worker consistency, chunk boundaries and whole-output preservation on late
  invalid input, using the shared contract harness.
- This establishes execution/transaction consistency, not independent color
  accuracy. The existing bundled LittleCMS oracle remains a separate test.

Reproduce with `ICC_V4_MAB_TEST_PROFILE` pointing to the original file:

```text
cargo test --test lut_synthetic official_v4_mab_fixture_compiles_without_nonfinite_output -- --ignored
cargo test --test icc_profile_load srgb_v4
```

Other external samples (manufacturer/display, CMYK, spectral and anonymous
sample profiles) remain unavailable and were not tested. Their download and
redistribution terms have not been established. They are not replaced with
unrelated profiles or treated as passing.

## Paired release-mode timing

Windows x86_64, AMD Ryzen 7 7700, rustc 1.99.0 (b940084d7), LLVM 23.1.1.
65,536 pixels per buffer; four conversions per timing sample; nine samples
per method; alternating method order after warmup; one test thread.
Construction and input/output allocation are outside the timed region.
Constant input 0.4, synthetic gamma-2.2 Gray/RGB matrix profiles and RGB mft2
identity LUT pair. These are warm steady-state measurements, not diverse-image
benchmarks. CPU scheduling, power state and frequency were not pinned.

| Route | Ordinary f32 ns/pixel min / median / max | Atomic min / median / max | Median ratio |
| --- | --- | --- | --- |
| Gray matrix | 30.45 / 30.82 / 51.41 | 61.31 / 62.29 / 78.68 | 2.021 |
| RGB matrix | 62.54 / 65.29 / 74.64 | 125.09 / 126.22 / 145.54 | 1.933 |
| RGB mft2 pair | 141.60 / 144.47 / 161.51 | 284.31 / 287.78 / 298.44 | 1.992 |

```text
cargo test --release --test transform --test lut_synthetic benchmark_atomic -- --ignored --nocapture --test-threads=1
```

The roughly twofold cost matches the two-evaluation design. These numbers do
not promise a platform-independent speed ratio. Integer atomic APIs directly
call the existing integer methods; no new algorithm or extra pass is inserted.
The benchmark is ignored in normal CI and imposes no timing threshold.

## Remaining decisions

Before 0.1 stabilization, reconcile the historical scope checklist with the
current implementation and explicitly decide the supported color-space/intent
matrix and legacy API lifecycle. Atomic output consistency alone does not add
CMYK/N-color, MPE or black-point compensation support. Publication, merge and
PR creation are not part of this validation.
