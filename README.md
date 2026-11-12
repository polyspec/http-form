# http-forms

The form values and the `multipart/form-data` bodies of hyper, read by one Rust implementation that PHP, Python and
Node bind. A binding converts types; every rule is in the core, so the three servers run the same code and give the
same results.

```
core/          the Rust crate http-forms: every rule
php/           ext-php-rs extension, namespace Polyspec\Forms
python/        PyO3 module http_forms
node/          napi-rs addon
contracts/     hyper's contracts/fields.json
conformance/   the runners of the three bindings, their comparison and a differential test against hyper
spec/forms.md  the rules, the API and the error classes
```

## Use

```php
$body = new Polyspec\Forms\RequestBody($contentType, $limit, true, $valueLimit, $partLimit, $fileLimit);
while (!feof($input) && $body->feed(fread($input, 65536))) {}
[$fields, $files] = $body->parts() ?? throw new BadRequest();
```

```python
body = http_forms.RequestBody(content_type, limit, True, value_limit, part_limit, file_limit)
for chunk in chunks:
    if not body.feed(chunk):
        break
parts = body.parts()
```

```js
const body = new RequestBody(contentType, limit, true, valueLimit, partLimit, fileLimit);
for await (const chunk of request) if (!(await body.feed(chunk))) break;
const parts = body.parts();
```

## Checks

```
make test          # the cases in the core
make conformance   # the cases through the three bindings, compared with each other
make differential REFERENCE=<dir of polyspec/hyper>   # random bodies against hyper's Python reader
make check         # clippy, test and conformance
```

Requirements: Rust 1.85 or later, PHP 8.3 or later (checked with 8.3.6) with its development headers and libclang (for ext-php-rs),
Python 3.10 or later, Node 18 or later. Windows is not supported yet.
