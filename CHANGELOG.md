
- The repository is http-form: a repository is named after what it handles, in the singular, with the prefix `http-`
  for a part of an HTTP message (plan N-1, N-2). Every name follows it (design P-1, E-2): the crate `polyspec-http-form`,
  the Composer package `polyspec/http-form` with the namespace `Polyspec\HttpForm` and the extension
  `polyspec_http_form`, the PyPI package `polyspec-http-form` with the module `polyspec.http_form`, the npm package
  `@polyspec/http-form` and the error classes `HttpForm<Category>Error`; the directory of a request starts with
  `http-form-` (F-10), and the test settings are `HTTP_FORM_ROUNDS` and `HTTP_FORM_OTHER_FS`. The contract records the
  change in `changes`. The hand-written bindings under `legacy/` keep their names.
