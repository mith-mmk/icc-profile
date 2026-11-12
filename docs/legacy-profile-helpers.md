# Checked legacy profile helpers

`try_cmyk_to_rgb_from_profile` and `try_rgb_to_cmyk_from_profile` return
`std::io::Result` and never substitute a profile-free conversion on failure.
They are checked legacy approximations, not ICC-accurate transforms.

Supported subset:
- CMYK output (`prtr`) class, Lab PCS, ICC major version 2 or 4.
- D50 header illuminant, allowing two fixed-point steps for the legacy decoder.
- An explicit A2B0 (4 to 3) or B2A0 (3 to 4) LUT8 with valid stage lengths and grid.
- No alternate rendering-intent tag selection or implicit tag fallback.

LUT16 and other tag types return `Unsupported`. LUT16 uses legacy ICC Lab
coding (L*=100 at code 65280), including in v4 profiles; the inherited byte
bridge cannot represent that route faithfully. Invalid LUT8 shapes return
`InvalidInput`; unsupported metadata and missing tags return `Unsupported`.

## Deliberate limitations

The inherited color bridge interprets RGB bytes through a linear RGB matrix,
without the sRGB transfer curve or D50/D65 chromatic adaptation. Its reverse
Lab encoding uses the historical -127..127 chroma mapping and truncates to
bytes. Its forward helper retains the historical C/Y permutation and Lab
mapping. The header illuminant is used by the legacy Lab equations; it is
not the media white point, and `wtpt` is not substituted for it. Three-channel
LUT evaluation retains the legacy matrix stage even for Lab input. Thus
successful validation does not certify color accuracy or full ICC semantics.

For color management, retain original ICC bytes and use `Profile` / `Transform`
with explicit source and destination profiles. `DecodedICCProfile` contains
parsed legacy fields, not the original bytes needed by the checked engine.

## Compatibility behavior

The tuple-returning names retain their signatures and historical profile-free
fallback values. Unsupported metadata, unsupported tags, and malformed LUTs
now reach that fallback explicitly rather than risking grid underflow or an
out-of-bounds read. They also retain the approximate LUT16 route for valid
metadata and shapes. This route still has legacy Lab coding/byte quantization
limitations and must not be treated as a correct ICC transform.

Valid reverse LUT routes now use the complete evaluator: matrix, input tables,
interpolated CLUT and output tables. Results intentionally change from the
old nearest-CLUT lookup. LUT16 output is scaled over 0..65535 to bytes rather
than just discarding its low byte. Metadata outside the supported subset now
falls back in the tuple API rather than being interpreted as Lab accidentally.
The separate ICC-free `cmyk_to_rgb(y,m,c,k)` function is unchanged.

Regression tests in `tests/legacy_profile_helpers.rs` use synthetic tables
with analytically known corner/interior results, independent of production
LUT helper outputs. They cover stage tables, matrix effect, interpolation,
endpoints, zero/one grids, channel/length errors, unsupported metadata/tag
routes, versions, white-point distinction, and legacy-only fallback behavior.
