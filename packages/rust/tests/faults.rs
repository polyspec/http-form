//! The fault injection points of a stored file (spec "Fault injection points", G-5): the points that only a file of
//! the directory of a request reaches, which a case of one call cannot make yet. Each test arms its points for the one
//! call under test, as the generated tests do, and requires every armed fault to fire.

mod common;

use std::fs;
use std::path::PathBuf;

use binding_core::fault;
use common::{errno_symbol, io};
use polyspec_http_form::{BodyLimits, Error, RequestBody, UploadedFile};
use serde_json::{json, Value};

const BODY: &[u8] =
    b"--XyZ\r\nContent-Disposition: form-data; name=\"f\"; filename=\"x\"\r\n\r\npayload\r\n--XyZ--\r\n";

/// A body read in chunks whose one file is stored in its directory, and that file.
fn stored() -> (RequestBody, UploadedFile) {
    let limits = BodyLimits { limit: 1000, value_limit: 1000, part_limit: 10, file_limit: 1000 };
    let mut read = match RequestBody::start(io(), Some(b"multipart/form-data; boundary=XyZ"), None, true, &limits) {
        Ok(read) => read,
        Err(error) => panic!("{error}"),
    };
    if let Err(error) = read.feed(io(), BODY).and_then(|()| read.finish(io())) {
        panic!("{error}");
    }
    let file = match read.parts() {
        Ok(form) => form.files[0].1[0].clone(),
        Err(error) => panic!("{error}"),
    };
    (read, file)
}

/// Runs `work` with `faults` armed for `member`, and requires every armed fault to fire.
fn armed<T>(member: &str, faults: Value, work: impl FnOnce() -> T) -> T {
    let id = match fault::arm(member, &faults.to_string()) {
        Ok(id) => id,
        Err(error) => panic!("{member}: {error}"),
    };
    let result = fault::take(member).run(work);
    assert_eq!(fault::left(id), Ok(Vec::<String>::new()), "{member}: every armed fault fires");
    result
}

/// The kind and the errno symbol of a failure.
fn failure<T: std::fmt::Debug>(result: Result<T, Error>) -> (String, Value) {
    match result {
        Ok(value) => panic!("the call gave {value:?}"),
        Err(error) => (error.kind().to_owned(), errno_symbol(error.errno())),
    }
}

/// A scratch directory of one test, removed first when an earlier run left it.
fn scratch(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("http-form-fault-{}-{label}", std::process::id()));
    match fs::remove_dir_all(&path) {
        Ok(()) => {}
        Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::NotFound, "{label}"),
    }
    if let Err(error) = fs::create_dir(&path) {
        panic!("{label}: {error}");
    }
    path
}

/// F-11, F-15: a read of a stored file that the system fails is `io` with its errno, and a short read is found by the
/// size of the file, `io` without errno; the file stays readable.
#[test]
fn a_read_of_a_stored_file() {
    let (mut read, file) = stored();
    let io_fault = json!([{"point": "http_form.read", "kind": "io", "errno": "EIO"}]);
    assert_eq!(failure(armed("t.read", io_fault, || file.content(io()))), ("io".to_owned(), json!("EIO")));
    let short = json!([{"point": "http_form.read", "kind": "short", "bytes": 3}]);
    assert_eq!(failure(armed("t.short", short, || file.content(io()))), ("io".to_owned(), Value::Null));
    assert_eq!(file.content(io()), Ok(b"payload".to_vec()));
    assert_eq!(read.remove(io()), Ok(()));
}

/// F-11: a rename that the system fails with another errno than `EXDEV` is the error of the move, `EEXIST` is
/// `exists`, and the file stays where it is.
#[test]
fn a_rename_of_a_stored_file() {
    let place = scratch("rename");
    let (mut read, file) = stored();
    let target = place.join("target");
    for (errno, expected) in [("EIO", ("io".to_owned(), json!("EIO"))), ("EEXIST", ("exists".to_owned(), Value::Null))]
    {
        let faults = json!([{"point": "http_form.rename", "kind": "io", "errno": errno}]);
        assert_eq!(failure(armed("t.rename", faults, || file.move_to(io(), &target))), expected, "{errno}");
        assert!(!target.exists(), "{errno}");
    }
    assert_eq!(file.content(io()), Ok(b"payload".to_vec()));
    assert_eq!(read.remove(io()), Ok(()));
    assert_eq!(fs::remove_dir_all(&place).map_err(|error| error.kind()), Ok(()));
}

/// F-11: after `EXDEV` of the rename, a failure of the open of the source, of the new target, of the copy, of a short
/// copy or of the write to the disk is the error of the move, the target that the copy created is removed, and the
/// source stays.
#[test]
fn a_copy_of_a_stored_file_that_fails() {
    let place = scratch("copy");
    let (mut read, file) = stored();
    let target = place.join("target");
    let exdev = json!({"point": "http_form.rename", "kind": "io", "errno": "EXDEV"});
    let steps = [
        (
            json!({"point": "http_form.open", "kind": "io", "errno": "EACCES"}),
            ("permission_denied".to_owned(), json!("EACCES")),
        ),
        (
            json!({"point": "http_form.create", "kind": "io", "errno": "ENOSPC"}),
            ("storage_full".to_owned(), json!("ENOSPC")),
        ),
        (json!({"point": "http_form.create", "kind": "io", "errno": "EEXIST"}), ("exists".to_owned(), Value::Null)),
        (json!({"point": "http_form.copy", "kind": "io", "errno": "EIO"}), ("io".to_owned(), json!("EIO"))),
        (json!({"point": "http_form.copy", "kind": "short", "bytes": 2}), ("io".to_owned(), Value::Null)),
        (json!({"point": "http_form.sync", "kind": "io", "errno": "EIO"}), ("io".to_owned(), json!("EIO"))),
    ];
    for (step, expected) in steps {
        let label = step.to_string();
        let result = failure(armed("t.copy", json!([exdev, step]), || file.move_to(io(), &target)));
        assert_eq!(result, expected, "{label}: the kind of the errno by the table of Errors");
        assert!(!target.exists(), "{label}: the target that the copy created is removed");
    }
    assert_eq!(file.content(io()), Ok(b"payload".to_vec()));
    assert_eq!(read.remove(io()), Ok(()));
    assert_eq!(fs::remove_dir_all(&place).map_err(|error| error.kind()), Ok(()));
}

/// F-11: a failed copy whose target the system does not remove is `partial_target`, with the errno of the copy and
/// the errno of the removal; the source stays.
#[test]
fn a_copy_whose_target_stays_is_partial_target() {
    let place = scratch("partial");
    let (mut read, file) = stored();
    let target = place.join("target");
    let faults = json!([
        {"point": "http_form.rename", "kind": "io", "errno": "EXDEV"},
        {"point": "http_form.copy", "kind": "io", "errno": "ENOSPC"},
        {"point": "http_form.remove", "kind": "io", "errno": "EACCES"},
    ]);
    let error = match armed("t.partial", faults, || file.move_to(io(), &target)) {
        Ok(()) => panic!("the file moved"),
        Err(error) => error,
    };
    assert_eq!((error.kind(), errno_symbol(error.errno())), ("partial_target", json!("ENOSPC")));
    assert_eq!(error.removal_kind(), Ok("permission_denied".to_owned()));
    let removal = match error.removal() {
        Some(removal) => errno_symbol(removal.errno()),
        None => panic!("partial_target has its removal"),
    };
    assert_eq!(removal, json!("EACCES"));
    assert!(target.exists(), "the target stays");
    assert_eq!(file.content(io()), Ok(b"payload".to_vec()));
    assert_eq!(read.remove(io()), Ok(()));
    assert_eq!(fs::remove_dir_all(&place).map_err(|error| error.kind()), Ok(()));
}

/// F-11: a copy whose source the system does not remove is `source_left` with the errno of the removal, and the file
/// counts as moved.
#[test]
fn a_copy_whose_source_stays_is_source_left() {
    let place = scratch("source-left");
    let (mut read, file) = stored();
    let target = place.join("target");
    let faults = json!([
        {"point": "http_form.rename", "kind": "io", "errno": "EXDEV"},
        {"point": "http_form.remove", "kind": "io", "errno": "EACCES"},
    ]);
    let result = armed("t.source", faults, || file.move_to(io(), &target));
    assert_eq!(failure(result), ("source_left".to_owned(), json!("EACCES")));
    assert_eq!(fs::read(&target).map_err(|error| error.kind()), Ok(b"payload".to_vec()));
    assert_eq!(file.content(io()), Err(Error::Moved));
    assert_eq!(read.remove(io()), Ok(()));
    assert_eq!(fs::remove_dir_all(&place).map_err(|error| error.kind()), Ok(()));
}

/// X-1, H-8: the point `internal` makes the lock of a file a poisoned one in a move as in a read.
#[test]
fn a_broken_lock_fails_a_move() {
    let place = scratch("internal");
    let (mut read, file) = stored();
    let target = place.join("target");
    let faults = json!([{"point": "http_form.internal", "kind": "broken"}]);
    assert_eq!(
        failure(armed("t.internal", faults, || file.move_to(io(), &target))),
        ("internal".to_owned(), Value::Null)
    );
    assert!(!target.exists());
    assert_eq!(read.remove(io()), Ok(()));
    assert_eq!(fs::remove_dir_all(&place).map_err(|error| error.kind()), Ok(()));
}
