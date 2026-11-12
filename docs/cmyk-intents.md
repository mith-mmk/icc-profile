# CMYK input intents: first CMS-completeness step

The display use case prioritizes perceptual and saturation input tables before
CMYK output or BPC. This change preserves the existing v2 mft2 / v4 mAB,
four-input to three-PCS, RGB-destination boundary and all resource limits.

Normative reference: [ICC.1:2022](https://www.color.org/specifications/ICC.1-2022-05.pdf),
section 8.10.2 and section 9.1 Table 25. Perceptual selects A2B0, relative selects
A2B1, saturation selects A2B2. Existing direction selection permits A2B0 fallback
when 1/2 are absent; RouteInfo exposes the chosen tag and fallback. A selected
malformed/unsupported tag fails without trying another route. No external CMS
implementation source was used.

Absolute CMYK remains rejected pending separate media-white/adaptation scope
and test review. BPC and output CMYK remain rejected. Matrix destinations keep
the existing media-white requirement for non-relative intents. This is not a
general new v2/v4 perceptual reference-medium connection implementation. The
sample results below do not establish all profile-pair combinations or perceptual
accuracy. The old relative behavior is unchanged.

## Verification

Synthetic v2/v4 fixtures have distinct constant A2B0/1/2 tables. Tests assert
analytic physical XYZ results, selected tags, fallback visibility, matrix/LUT
RGB destinations, worker and atomic contracts, malformed selected tags,
missing perceptual tables, and retained unsupported routes.

Pinned comparison uses LittleCMS 2.19 and moxcms 0.9.2 public APIs, existing
licensed X-Rite CGATS21_CRPC6 and Heidelberg PSOcoated_v3 profiles, sRGB2014
output, 9^4 = 6561 CMYK samples, u16, BPC off, LCMS NOOPTIMIZE|NOCACHE.
All four runs converted every sample in all three engines. Maximum / mean
absolute u16-channel differences against LittleCMS:

| Profile / intent | icc-profile | moxcms |
| --- | --- | --- |
| X-Rite perceptual | 185 / 7.884570 | 329 / 42.462887 |
| X-Rite saturation | 74 / 5.548036 | 321 / 38.244272 |
| Heidelberg perceptual | 443 / 18.213585 | 546 / 47.040238 |
| Heidelberg saturation | 443 / 18.213585 | 546 / 47.040238 |

All have zero samples over the pre-existing 1024-code screening threshold.
This is NOT a DeltaE criterion or release acceptance claim. Raw JSON with
hashes/versions/failure status is under cms-comparison-results/intent-*.json.
See manufacturer-cms-comparison.md for licenses and baseline conditions.

The comparison harness now takes an optional fourth argument: intent number
0=perceptual, 1=relative (default), 2=saturation, 3=absolute. Both Rust engines
and LittleCMS receive the same intent. Use a fresh prefix for each run:

```powershell
$env:CARGO_TARGET_DIR = Join-Path (Get-Location) 'target/cms-compare'
cargo build --locked --release --manifest-path tools/cms-compare/Cargo.toml
python tools/cms-compare/compare.py _test_samples/CGATS21_CRPC6.icc _test_samples/sRGB2014.icc .test-cms-compare/fresh-perceptual 0
```

Next candidates are absolute CMYK with explicit version-aware PCS semantics,
additional required input LUT encodings, then separately designed CMYK output
and BPC. These are not claimed as implemented by this change.
