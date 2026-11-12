
- The repository depends on `polyspec-binding-core` of the repository binding (`binding_core`, the Git dependency
  `https://github.com/polyspec/binding.git`, which `Cargo.lock` pins), the new name of bindgen; `make bindings` and
  `make bindings-check` take the cli of binding in `BINDING`, and the binding packages are those that `binding
  scaffold` writes. `Cargo.lock` takes the commit of `polyspec-http-media-type` on the same `binding_core`. The Python
  error classes are `HTTPForm<Category>Error`, the class form of the repository name in Python (design N-2: an acronym
  word is all capitals in Python and one capital word in PHP and Node, which keep `HttpForm<Category>Error`).
