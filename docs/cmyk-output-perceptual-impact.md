# CMYK output differences: predicted perceptual impact

## Why code differences alone are insufficient

Four CMYK channels describe ink separations, not three perceptually uniform
coordinates. A large per-channel code difference need not mean the same size
visible colour difference. We therefore transform both outputs through the
same X-Rite CGATS21_CRPC6 A2B1 using LittleCMS 2.19 into physical Lab (double),
then evaluate CIEDE2000 and CIE76. This common evaluation leg uses relative
colourimetric intent, NOOPTIMIZE|NOCACHE, and no BPC. It predicts printed colour
under this profile; it is not a printer measurement, a proof of absolute colour
accuracy, or a universal visibility threshold.

## 17^3 RGB grid

All 4,913 previously compared colors are included. CIEDE2000 differences
between icc-profile and LittleCMS (mean / p95 / p99 / max):

- BPC off, trilinear: 0.00187 / 0.00451 / 0.00630 / 0.01163.
- BPC off, tetrahedral: 0.06888 / 0.23462 / 0.41540 / 1.32134.
- BPC on, trilinear: 0.00229 / 0.00536 / 0.00726 / 0.01299.
- BPC on, tetrahedral: 0.06778 / 0.23353 / 0.41887 / 1.60543.

The earlier maximum CMYK difference of 2859 codes has DeltaE00 0.95749.
The worst perceptual BPC-on difference is instead at RGB16 [8192,4096,16384]
(a dark violet). Only 2/4913 BPC-on tetrahedral samples exceed DeltaE00 1;
none exceeds 2. These grid maxima are not bounds on the entire RGB space.

## Actual photograph, every pixel

A NASA photograph of Eileen Collins, the scikit-image astronaut dataset, is
512x512 RGB. All 262,144 pixels were evaluated without resizing; each 8-bit
component becomes 16-bit by multiplication by 257. Its embedded sRGB
IEC61966-2.1 profile is used by both engines; we do not substitute sRGB2014.
The image and embedded vendor ICC remain local and are not committed.

Provenance: https://scikit-image.org/docs/stable/api/skimage.data#skimage.data.astronaut
The dataset documentation identifies NASA and public-domain status. The image
was extracted from the official scikit-image 0.25.2 PyPI wheel, not generated.
Photo SHA256: 88431cd9653ccd539741b555fb0a46b61558b301d4110412b5bc28b5e3ea6cb5.
Embedded ICC SHA256: 2b3aa1645779a9e634744faf9b01e9102b0c9b88fd6deced7934df86b949af7e.
An earlier Wikimedia landscape download returned HTTP403 and was not bypassed;
it was not used in these results.

CIEDE2000 differences (mean / p95 / p99 / max):

- BPC off, trilinear: 0.00399 / 0.00948 / 0.01221 / 0.02906.
- BPC off, tetrahedral: 0.05277 / 0.19954 / 0.42191 / 2.95137.
- BPC on, trilinear: 0.00440 / 0.00961 / 0.01401 / 0.02455.
- BPC on, tetrahedral: 0.05867 / 0.22759 / 0.44923 / 1.72302.

BPC-on tetrahedral exceeds 1 in 417 pixels (0.1591%) and exceeds 2 in none.
BPC-off tetrahedral exceeds 2 in six pixels (0.00229%). The worst BPC-off
pixel is RGB8 [27,19,51] at (42,24), and worst BPC-on is [31,19,66] at (13,40),
with top-left, zero-based coordinates. Thus real photographic inputs exhibit
localized differences, predominantly dark violet here. It would be incorrect
to promise that all differences are invisible or that a coarse grid guarantees
all photographic colors.

## Cause, controls, and algorithm selection

The BPC-off and BPC-on tetrahedral distributions are similar; switching only
our 3D interpolation to trilinear sharply reduces differences. Local tri/tetra
comparisons with identical source/destination/intent/BPC are also recorded.
These controlled results, together with earlier independent LUT evaluation,
strongly support interpolation as the dominant source of these differences.
LittleCMS interpolation remains engine-selected: this is not a claim that its
internal algorithms were inspected or that every residual has been isolated.

BPC on/off is a separate intentional operation. Even LittleCMS differs from
itself by mean DeltaE00 0.94528 and maximum 5.63983 on the photograph when BPC
is toggled. That effect must not be confused with implementation disagreement.

The existing replaceable algorithm design is retained:
AlgorithmProfile::default().with_clut_3d(ClutInterpolation3D::Trilinear), selected
with Transform::with_algorithm_profile, bundles the numerical choice. The
historical default stays tetrahedral, and 4D CMYK remains multilinear. BPC
coefficients/diagnostics select the matching precomputed estimate; clone/worker
stability is regression-tested. Private profile fields and non-exhaustive
algorithm enums allow later choices/presets without adding a false CMS-name
compatibility promise. Trilinear is an explicit option for users prioritizing
agreement with this measured reference; neither mode is declared universally
more correct from these tests alone.

## Reproduction and limits

Run the existing grid comparison harness for BPC on/off with fresh prefixes,
then tools/cms-compare/measure_output_perceptual_error.py. It expects the
recorded output-xrite-* (off) and output-bpc-final-xrite-* (on) raw files.
For the photograph, obtain astronaut.png from official scikit-image 0.25.2,
place it in .test-cms-compare, then from the repository root:

    python tools/cms-compare/prepare_photo.py
    cargo run --release --locked --manifest-path tools/cms-photo-compare/Cargo.toml
    python tools/cms-compare/measure_photo_perceptual_error.py

Python Pillow and locally discoverable LittleCMS 2.19 are required. The Rust
harness and scripts intentionally write only named files in .test-cms-compare.
Do not mix outputs from different core versions. The known Sharma DeltaE00
case (50,2.6772,-79.7751) vs (50,0,-82.7485) was checked at 2.04245968.
Both JSON reports retain hashes, metrics, exceedance counts and worst inputs:
cms-comparison-results/output-xrite-perceptual-error.json and
cms-comparison-results/photo-xrite-perceptual-error.json. CIE76 is included too.

One photo and one printing profile do not establish all-image/all-printer
coverage. Paper, ink, viewing conditions, printer variation and measurement
uncertainty were not measured. No production algorithm default is changed.
