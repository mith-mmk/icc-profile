# Legacy fixed-point numeric correction

The legacy `iccprofile::ICCNumber` conversions now use binary scaling:

| Type | Old divisor | Correct divisor |
| --- | ---: | ---: |
| S15Fixed16Number / U16Fixed16Number | 65535 | 65536 |
| U8Fixed8Number | 255 | 256 |
| U1Fixed15Number | 32767 | 32768 |

For example, signed raw `0x00008000` now decodes to exactly 0.5 rather
than 32768/65535. Signed raw `0xffffffff` is -1/65536 rather than zero.
Unsigned 8.8 raw `0x0080` is 0.5 rather than 128/255, and unsigned 1.15
raw `0x8000` is exactly 1 rather than 32768/32767. This intentionally changes
legacy rendering and formatted values. The checked transform engine's binary
reader already uses the correct scaling and is unchanged.

`int()` is the floor integer component, including for negative signed values.
U1Fixed15Number now returns its integer bit (0 or 1) instead of always zero.
`decimal()` retains its existing raw-field contract. In particular, 1.15
returns the entire stored 16-bit word, including the integer bit; callers can
mask with 0x7fff to obtain only fractional bits. Changing that historical
exception would be a separate compatibility decision.

`S15Fixed16Number::new` preserves its signature and uses floor(value * 65536)
for finite values in the inclusive range [-32768, 32767 + 65535/65536].
Thus 0.5 encodes as raw 32768 (previously 32767), and -0.5 as raw -32768
(previously -32769). Flooring is toward negative infinity, not toward zero.
`checked_new` returns None for nonfinite or out-of-range values, and Some for
valid values. Outside the exact range, `new` retains its old cast-based field
construction rather than introducing a panic or inventing a failure sentinel.
Those legacy invalid-input results are not meaningful ICC numbers; use
`checked_new` for untrusted numeric inputs. Corrected decoding still applies
to all stored fields, including fields produced by the compatibility fallback.

Tests use literal big-endian words through public `Data::xyz_number` and
`Data::get` readers, independent integer arithmetic, exact signed endpoints,
f32/f64 conversion, and exhaustive fractions at selected signed integers.
The 8.8 and 1.15 types have no public constructor/parser path, so their full
65,536-word domains are covered in private unit tests.
