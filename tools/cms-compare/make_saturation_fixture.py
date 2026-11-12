"""Create an original synthetic v4 A2B2 profile, never modify vendor profiles.

The normalized RGB signal is an artificial encoded-Lab test cube, not a
calibrated RGB space or a profile for production use. ICC lutAToBType permits
an equal-channel B-curve-only stage. A2B0 uses gamma 2; A2B2 uses identity,
so accidental perceptual fallback is observably different.
"""
import struct
import sys
from pathlib import Path


def curve(gamma):
    if gamma == 1:
        return b"curv" + bytes(8)
    return b"curv" + bytes(4) + struct.pack(">IHH", 1, gamma * 256, 0)


def mab(gamma):
    header = bytearray(32)
    header[:4] = b"mAB "
    header[8:10] = bytes([3, 3])
    struct.pack_into(">I", header, 12, 32)
    return bytes(header) + curve(gamma) * 3


def make_profile():
    white = [63190, 65536, 54061]
    tags = [(b"A2B0", mab(2)), (b"A2B2", mab(1)),
            (b"wtpt", b"XYZ " + bytes(4) + struct.pack(">iii", *white)),
            (b"cprt", b"text" + bytes(4) + b"Original repository synthetic diagnostic; not calibrated.\0")]
    data = bytearray(132 + 12 * len(tags))
    data[8:12] = bytes([4, 3, 0, 0])
    data[12:24] = b"spacRGB Lab "
    data[36:40] = b"acsp"
    struct.pack_into(">I", data, 64, 2)
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
        target.write(make_profile())
    print(path)
