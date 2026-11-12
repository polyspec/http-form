"""Writes the bodies of the benchmark into the directory of the first argument: a complex urlencoded form, a complex
multipart form with 300 text fields and 10 files, and a 50 MiB attachment."""
import sys
import random, urllib.parse
rng = random.Random(7)
words = ["서울", "부산", "alpha", "beta", "δοκιμή", "x" * 40, "line1\r\nline2", "100%", "a&b=c", "😀"]
# complex urlencoded form: 1000 pairs, bracketed and repeated names, percent-encoded UTF-8
pairs = []
for i in range(1000):
    name = rng.choice([f"items[{i%50}][name]", f"items[{i%50}][qty]", "tags[]", f"field{i}", "note"])
    pairs.append(urllib.parse.quote_plus(name) + "=" + urllib.parse.quote_plus(" ".join(rng.choice(words) for _ in range(3))))
open(sys.argv[1] + "/form.txt", "wb").write("&".join(pairs).encode())
# complex multipart form: 300 text fields and 10 files of 8 KiB
b = b"----bench7MA4YWxkTrZu0gW"
parts = []
for i in range(300):
    name = rng.choice([f"items[{i%50}][name]", "tags[]", f"field{i}", "note"]).encode()
    value = " ".join(rng.choice(words) for _ in range(3)).encode()
    parts.append(b"--" + b + b'\r\nContent-Disposition: form-data; name="' + name + b'"\r\n\r\n' + value + b"\r\n")
for i in range(10):
    content = bytes(rng.getrandbits(8) for _ in range(8192))
    parts.append(b"--" + b + b'\r\nContent-Disposition: form-data; name="docs[]"; filename="doc' + str(i).encode() + b'.bin"\r\nContent-Type: application/octet-stream\r\n\r\n' + content + b"\r\n")
open(sys.argv[1] + "/form.bin", "wb").write(b"".join(parts) + b"--" + b + b"--\r\n")
# large attachment: three fields and one 50 MiB file of random bytes
content = random.Random(9).randbytes(50 * 1024 * 1024)
head = b"".join(b"--" + b + b'\r\nContent-Disposition: form-data; name="f' + str(i).encode() + b'"\r\n\r\nvalue\r\n' for i in range(3))
open(sys.argv[1] + "/large.bin", "wb").write(head + b"--" + b + b'\r\nContent-Disposition: form-data; name="upload"; filename="big.bin"\r\nContent-Type: application/octet-stream\r\n\r\n' + content + b"\r\n--" + b + b"--\r\n")
open(sys.argv[1] + "/type.txt", "w").write("multipart/form-data; boundary=" + b.decode())
