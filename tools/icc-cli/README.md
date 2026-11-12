# icc-cli

Small, unpublished Rust 1.91 command-line frontend for this repository's checked
`Profile` and `Transform` APIs. It has no extra runtime dependencies, image codec,
implicit system profile, legacy CMS fallback, or output-file writer.

From the repository root:

```sh
cargo +1.91.0 build --manifest-path tools/icc-cli/Cargo.toml --locked
cargo +1.91.0 test --manifest-path tools/icc-cli/Cargo.toml --locked
cargo +1.91.0 run --manifest-path tools/icc-cli/Cargo.toml -- inspect tests/fixtures/lab-pcs-rgb.icc
cargo +1.91.0 run --manifest-path tools/icc-cli/Cargo.toml -- validate tests/fixtures/lab-pcs-rgb.icc
cargo +1.91.0 run --manifest-path tools/icc-cli/Cargo.toml -- convert --source tests/fixtures/lab-pcs-rgb.icc --destination tests/fixtures/lab-pcs-rgb.icc --format f32 --samples "0.2,0.4,0.7 0,0,1" --intent relative --interpolation tetrahedral
```

The package is an independent workspace with `publish = false` and a local path
dependency. The synthetic fixture is not an identity profile or a real display
profile. Examples demonstrate the interface, not color accuracy.

## Commands and contract

- `inspect PROFILE`: `Profile::parse`, followed by declared size, raw version/class
  bytes (hex), device color space, PCS, and header intent. Header summary only; no
  tag inventory or route compilation.
- `validate PROFILE`: `Profile::from_bytes`, which performs the library's checked
  container and supported semantic checks. It does **not** validate every tag,
  guarantee either transform direction, or certify universal ICC conformance.
- `convert --source PROFILE --destination PROFILE --format f32|u8|u16 --samples "VALUES"`:
  structurally parses both profiles, compiles the requested checked transform,
  converts the numeric device samples, and prints one destination pixel per line.
  Source and destination are explicit profile file paths, not sample files.
- `--help`, `-h`, or `help`: usage. Only standalone help is recognized.

Optional convert flags:

- `--intent relative|perceptual|saturation|absolute`, default `relative`.
- `--interpolation tetrahedral|trilinear`, default `tetrahedral`.

Options are separate name/value pairs in any order; duplicate/unknown flags and
missing values fail. Paths can contain spaces when quoted. Only paths can contain
non-UTF-8 OS strings. There is no `--key=value` syntax, stdin mode, automatic profile
selection, or alpha handling. The header intent does not override `--intent`.

Samples are separated by commas or ASCII whitespace. Empty comma fields fail.
Multiple pixels are interleaved in device channel order (Gray; R,G,B; C,M,Y,K for
supported CMYK source routes). The component count must be a positive multiple of
source channel count. f32 inputs must be finite in inclusive 0..1; u8 and u16 must
be integers in inclusive 0..255 and 0..65535. Output uses the same numeric type,
space-separated channels and newline-separated pixels. Decimal floats use Rust's
round-trip display, without fixed decimal precision. f32 processing is the engine
core; integers use the checked library's scaling and rounding.

Black point compensation is off and output clipping is on. The 3D interpolation
choice affects only 3D CLUTs; 4D remains multilinear. Matrix/curve-only paths are
unaffected. Unsupported routes fail rather than using legacy approximations.
The checked engine's documented same-direction intent tag selection/fallback is
retained. CMYK is limited to the engine's supported relative-colorimetric source
route; there is no general CMYK-output support or engine compatibility promise.

All computations finish before result text is written to stdout. On argument,
profile, or transform failure stdout is empty; diagnostics go to stderr. A stdout
I/O failure may leave partial output. The executable never opens an output file,
so cannot itself overwrite a file; shell redirection can overwrite and is the
caller's responsibility. Metadata text is human-readable, not a stable JSON API.

Exit codes: 0 success; 2 invalid arguments/samples; 3 input/output I/O failure or
non-regular input file; 4 checked profile/transform rejection or profile size limit.
OS termination is outside these codes.

## Resource limits and verification

Each profile must be a regular file at most 64 MiB. Reads use a 64 MiB + 1 byte cap
even if a file grows after metadata inspection. Non-regular paths are checked
before open and again on the opened handle. This is not a race-proof guarantee
against concurrent hostile filesystem changes. Total OS argument string length is
at most 1 MiB (the OS itself may impose a much smaller command-line limit), and
sample count is at most 65,536 components. Library default parse/compile limits
remain active. There is no streaming or image-size allocation. Standard-library
allocation failure and externally terminated processes are not caught.

Tests use only the repository synthetic fixture and generated malformed inputs.
They cover successful metadata/validation, both interpolation choices against the
checked API for all three sample formats, ranges, channel lengths, command errors,
missing/unsupported/malformed profiles, oversized files and argument/sample limits.
No external CMS implementation source was used.

On Unix, the FIFO regression test requires the standard mkfifo command.
