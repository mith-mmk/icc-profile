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
bytes. Its forward helper uses C,M,Y,K channel order and historical Lab
mapping. The header illuminant is used by the legacy Lab equations; it is
not the media white point, and `wtpt` is not substituted for it. Three-channel
LUT evaluation retains the legacy matrix stage even for Lab input. Thus
successful validation does not certify color accuracy or full ICC semantics.

For color management, retain original ICC bytes and use `Profile` / `Transform`
with explicit source and destination profiles. `DecodedICCProfile` contains
parsed legacy fields, not the original bytes needed by the checked engine.

## Compatibility behavior

The tuple-returning names retain their signatures and historical admission:
only CMYK color space and an explicit A2B0/B2A0 LUT8/LUT16 tag are required.
They deliberately do not acquire the checked API's stricter PCS, version,
class or D50 restrictions. Unsupported metadata therefore does not silently
switch an existing legacy route to approximate fallback colors.

Historical profile-free fallback is retained only for non-CMYK profiles,
missing tags and unsupported tag types. Reverse conversion also retains the
exact old missing-nearest-node fallback condition. A private predicate uses
the old byte/u16 Lab quantization and checked index arithmetic only to decide
whether that formerly selected node exists; it never samples color values.
A partial CLUT with that node present fails full validation explicitly rather
than acquiring a new fallback case. Zero grid explicitly panics.

For a selected LUT, all other invalid shape or non-finite color-bridge failures
panic in the old tuple API and return errors in the applicable checked API.
The historical reverse missing-node test precedes these new validations, as
it did before. Its old saturated float-to-integer behavior is isolated from
actual evaluation, which rejects non-finite Lab values before casting.
The new `try_*` APIs never fall back and do not unwind on malformed inputs.

Valid reverse LUT routes now use the complete evaluator: matrix, input tables,
interpolated CLUT and output tables. Results intentionally change from the
old nearest-CLUT lookup. LUT16 output is scaled over 0..65535 to bytes rather
than just discarding its low byte. The retained legacy LUT16 route still has
Lab coding/byte quantization limitations and is not a correct ICC transform.

The separate ICC-free `cmyk_to_rgb(y,m,c,k)` function is unchanged. Only the
old profile-free fallback intentionally calls it as `cmyk_to_rgb(c,m,y,k)`,
preserving its historical result even when C and Y differ.

Regression tests in `tests/legacy_profile_helpers.rs` cover independent stage
constants/interior results, asymmetric channel order, matrix effect, endpoints,
shape errors, strict metadata rejection versus old admission, white-point
semantics, explicit panics versus checked errors, and exact historical
missing-node/fallback behavior.