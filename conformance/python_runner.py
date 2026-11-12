"""Runs contracts/fields.json through the Python binding as hyper's runners do and prints the actual results as JSON.

A urlencoded text is the UTF-8 encoding of its JSON string; a multipart type and body are text whose characters are
their bytes. The runner holds no rule: it calls the binding and records what it returns."""

import json
import os
import shutil
import sys
import tempfile

import http_forms as forms


def text(data: bytes) -> str:
    return data.decode("latin-1")


def fields_of(pairs):
    return None if pairs is None else [[name, list(values)] for name, values in pairs]


def files_of(pairs, size=False):
    if pairs is None:
        return None
    out = []
    for _, items in pairs:
        for item in items:
            entry = {
                "name": item.name(),
                "filename": item.filename(),
                "contentType": item.content_type(),
                "content": text(item.content()),
            }
            if size:
                entry["size"] = item.size()
            out.append(entry)
    return out


def failure(work):
    try:
        work()
        return None
    except TypeError as error:
        return (str(error), "type")
    except ValueError as error:
        return (str(error), "value")


def urlencoded(case):
    pairs = forms.parse_urlencoded(case["text"].encode("utf-8"))
    return {"fields": fields_of(pairs), "names": None if pairs is None else [name for name, _ in pairs]}


def multipart(case):
    kind = case["type"]
    body = case["body"].encode("latin-1")
    whole = [
        fields_of(forms.fields_from_body(kind, body)),
        files_of(forms.files_from_body(kind, body)),
    ]
    result = whole
    boundary = forms.multipart_boundary(kind)
    if boundary is not None:
        for at in range(len(body) + 1):
            parts = forms.parse_multipart_chunks([body[:at], body[at:]], boundary)
            divided = [fields_of(None if parts is None else parts[0]), files_of(None if parts is None else parts[1])]
            if divided != whole:
                result = divided
                break
    return {"fields": result[0], "files": result[1]}


def streamed(case):
    kind = case["type"]
    body = case["body"].encode("latin-1")
    chunk = case["chunk"]
    read = forms.RequestBody(
        kind, case["limit"], case["stream"] is True, case["valueLimit"], case["partLimit"], case["fileLimit"]
    )
    scratch = tempfile.mkdtemp(prefix="http-forms-case-")
    out = {}
    try:
        stopped = False
        at = 0
        while at < len(body) and not stopped:
            stopped = not read.feed(body[at : at + chunk])
            at += chunk
        held = read.held()
        if read.streamed():
            parts = read.parts()
            fields = None if parts is None else parts[0]
            files = None if parts is None else parts[1]
        else:
            fields = None if stopped else forms.fields_from_body(kind, held)
            files = None if stopped else forms.files_from_body(kind, held)
        directory = read.directory()

        def count():
            return None if directory is None else len(os.listdir(directory))

        out["stopped"] = stopped
        out["exceeded"] = read.exceeded()
        out["held"] = text(held)
        out["fields"] = fields_of(fields)
        out["files"] = files_of(files, size=True)
        out["entries"] = count()
        pairs = [] if files is None else files
        move = case["move"]
        if move == "none" or not pairs:
            out["moved"] = None
        else:
            first = pairs[0][1][0]
            target = os.path.join(scratch, "target")
            if move == "exists":
                with open(target, "wb") as taken:
                    taken.write(b"taken")
            failed = failure(lambda: first.move_to(7 if move == "type" else target))
            after = None
            if failed is None:
                with open(target, "rb") as moved:
                    content = text(moved.read())
                failed = failure(lambda: first.move_to(os.path.join(scratch, "second")))
                read_failure = failure(first.content)
                after = None if read_failure is None else read_failure[0]
            else:
                content = text(first.content())
            out["moved"] = {
                "content": content,
                "left": count(),
                "again": None if failed is None else failed[0],
                "read": after,
                "class": None if failed is None else failed[1],
            }
        read.remove()
        out["removed"] = None if directory is None else not os.path.isdir(directory)
    finally:
        read.remove()
        shutil.rmtree(scratch)
    return out


def main():
    cases = json.load(open(sys.argv[1], encoding="utf-8"))
    run = {"urlencoded": urlencoded, "multipart": multipart, "streamed": streamed}
    print(json.dumps({group: {case["id"]: run[group](case) for case in cases[group]} for group in run}, sort_keys=True))


if __name__ == "__main__":
    main()
