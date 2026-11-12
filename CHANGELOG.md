
- K-1, G-5: the cases follow the case convention of binding: the keys are the parameter names of the member
  (`content_type`, `content_length`, `limits`, `chunk`), bytes are `{"hex"}`, and every public member names a group of
  its cases. `forms.json` has the groups `request_body.new`, `request_body.feed`, `request_body.finish`,
  `request_body.parts` and `request_body.properties`, which take a body from the keys of the constructor, and `boundary`;
  the scenarios of more than one call stay in `body`, which only the tests of the core read. New cases raise
  `malformed_content_type` and `no_boundary` of `fieldsFromBody`, `filesFromBody` and `readHeldBody`, `invalid` and
  `header_limit` of `readHeldBody`, `no_boundary` of `multipartBoundary`, `file_limit` of `feed`, `invalid` of
  `finish` and `not_finished` and `not_streamed` of `parts`. A kind that only a sequence of calls reaches (`finished`
  and `removed` of `feed`, the failures of a fed body at `finish` and `parts`, `moved`, `removed` and `exists` of an
  uploaded file) has no case of its member yet. The legacy runners read the earlier form from
  `legacy/conformance/fields.json`.
