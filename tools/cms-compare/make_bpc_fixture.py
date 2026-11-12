"""Original neutral matrix/TRC diagnostics with a selectable nonzero black.
Not a calibrated device profile. Vendor profiles are never modified.
"""
import struct
import sys
from pathlib import Path


def make_profile(black_code):
    if not 0 <= black_code < 65535:
        raise ValueError("black_code must be 0..65534")
    white = [63190, 65536, 54061]
    xyz = lambda values: b"XYZ " + bytes(4) + struct.pack(">iii", *values)
    curve = b"curv" + bytes(4) + struct.pack(">IHH", 2, black_code, 65535)
    tags = [(b"wtpt", xyz(white)),
            (b"rXYZ", xyz([white[0], 0, 0])),
            (b"gXYZ", xyz([0, white[1], 0])),
            (b"bXYZ", xyz([0, 0, white[2]])),
            (b"rTRC", curve), (b"gTRC", curve), (b"bTRC", curve),
            (b"cprt", b"text" + bytes(4) + b"Original synthetic matrix diagnostic; not calibrated.\0")]
    data = bytearray(132 + 12 * len(tags))
    data[8:12] = bytes([4, 3, 0, 0])
    data[12:24] = b"mntrRGB XYZ "
    data[36:40] = b"acsp"
    struct.pack_into(">I", data, 64, 1)
    struct.pack_into(">iii", data, 68, *white)
    struct.pack_into(">I", data, 128, len(tags))
    for index, (name, payload) in enumerate(tags):
        data.extend(bytes((-len(data)) % 4))
        offset = len(data)
        struct.pack_into(">4sII", data, 132 + index * 12, name, offset, len(payload))
        data.extend(payload)
    struct.pack_into(">I", data, 0, len(data))
    return bytes(data)


if __name__ == "__main__":
    path = Path(sys.argv[1])
    with path.open("xb") as target:
        target.write(make_profile(int(sys.argv[2])))
    print(path)
