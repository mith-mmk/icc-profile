# Manufacturer/CMYK black-box comparison (2026-11-12)

## Corpus and permissions

Profiles remain outside Git in `_test_samples`. They are not modified, bundled
or redistributed. Only hashes, aggregate results and harness code are retained.

- X-Rite CGATS21_CRPC6, provided by Idealliance:
  https://registry.color.org/profile-registry/CGATS21_CRPC6
  Download: https://registry.color.org/profile-registry/profiles/CGATS21_CRPC6.icc
  Use/embed/exchange/share permitted; alteration or sale needs written permission.
- Heidelberg PSOcoated_v3, provided by ECI:
  https://registry.color.org/profile-registry/PSOcoated_v3
  Download ZIP: https://eci.org/lib/exe/pso-coated_v3.zip
  Use/embed/exchange permitted; distribution, sale or alteration require permission.
- ICC sRGB v4 preference and sRGB2014:
  https://registry.color.org/rgb-registry/srgbprofiles
  sRGB v4 terms require unmodified file/copyright and no ICC-name advertising;
  sRGB2014 uses ICC general profile terms, https://registry.color.org/profile-library/.
- Repository-owned synthetic Lab LUT fixture: `tests/fixtures/README.md`.

Installed Canon, Agfa and HP files were not used for conversion: their embedded
copyright alone did not establish file-specific development/distribution terms.
Thus manufacturer coverage here is two printing-profile vendors, not a hardware
RGB monitor/printer sample. Adobe's click-through download was not accepted.

## Versions and common conditions

- icc-profile 0.0.8 with atomic API at c9081ab0456aa1a130fb66cfb75d941ff4b3f26b.
- moxcms exactly 0.9.2, dependency lockfile retained in standalone harness.
- LittleCMS 2.19 (encoded version 2190), installed DLL used through public C API;
  its SHA256 and all profile hashes are in the result JSON. No external CMS
  implementation source was inspected or copied for this comparison.
- Windows x86_64; Rust 1.99.0 release build. No speed comparison is claimed here.
- Same source/destination bytes, relative-colorimetric intent, BPC off, no alpha,
  interleaved u16 input and sRGB2014 RGB16 output. ICC uses clamp=true.
- RGB 17^3 = 4913 points, CMYK 9^4 = 6561 points, endpoints included. Input samples
  round normalized grid coordinates to 0..65535; CMYK is conventional increasing
  ink, not an image-format-specific inverted convention.
- LittleCMS NOOPTIMIZE|NOCACHE (0x0140). moxcms CICP transfer override off,
  fixed-point preference off, tetrahedral requested, barycentric scale Low.
  Internal interpolation/curve quantization is not identical: moxcms documents
  paths where its interpolation option does not apply. Matching intent/sample
  settings does not assert bit-equivalent algorithms.

## Results against LittleCMS

Differences are absolute RGB16 codes (0..65535). Mean includes all three output
channels and all points. A point exceeds tolerance if any channel does.
1024 codes is the preselected broad screening tolerance from the existing LUT
regression fixture, not a perceptual or release-acceptance criterion. Additional
32/256-code exceedance counts are recorded to avoid hiding smaller differences.

| Source | Engine | Maximum | Mean | >32 points | >256 points | >1024 points |
|---|---|---:|---:|---:|---:|---:|
| X-Rite CRPC6 | moxcms | 321 | 38.2443 | 4526 | 3 | 0 |
| Heidelberg PSOcoated_v3 | moxcms | 318 | 38.0583 | 4326 | 4 | 0 |
| ICC sRGB v4 | icc-profile | 26 | 1.3981 | 0 | 0 | 0 |
| ICC sRGB v4 | moxcms | 143 | 60.7864 | 4786 | 0 | 0 |
| Synthetic Lab LUT | icc-profile | 8 | 0.7134 | 0 | 0 | 0 |
| Synthetic Lab LUT | moxcms | 140 | 62.1501 | 4817 | 0 | 0 |

All four profiles parsed successfully in icc-profile and moxcms. icc-profile
rejected the two CMYK transform constructions with
`UnsupportedProfileFeature("only Gray/RGB transforms are supported")`, as
required by its declared scope. Those are two expected unsupported cases, not
successful CMYK conversions and not zero-error color comparisons. Its two RGB
pipelines and all four moxcms/LittleCMS pipelines executed successfully; no
unexpected parse/construction/execution failures occurred. CMYK icc-profile
pixel-error metrics are intentionally absent.

These results do not prove a universal engine ranking, DeltaE conformance,
perceptual/absolute/saturation intent behavior, reverse RGB-to-CMYK correctness,
invalid-profile robustness or support for all manufacturer profiles. No
production change is justified solely by these aggregate differences.

## Reproduction

Download the originals under their terms to `_test_samples`. Build the separate
non-publishable harness; no dependency is added to the library itself:

```powershell
$env:CARGO_TARGET_DIR = Join-Path (Get-Location) 'target/cms-compare'
cargo build --locked --release --manifest-path tools/cms-compare/Cargo.toml
python tools/cms-compare/compare.py _test_samples/CGATS21_CRPC6.icc _test_samples/sRGB2014.icc .test-cms-compare/new-xrite
python tools/cms-compare/compare.py _test_samples/PSOcoated_v3.icc _test_samples/sRGB2014.icc .test-cms-compare/new-heidelberg
python tools/cms-compare/compare.py _test_samples/sRGB_v4_ICC_preference.icc _test_samples/sRGB2014.icc .test-cms-compare/new-srgb
python tools/cms-compare/compare.py tests/fixtures/lab-pcs-rgb.icc _test_samples/sRGB2014.icc .test-cms-compare/new-lablut
```

Create the output directory first. Python requires a locally discoverable
LittleCMS 2.19 DLL; the script refuses a different version and refuses an old
output prefix, preventing stale output from masquerading as a successful run.
A different DLL build with the same version is recorded by hash and may differ.
The harness is deliberately Windows-local for this run, not a portable CI job.
Raw aggregate records are in `cms-comparison-results/*-tetra.json`.

## CMYK v2 implementation checkpoint

The initial display-only route now admits v2 mft2 CMYK DeviceToPcs and RGB
output with relative colorimetric intent only. Four-dimensional multilinear
CLUT interpolation uses all 16 corners, first input axis slowest. Input table
application precedes interpolation; output tables follow it. The CMYK mft2
matrix is required to be identity (ICC.1:2022 lut16Type, non-PCSXYZ input).
No legacy CMYK fallback or external CMS implementation is used.

Heidelberg PSOcoated_v3 -> sRGB2014, same fixed comparison conditions above:
6561 points succeeded; maximum RGB16 difference 75, mean 9.6887 codes versus
LittleCMS 2.19; >32: 1030 points, >256: 0, >1024: 0. This establishes initial
coverage, not DeltaE acceptance or printing/separation support. The JSON is
`cms-comparison-results/heidelberg-cmyk-v2.json`.
