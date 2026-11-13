# Scalar Lab/XYZ and white-point corrections

These profile-free helpers keep their public signatures, D65 defaults, XYZ
scale and unclamped floating-point results. No gamma or ICC engine selection
semantics change. Correcting the math intentionally changes numeric outputs.

- Lab to XYZ now selects the inverse CIE 1976 branch separately for fx and fz,
  not from fy for all three channels. The threshold is delta = 6/29.
- `WhitePoint::d50()` changes from (0.9568, 1, 0.9214) to
  (0.9642, 1, 0.8251); `d55()` changes from (0.9642, 1, 0.8251) to
  (0.9568, 1, 0.9214). These rounded CIE 1931 2-degree illuminants were swapped.
- `WhitePoint::icc()` changes X from 1.0371292263 to 0.9642;
  Y=1 and Z=0.8249 remain the ICC PCS D50 reference values. This intentional
  difference from rounded CIE D50 Z=0.8251 is retained.
- Both Lab batch helpers check `entries * 3` before indexing/allocation;
  overflow now returns InvalidInput instead of panicking or wrapping. Existing
  short-input errors, zero-entry and trailing-input behavior are preserved.

For Lab (5, 100, -100) with the unchanged D65 (0.9504, 1, 1.0888):

- Before: XYZ (0.029670530157038, 0.005535282299397, 0.075937873631555).
- After: XYZ (0.052577419607200, 0.005535282299397, 0.343918405326172).

The new regression test uses independent CIE inverse-equation reference values:
fx=(L+16)/116+a/500, fy=(L+16)/116, fz=(L+16)/116-b/200; each
component uses f^3 above delta and 3*delta^2*(f-4/29) below it.
For the example above the exact rational XYZ values are
3205776717/60972500000, 135/24389, and 671026079/1951120000.
For f=5/29, 6/29 and 7/29, inverse values are respectively
108/24389, 216/24389 and 343/24389; all 27 component combinations
are tested. L=8 continuity, dark/bright chromatic cases, negative XYZ,
neutral whites, round trips and batch limits are also covered.

References: CIE 1976 L*a*b* equations and the ICC.1 profile specification
PCS adopted white (https://www.color.org/icc_specs2.xalter).
The pre-existing color-space test also expected D55 X under the D50 name.
Its reference is corrected to 0.9642 without widening its tolerance.
