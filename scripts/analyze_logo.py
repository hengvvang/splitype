"""Print an ASCII preview of a PNG to check the rendered content."""
import struct, zlib, sys

def decode_png(path):
    data = open(path, "rb").read()
    pos = 8
    w = h = ch = None
    idat = b""
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos+4])
        ctype = data[pos+4:pos+8]
        chunk = data[pos+8:pos+8+length]
        if ctype == b"IHDR":
            w, h, bitdepth, colortype = struct.unpack(">IIBB", chunk[:10])
        elif ctype == b"IDAT":
            idat += chunk
        elif ctype == b"IEND":
            break
        pos += 12 + length
    raw = zlib.decompress(idat)
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[colortype]
    stride = w * channels
    out = bytearray()
    prev = bytearray(stride)
    p = 0
    for y in range(h):
        ft = raw[p]; p += 1
        line = bytearray(raw[p:p+stride]); p += stride
        if ft == 1:
            for i in range(channels, stride):
                line[i] = (line[i] + line[i-channels]) & 0xFF
        elif ft == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 0xFF
        elif ft == 3:
            for i in range(stride):
                a = line[i-channels] if i >= channels else 0
                line[i] = (line[i] + ((a + prev[i]) >> 1)) & 0xFF
        elif ft == 4:
            for i in range(stride):
                a = line[i-channels] if i >= channels else 0
                b = prev[i]
                c = prev[i-channels] if i >= channels else 0
                pp = a + b - c
                pa, pb, pc = abs(pp-a), abs(pp-b), abs(pp-c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[i] = (line[i] + pr) & 0xFF
        out += line
        prev = line
    return w, h, channels, bytes(out)

w, h, ch, px = decode_png(sys.argv[1])
gw, gh = 48, 48
for gy in range(gh):
    row = ""
    for gx in range(gw):
        # sample the center of each cell
        x = int((gx + 0.5) * w / gw)
        y = int((gy + 0.5) * h / gh)
        i = (y * w + x) * ch
        r, g, b = px[i], px[i+1], px[i+2]
        a = px[i+3] if ch == 4 else 255
        lum = (r + g + b) / 3
        if a < 128:
            row += " "
        elif lum < 128:
            row += "#"
        elif lum < 200:
            row += "+"
        elif lum < 245:
            row += "."
        else:
            row += " "
    print(row)
