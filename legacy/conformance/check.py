"""Compares the results that each runner printed with the expectations of contracts/fields.json, and the runners with
each other: every runner must give every expected key of every case, and all runners the same results."""

import json
import sys

cases = json.load(open(sys.argv[1], encoding="utf-8"))
results = {}
for argument in sys.argv[2:]:
    name, path = argument.split("=", 1)
    results[name] = json.load(open(path, encoding="utf-8"))
failures = []
checked = 0
for group, items in cases.items():
    for case in items:
        expected = {key: value for key, value in case.items() if key not in {"id", "label", "text", "type", "body", "chunk", "limit", "valueLimit", "partLimit", "fileLimit", "stream", "move"}}
        for name, result in results.items():
            actual = result[group][case["id"]]
            for key, value in expected.items():
                checked += 1
                if actual.get(key) != value:
                    failures.append(f"{name} {group}/{case['id']}: {key}: expected {value!r}, got {actual.get(key)!r}")
        first = None
        for name, result in results.items():
            actual = {key: result[group][case["id"]].get(key) for key in expected}
            if first is None:
                first = (name, actual)
            elif actual != first[1]:
                failures.append(f"{group}/{case['id']}: {name} differs from {first[0]}")
total = sum(len(items) for items in cases.values())
print(f"{total} cases, {len(results)} runners ({', '.join(results)}), {checked} comparisons, {len(failures)} failures")
for failure in failures:
    print(failure)
sys.exit(1 if failures or checked == 0 else 0)
