//! B-6: a binding places these types in memory of its runtime's allocator, which PHP and Python align to 16 bytes at
//! least; a type with a larger alignment would be read at a wrong address there.

const _: () = assert!(std::mem::align_of::<polyspec_http_forms::MultipartReader>() <= 16);
const _: () = assert!(std::mem::align_of::<polyspec_http_forms::RequestBody>() <= 16);
const _: () = assert!(std::mem::align_of::<polyspec_http_forms::UploadedFile>() <= 16);
const _: () = assert!(std::mem::align_of::<polyspec_http_forms::Form>() <= 16);
const _: () = assert!(std::mem::align_of::<polyspec_http_forms::Error>() <= 16);

#[test]
fn bound_types_need_at_most_16_bytes_of_alignment() {
    assert!(std::mem::align_of::<polyspec_http_forms::MultipartReader>() <= 16);
}
