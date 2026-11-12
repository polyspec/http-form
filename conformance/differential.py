"""Compares the core with hyper's Python reader on random bodies: the form values, the files and the failure of each
body, read whole and in random chunks (V-2). The first argument is the source directory of hyper's Python server, the
second the number of bodies. The core runs as `cargo run --example differential`.

This test reads a module of hyper that is not public (H-25); its result is a reference only. A difference that a rule
of this library explains (H-17, H-18: a failure where hyper's reader gives no values) is counted apart from the
others."""

import json
import random
import subprocess
import sys

sys.path.insert(0, sys.argv[1])
from polyspec.hyper._fields import Fields  # noqa: E402
from polyspec.hyper._files import Files  # noqa: E402
from polyspec.hyper._multipart import Multipart  # noqa: E402

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


def files_of(pairs):
    return [[n, [[f.filename(), f.content_type(), f.content().hex()] for f in items]] for n, items in pairs]


def reference(kind, data, cuts):
    text = kind.decode("latin-1")
    fields, files = Fields.from_body(text, data), Files.from_body(text, data)
    whole = [None if fields is None else [list(p) for p in fields.pairs()], None if files is None else files_of(files.pairs())]
    boundary = Multipart.boundary(text)
    chunked = None
    if boundary is not None:
        reader = Multipart(boundary, None, None, None, None)
        at = 0
        for cut in cuts + [len(data)]:
            reader.feed(data[at:cut])
            at = cut
        read = reader.result()
        chunked = None if read is None else [[list(p) for p in read[0].pairs()], files_of(read[1].pairs())]
    return {"whole": whole, "chunked": chunked}


def unclosed(data):
    """True when a header line of a part holds a quoted parameter value without its end (H-18)."""
    for line in data.split(b"\r\n"):
        head, _, rest = line.partition(b":")
        if head.strip().lower() != b"content-disposition":
            continue
        at = 0
        while True:
            equals = rest.find(b"=", at)
            if equals < 0:
                break
            if rest[equals + 1:equals + 2] == b'"':
                close = rest.find(b'"', equals + 2)
                if close < 0:
                    return True
                at = close + 1
            else:
                at = equals + 1
    return False


def failed(value):
    return isinstance(value, dict) and "raises" in value


def as_reference(result):
    """The core result with a failure as null, as hyper's reader gives it."""
    whole = [None if failed(v) else v for v in result["whole"]]
    chunked = result["chunked"]
    if chunked is not None:
        chunked = None if failed(chunked[0]) else chunked
    return {"whole": whole, "chunked": chunked}


def main():
    rng = random.Random(20261112)
    count = int(sys.argv[2])
    kinds = [b"multipart/form-data; boundary=X", b"multipart/form-data;boundary=\"X\"", b"Multipart/Form-Data ; boundary=X",
             b"application/x-www-form-urlencoded", b"text/plain", b"multipart/form-data; boundary="]
    inputs = []
    for _ in range(count):
        kind = rng.choice(kinds)
        data = body(rng)
        cuts = sorted(rng.sample(range(len(data) + 1), min(len(data) + 1, rng.randint(0, 4))))
        inputs.append((kind, data, cuts))
    lines = "".join(json.dumps({"type": k.hex(), "body": d.hex(), "cuts": c}) + "\n" for k, d, c in inputs)
    run = subprocess.run(["cargo", "run", "--quiet", "--release", "--example", "differential"], input=lines.encode(),
                         capture_output=True, check=True)
    results = [json.loads(line) for line in run.stdout.decode().splitlines()]
    assert len(results) == count
    by_rule, other = 0, 0
    for (kind, data, cuts), result in zip(inputs, results):
        expected, actual = reference(kind, data, cuts), as_reference(result)
        if expected == actual:
            continue
        kinds_raised = {v["raises"] for v in result["whole"] if failed(v)}
        if kinds_raised & {"no_boundary", "malformed_content_type"} or (
                "invalid" in kinds_raised and expected["whole"][0] is not None and unclosed(data)):
            by_rule += 1
            continue
        other += 1
        if other <= 5:
            print("differs:", kind, data, cuts, "\n  reference", expected, "\n  core     ", result)
    print(f"{count} bodies, {by_rule} differ by H-17 or H-18, {other} differ otherwise")
    sys.exit(1 if other else 0)


main()
