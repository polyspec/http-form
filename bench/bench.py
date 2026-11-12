"""Times hyper's Python reader and the binding on the bodies of generate.py. Arguments: the directory that holds the
package polyspec.hyper of hyper (packages/server/python/src) and the directory of the bodies."""

import json, os, statistics, sys, time
sys.path.insert(0, sys.argv[1]); sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))
from polyspec.hyper._fields import Fields
from polyspec.hyper._multipart import Multipart
from polyspec.hyper._request_body import RequestBody as HyperBody
import http_forms as forms

kind = open(sys.argv[2] + "/type.txt").read()
boundary = kind.split("=", 1)[1]
text = open(sys.argv[2] + "/form.txt", "rb").read()
form = open(sys.argv[2] + "/form.bin", "rb").read()
large = open(sys.argv[2] + "/large.bin", "rb").read()
CHUNK = 65536
BIG = 1 << 40

def median(work, runs):
    times = []
    for _ in range(runs):
        start = time.perf_counter(); work(); times.append((time.perf_counter() - start) * 1000)
    return statistics.median(times)

def chunks(data):
    return [data[i:i + CHUNK] for i in range(0, len(data), CHUNK)]

def hyper_stream(data):
    body = HyperBody(kind, BIG, True, BIG, BIG, BIG)
    for c in chunks(data): body.feed(c)
    assert body.parts() is not None
    body.remove()

def core_stream(data):
    body = forms.RequestBody(kind, BIG, True, BIG, BIG, BIG)
    for c in chunks(data): body.feed(c)
    assert body.parts() is not None
    body.remove()

assert Fields.parse(text) is not None and forms.parse_urlencoded(text) is not None
out = {
  "urlencoded 1000 pairs": (median(lambda: Fields.parse(text), 300), median(lambda: forms.parse_urlencoded(text), 300)),
  "multipart form, whole": (median(lambda: Multipart.read(form, boundary), 300), median(lambda: forms.parse_multipart_chunks([form], boundary), 300)),
  "multipart form, streamed": (median(lambda: hyper_stream(form), 100), median(lambda: core_stream(form), 100)),
  "50 MiB attachment, streamed": (median(lambda: hyper_stream(large), 7), median(lambda: core_stream(large), 7)),
}
print(json.dumps({"python": out}))
