"""Compares the core, through the Python binding, with a reference reader on random bodies: the form values, the
files and the failure of each body, read whole and in random chunks. The reference is hyper's Python reader, whose
source directory is the first argument; the second is the number of bodies."""

import random
import sys

sys.path.insert(0, sys.argv[1])
from polyspec.hyper._fields import Fields  # noqa: E402
from polyspec.hyper._files import Files  # noqa: E402
from polyspec.hyper._multipart import Multipart  # noqa: E402

import http_forms as forms  # noqa: E402

PIECES = [
    b"--X", b"--X--", b"\r\n", b"\r", b"\n", b"--", b"-", b"X", b"Content-Disposition: ", b"content-disposition:",
    b"form-data", b"FORM-DATA", b"attachment", b"; ", b";", b"\t", b" ", b"\xa0", b"name=", b'name="a"', b"name=a",
    b'filename="f"', b"filename=", b'filename="\\"x', b'"', b"\\", b"Content-Type: text/plain", b"Content-Type:",
    b"\xff", b"\xc3\xa9", b"\xe2\x80", b"value", b"=", b"%41", b"\x00", b"\r\n\r\n", b"--X\r\n", b"\r\n--X",
]


def body(rng):
    if rng.random() < 0.5:
        parts = []
        for _ in range(rng.randint(0, 4)):
            header = rng.choice([
                b'Content-Disposition: form-data; name="a"',
                b'Content-Disposition: form-data; name="f"; filename="x.txt"\r\nContent-Type: text/plain',
                b"Content-Disposition: form-data; name=b",
                b"X-Other: 1",
            ])
            content = b"".join(rng.choice(PIECES) for _ in range(rng.randint(0, 5)))
            parts.append(b"--X\r\n" + header + b"\r\n\r\n" + content + b"\r\n")
        tail = rng.choice([b"--X--\r\n", b"--X--", b"--X", b"", b"--X--trailing"])
        return b"".join(parts) + tail
    return b"".join(rng.choice(PIECES) for _ in range(rng.randint(0, 30)))


def reference(kind, data, cuts):
    whole = (Fields.from_body(kind, data), Files.from_body(kind, data))
    out = [
        None if whole[0] is None else [list(p) for p in whole[0].pairs()],
        None if whole[1] is None else [[n, [(f.filename(), f.content_type(), f.content()) for f in items]] for n, items in whole[1].pairs()],
    ]
    boundary = Multipart.boundary(kind)
    chunked = None
    if boundary is not None:
        reader = Multipart(boundary, None, None, None, None)
        at = 0
        for cut in cuts + [len(data)]:
            reader.feed(data[at:cut])
            at = cut
        read = reader.result()
        chunked = None if read is None else [
            [list(p) for p in read[0].pairs()],
            [[n, [(f.filename(), f.content_type(), f.content()) for f in items]] for n, items in read[1].pairs()],
        ]
    return out, chunked


def core(kind, data, cuts):
    fields = forms.fields_from_body(kind, data)
    files = forms.files_from_body(kind, data)
    out = [
        None if fields is None else [[n, list(v)] for n, v in fields],
        None if files is None else [[n, [(f.filename(), f.content_type(), f.content()) for f in items]] for n, items in files],
    ]
    boundary = forms.multipart_boundary(kind)
    chunked = None
    if boundary is not None:
        chunks, at = [], 0
        for cut in cuts + [len(data)]:
            chunks.append(data[at:cut])
            at = cut
        read = forms.parse_multipart_chunks(chunks, boundary)
        chunked = None if read is None else [
            [[n, list(v)] for n, v in read[0]],
            [[n, [(f.filename(), f.content_type(), f.content()) for f in items]] for n, items in read[1]],
        ]
    return out, chunked


def main():
    rng = random.Random(20261112)
    count = int(sys.argv[2])
    kinds = ["multipart/form-data; boundary=X", "multipart/form-data;boundary=\"X\"", "Multipart/Form-Data ; boundary=X",
             "application/x-www-form-urlencoded", "text/plain", "multipart/form-data; boundary="]
    differ = 0
    for index in range(count):
        kind = rng.choice(kinds)
        data = body(rng)
        cuts = sorted(rng.sample(range(len(data) + 1), min(len(data) + 1, rng.randint(0, 4))))
        expected, actual = reference(kind, data, cuts), core(kind, data, cuts)
        if expected != actual:
            differ += 1
            if differ <= 5:
                print("differs:", repr(kind), repr(data), cuts, "\n  reference", expected, "\n  core     ", actual)
    print(f"{count} bodies, {differ} differ")
    sys.exit(1 if differ else 0)


main()
