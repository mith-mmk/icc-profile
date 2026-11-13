# Direct ColorSync CMYK intent comparison (macOS only)

This opt-in diagnostic compares identical raw CMYK8 input and identical profile
bytes through `icc-profile` and Apple's public `ColorSyncTransformCreate` /
`ColorSyncTransformConvert` APIs. It does not use image decoding as the primary
oracle. No external CMS implementation source is read.

Build and run from the repository root (macOS SDK, Rust, Python 3, `sips`):

```sh
mkdir .test-colorsync-build
clang -Wall -Wextra -Werror tools/colorsync-compare/colorsync.c \
  -framework ColorSync -framework CoreFoundation \
  -o .test-colorsync-build/colorsync
CARGO_TARGET_DIR="$PWD/.test-colorsync-build/target" \
  cargo build --locked --offline --release --manifest-path tools/colorsync-compare/Cargo.toml
python3 tools/colorsync-compare/compare.py /path/to/source-CMYK.icc \
  '/System/Library/ColorSync/Profiles/sRGB Profile.icc' .test-colorsync-run \
  --icc-bin "$PWD/.test-colorsync-build/target/release/icc-colorsync-compare" \
  --colorsync-bin "$PWD/.test-colorsync-build/colorsync"
```

The output directory must not already exist. `--offline` assumes cached Rust
dependencies. The checked-in result identifies the exact profile hashes; this
repository does not redistribute the local SWOP or system profiles.

- The two grids are 5^4=625 and 9^4=6561 samples, C varying fastest.
- Both ColorSync sequence entries receive the same explicit intent, BPC false
  and BestQuality (no coalescing). Direction is device-to-PCS then PCS-to-device.
  The system-default CMM is not pinned to a separate version.
- A separate BPC=true ColorSync run diagnoses BPC sensitivity; it is not a
  comparison to an icc-profile BPC-enabled transform.
- The raw pipeline uses CMYK8/RGB8, no alpha. ICC uses its default interpolation
  and BPC=false. Unsupported absolute remains an error and is not a pass.
- The sips experiment embeds the same original profile in a simple TIFF and
  repeats the historical command. It is deliberately a separate pipeline.
- Header controls change **only bytes 64..68 of private in-memory copies** of
  the source profile (0,1,2,3), embed those copies in new diagnostic TIFFs, and
  repeat all four sips command intents. Original profiles remain unchanged.
- `report.json` includes profile/tag hashes, routes, statuses, output hashes,
  worst samples and differences. Complete raw inputs/outputs and logs stay in
  the run directory. A tool failure is not an accuracy pass.

Move generated build directories and redundant experiment outputs to Trash when
finished. See `docs/colorsync-intents-validation-20261114.md` for the measured
result and limits. Do not use sips as an intent-matched CMYK oracle without
verifying its behavior with controls on that OS/profile pair.
