"""Generate the paired Lab-LUT regression fixture with system LittleCMS.

Usage: python tests/generate_pcs_lut_oracle.py OUTPUT_DIRECTORY
Only fixture regeneration needs liblcms2; Rust tests use checked-in artifacts.
The recorded fixture and CSV were produced with LittleCMS 2.16.
"""
import csv
import ctypes as C
import ctypes.util
import struct
import sys
from pathlib import Path

DESTINATION = Path(sys.argv[1])
DESTINATION.mkdir(parents=True, exist_ok=True)
LCMS = C.CDLL(ctypes.util.find_library("lcms2"))
P, U, H = C.c_void_p, C.c_uint32, C.c_uint16


def function(name, result, arguments):
    call = getattr(LCMS, name)
    call.restype, call.argtypes = result, arguments
    return call


create_srgb = function("cmsCreate_sRGBProfile", P, [])
create_lab = function("cmsCreateLab4Profile", P, [P])
placeholder = function("cmsCreateProfilePlaceholder", P, [P])
set_class = function("cmsSetDeviceClass", None, [P, U])
set_color = function("cmsSetColorSpace", None, [P, U])
set_pcs = function("cmsSetPCS", None, [P, U])
set_version = function("cmsSetProfileVersion", None, [P, C.c_double])
read_tag = function("cmsReadTag", P, [P, U])
write_tag = function("cmsWriteTag", C.c_int, [P, U, P])
build_gamma = function("cmsBuildGamma", P, [P, C.c_double])
free_curve = function("cmsFreeToneCurve", None, [P])
allocate_pipeline = function("cmsPipelineAlloc", P, [P, U, U])
curve_stage = function("cmsStageAllocToneCurves", P, [P, U, P])
clut_stage = function("cmsStageAllocCLut16bit", P, [P, U, U, U, P])
insert_stage = function("cmsPipelineInsertStage", C.c_int, [P, C.c_int, P])
free_pipeline = function("cmsPipelineFree", None, [P])
create_transform = function("cmsCreateTransform", P, [P, U, P, U, U, U])
transform = function("cmsDoTransform", None, [P, P, P, U])
delete_transform = function("cmsDeleteTransform", None, [P])
save_profile = function("cmsSaveProfileToFile", C.c_int, [P, C.c_char_p])
open_profile = function("cmsOpenProfileFromFile", P, [C.c_char_p, C.c_char_p])
close_profile = function("cmsCloseProfile", C.c_int, [P])
version = function("cmsGetEncodedCMMversion", U, [])


def signature(value):
    return int.from_bytes(value.encode("ascii"), "big")


RGB16 = (4 << 16) | (3 << 3) | 2
LAB16 = (10 << 16) | (3 << 3) | 2
RELATIVE = 1
FLAGS = 0x0100 | 0x0040  # cmsFLAGS_NOOPTIMIZE | cmsFLAGS_NOCACHE; BPC off.
srgb, lab, profile = create_srgb(), create_lab(None), placeholder(None)
assert srgb and lab and profile
set_class(profile, signature("mntr"))
set_color(profile, signature("RGB "))
set_pcs(profile, signature("Lab "))
set_version(profile, 4.3)
assert write_tag(profile, signature("wtpt"), read_tag(srgb, signature("wtpt")))
grid = 17
axis = [round(i * 65535 / (grid - 1)) for i in range(grid)]
inputs = (H * (grid**3 * 3))(
    *[v for x in axis for y in axis for z in axis for v in (x, y, z)]
)
for name, source, target, input_type, output_type in [
    ("A2B0", srgb, lab, RGB16, LAB16),
    ("B2A0", lab, srgb, LAB16, RGB16),
]:
    conversion = create_transform(source, input_type, target, output_type, RELATIVE, FLAGS)
    assert conversion
    outputs = (H * len(inputs))()
    transform(conversion, inputs, outputs, grid**3)
    delete_transform(conversion)
    pipeline = allocate_pipeline(None, 3, 3)
    curves = (P * 3)(*[build_gamma(None, 1) for _ in range(3)])
    for stage in [
        curve_stage(None, 3, curves),
        clut_stage(None, grid, 3, 3, outputs),
        curve_stage(None, 3, curves),
    ]:
        assert stage and insert_stage(pipeline, 1, stage)  # cmsAT_END
    for curve in curves:
        free_curve(curve)
    assert write_tag(profile, signature(name), pipeline)
    free_pipeline(pipeline)
profile_path = DESTINATION / "lab-pcs-rgb.icc"
assert save_profile(profile, str(profile_path).encode())
close_profile(profile)
close_profile(srgb)
close_profile(lab)
# Normalize the nonsemantic creation date for reproducible bytes.
data = bytearray(profile_path.read_bytes())
data[24:36] = struct.pack(">6H", 2026, 10, 18, 0, 0, 0)
profile_path.write_bytes(data)
# Reopen the exact serialized profile for the independent end-to-end oracle.
profile = open_profile(str(profile_path).encode(), b"r")
assert profile
conversion = create_transform(profile, RGB16, profile, RGB16, RELATIVE, FLAGS)
assert conversion
vectors = [
    (0, 0, 0), (32768, 32768, 32768), (65535, 0, 0),
    (0, 65535, 0), (0, 0, 65535), (65535, 65535, 65535),
    (10000, 30000, 50000),
]
inputs = (H * (len(vectors) * 3))(*[v for row in vectors for v in row])
outputs = (H * len(inputs))()
transform(conversion, inputs, outputs, len(vectors))
with (DESTINATION / "lab-pcs-rgb.csv").open("w", newline="", encoding="utf-8") as stream:
    writer = csv.writer(stream, lineterminator="\n")
    writer.writerow(["r", "g", "b", "out_r", "out_g", "out_b"])
    for i, vector in enumerate(vectors):
        writer.writerow([*vector, *outputs[i * 3:i * 3 + 3]])
delete_transform(conversion)
close_profile(profile)
print(f"LittleCMS encoded version {version()}: {len(vectors)} vectors")
