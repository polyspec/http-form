//! Rules that a contract case cannot hold: a failing sink, the order of the calls on a body, its directory, a move to
//! another file system, and the independence of the chunks on random bodies with limits.

use std::fs;
use std::path::PathBuf;

use bindgen_core::Io;
use polyspec_http_forms::{
    BodyLimits, Collector, Error, Files, Limits, MultipartReader, Piece, ReadError, RequestBody, UploadedFile,
};

/// The wait token of one call that does blocking I/O (spec F-17), as generated code makes it.
fn io() -> Io {
    Io::for_generated_call()
}

const TYPE: &[u8] = b"multipart/form-data; boundary=X";
const BIG: u64 = 1 << 20;
const LIMITS: BodyLimits = BodyLimits { limit: BIG, value_limit: BIG, part_limit: BIG, file_limit: BIG };

/// The last file of a name (HY-113.2).
fn last<'a>(files: &'a Files, name: &str) -> Option<&'a UploadedFile> {
    files.iter().find(|(known, _)| known == name).and_then(|(_, items)| items.last())
}

fn body(content_type: &[u8], stream: bool) -> RequestBody {
    match RequestBody::new(io(), Some(content_type), None, stream, LIMITS) {
        Ok(body) => body,
        Err(error) => panic!("{error}"),
    }
}

fn directory(body: &RequestBody) -> PathBuf {
    match body.directory() {
        Some(directory) => directory,
        None => panic!("a streamed body has a directory"),
    }
}

/// H-6: a failure of the sink is its own state, and not `invalid`.
#[test]
fn a_failing_sink_stops_the_reader() {
    let mut reader = MultipartReader::new(b"X", Limits::default());
    let data = b"--X\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n1\r\n--X--";
    let first = reader.feed_with(data, |_| Err::<(), &str>("full"));
    assert_eq!(first, Err(ReadError::Sink("full")));
    assert_eq!(reader.feed_with(b"", |_| Ok::<(), &str>(())), Err(ReadError::SinkFailed));
    assert_eq!(reader.finish(), Err(ReadError::SinkFailed));
}

/// H-8: pieces out of the order of a reader are `internal`.
#[test]
fn pieces_out_of_order_are_internal() {
    let mut form = Collector::default();
    assert!(matches!(form.take(Piece::FileData(b"x")), Err(Error::Internal(_))));
    assert!(matches!(form.take(Piece::FileEnd), Err(Error::Internal(_))));
    let start = || Piece::FileStart { name: "f".into(), filename: "f".into(), content_type: String::new() };
    assert_eq!(form.take(start()), Ok(()));
    assert!(matches!(form.take(start()), Err(Error::Internal(_))));
    assert!(matches!(form.into_form(), Err(Error::Internal(_))), "a file that did not end");
}

/// H-4: the order of the calls on a body.
#[test]
fn a_body_ends_once() {
    let mut read = body(TYPE, true);
    assert_eq!(read.feed(io(), b"--X\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n1\r\n--X--"), Ok(()));
    assert_eq!(read.parts().map(|form| form.fields.len()), Err(Error::NotFinished));
    assert_eq!(read.finish(io()), Ok(()));
    assert_eq!(read.finish(io()), Ok(()));
    assert_eq!(read.feed(io(), b"x"), Err(Error::Finished));
    assert_eq!(read.parts().map(|form| form.fields.len()), Ok(1));
    assert_eq!(read.remove(io()), Ok(()));
}

/// H-20: the directory of a request is removed once; a directory that is gone is an error; a dropped body removes it.
#[test]
fn the_directory_of_a_request() {
    let mut read = body(TYPE, true);
    let path = directory(&read);
    assert!(path.is_dir());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = match fs::metadata(&path) {
            Ok(metadata) => metadata.permissions().mode(),
            Err(error) => panic!("{error}"),
        };
        assert_eq!(mode & 0o777, 0o700, "HY-131.13: for the user of the server only");
    }
    assert_eq!(read.remove(io()), Ok(()));
    assert!(!path.exists());
    assert_eq!(read.remove(io()), Ok(()));
    assert_eq!(read.feed(io(), b"--X"), Err(Error::Removed));

    let mut gone = body(TYPE, true);
    let path = directory(&gone);
    if let Err(error) = fs::remove_dir(&path) {
        panic!("{error}");
    }
    let error = gone.remove(io());
    assert_eq!(error.as_ref().map_err(Error::kind), Err("not_found"));

    let dropped = body(TYPE, true);
    let path = directory(&dropped);
    drop(dropped);
    assert!(!path.exists(), "B-10: a body without remove() removes its directory when it is dropped");

    let held = body(b"application/x-www-form-urlencoded", true);
    assert!(held.directory().is_none());
}

/// H-9: a move to another file system copies, writes to the disk and removes the source. The other file system is
/// `HTTP_FORMS_OTHER_FS`, or `/dev/shm`; the test fails when it is on the file system of the temporary directory.
#[cfg(target_os = "linux")]
#[test]
fn a_move_to_another_file_system_copies() {
    use std::os::unix::fs::MetadataExt;
    let other = PathBuf::from(match std::env::var_os("HTTP_FORMS_OTHER_FS") {
        Some(path) => path,
        None => "/dev/shm".into(),
    });
    let device = |path: &std::path::Path| match fs::metadata(path) {
        Ok(metadata) => metadata.dev(),
        Err(error) => panic!("{}: {error}", path.display()),
    };
    assert_ne!(device(&other), device(&std::env::temp_dir()), "{} is on the temporary file system", other.display());
    let mut read = body(TYPE, true);
    let data = b"--X\r\nContent-Disposition: form-data; name=\"f\"; filename=\"a\"\r\n\r\npayload\r\n--X--";
    assert_eq!(read.feed(io(), data), Ok(()));
    assert_eq!(read.finish(io()), Ok(()));
    let form = match read.parts() {
        Ok(form) => form,
        Err(error) => panic!("{error}"),
    };
    let Some(file) = last(&form.files, "f") else { panic!("no file") };
    let target = other.join(format!("http-forms-move-{}", std::process::id()));
    assert_eq!(file.move_to(io(), &target), Ok(()));
    assert_eq!(fs::read(&target).map_err(|error| error.kind()), Ok(b"payload".to_vec()));
    match fs::read_dir(directory(&read)) {
        Ok(list) => assert_eq!(list.count(), 0, "the source was removed"),
        Err(error) => panic!("{error}"),
    }
    assert_eq!(file.move_to(io(), &target), Err(Error::Moved));
    let again = last(&form.files, "f").map(|file| file.content(io()));
    assert_eq!(again, Some(Err(Error::Moved)));
    if let Err(error) = fs::remove_file(&target) {
        panic!("{error}");
    }
    let mut taken = body(TYPE, true);
    assert_eq!(taken.feed(io(), data), Ok(()));
    assert_eq!(taken.finish(io()), Ok(()));
    let form = match taken.parts() {
        Ok(form) => form,
        Err(error) => panic!("{error}"),
    };
    let Some(file) = last(&form.files, "f") else { panic!("no file") };
    if let Err(error) = fs::write(&target, b"taken") {
        panic!("{error}");
    }
    assert_eq!(file.move_to(io(), &target), Err(Error::Exists), "EEXIST of the copy is exists");
    assert_eq!(fs::read(&target).map_err(|error| error.kind()), Ok(b"taken".to_vec()));
    if let Err(error) = fs::remove_file(&target) {
        panic!("{error}");
    }
    assert_eq!(read.remove(io()), Ok(()));
    assert_eq!(taken.remove(io()), Ok(()));
}

/// A small generator of random numbers, so that the test needs no dependency and repeats.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, bound: usize) -> usize {
        let next = self.next();
        match (u64::try_from(bound.max(1)), usize::try_from(next)) {
            (Ok(_), Ok(next)) => next % bound.max(1),
            (Err(error), _) => panic!("{error}"),
            (_, Err(error)) => panic!("{error}"),
        }
    }
    fn pick<'a>(&mut self, list: &[&'a [u8]]) -> &'a [u8] {
        list[self.below(list.len())]
    }
}

const PIECES: &[&[u8]] = &[
    b"--X",
    b"--X--",
    b"\r\n",
    b"\r",
    b"\n",
    b"--",
    b"-",
    b"X",
    b"Content-Disposition: ",
    b"form-data",
    b"; ",
    b"name=",
    b"name=\"a\"",
    b"filename=\"f\"",
    b"\"",
    b"\xff",
    b"\xc3\xa9",
    b"\xe2\x80",
    b"value",
    b"\r\n\r\n",
    b"--X\r\n",
    b"\r\n--X",
    b"\r\n--",
    b"\x00",
];

const HEADERS: &[&[u8]] = &[
    b"Content-Disposition: form-data; name=\"a\"",
    b"Content-Disposition: form-data; name=\"f\"; filename=\"x.txt\"\r\nContent-Type: text/plain",
    b"Content-Disposition: form-data; name=b",
    b"Content-Disposition: form-data; name=\"q",
    b"X-Other: 1",
];

fn random_body(random: &mut Random) -> Vec<u8> {
    let mut data = Vec::new();
    if random.below(2) == 0 {
        for _ in 0..random.below(5) {
            data.extend_from_slice(b"--X\r\n");
            data.extend_from_slice(random.pick(HEADERS));
            data.extend_from_slice(b"\r\n\r\n");
            for _ in 0..random.below(6) {
                data.extend_from_slice(random.pick(PIECES));
            }
            data.extend_from_slice(b"\r\n");
        }
        data.extend_from_slice(random.pick(&[b"--X--\r\n", b"--X--", b"--X", b"", b"--Xy"]));
    } else {
        for _ in 0..random.below(30) {
            data.extend_from_slice(random.pick(PIECES));
        }
    }
    data
}

/// The result of a body read in the chunks: the stage and the kind of its first failure, or its values and files.
fn outcome(chunks: &[&[u8]], limits: (u64, u64, u64, u64)) -> String {
    let limits = BodyLimits { limit: limits.0, value_limit: limits.1, part_limit: limits.2, file_limit: limits.3 };
    let mut read = match RequestBody::new(io(), Some(TYPE), None, true, limits) {
        Ok(read) => read,
        Err(error) => return format!("new {}", error.kind()),
    };
    let mut result = None;
    for chunk in chunks {
        if let Err(error) = read.feed(io(), chunk) {
            result = Some(format!("feed {}", error.kind()));
            break;
        }
    }
    let result = match result {
        Some(result) => result,
        None => match read.finish(io()) {
            Err(error) => format!("finish {}", error.kind()),
            Ok(()) => match read.parts() {
                Ok(form) => {
                    let files: Vec<(String, Result<Vec<u8>, Error>)> = form
                        .files
                        .iter()
                        .flat_map(|(_, files)| files.iter().map(|file| (file.filename(), file.content(io()))))
                        .collect();
                    format!("{:?} {files:?}", form.fields)
                }
                Err(error) => format!("parts {}", error.kind()),
            },
        },
    };
    if let Err(error) = read.remove(io()) {
        panic!("{error}");
    }
    result
}

/// HY-131.6, H-5: on random bodies and limits, every division of a body gives the result of the body read byte by
/// byte, the first failure included.
#[test]
fn random_bodies_do_not_depend_on_their_chunks() {
    let mut random = Random(0x9e37_79b9_7f4a_7c15);
    let rounds = match std::env::var("HTTP_FORMS_ROUNDS").map(|text| text.parse::<usize>()) {
        Ok(Ok(rounds)) => rounds,
        Ok(Err(error)) => panic!("HTTP_FORMS_ROUNDS: {error}"),
        Err(_) => 3000,
    };
    let mut failed = 0;
    for _ in 0..rounds {
        let data = random_body(&mut random);
        let length = u64::try_from(data.len()).unwrap_or(u64::MAX);
        let mut limit =
            || if random.below(3) == 0 { u64::try_from(random.below(data.len() + 2)).unwrap_or(0) } else { BIG };
        let limits = (limit(), limit(), limit(), limit());
        let bytes: Vec<&[u8]> = data.chunks(1).collect();
        let expected = outcome(&bytes, limits);
        let whole = outcome(&[&data], limits);
        assert_eq!(whole, expected, "{:?} {limits:?} (length {length}): one chunk", data.escape_ascii().to_string());
        let mut cuts: Vec<usize> = (0..random.below(4)).map(|_| random.below(data.len() + 1)).collect();
        cuts.sort_unstable();
        let mut chunks = Vec::new();
        let mut at = 0;
        for cut in cuts {
            chunks.push(&data[at..cut]);
            at = cut;
        }
        chunks.push(&data[at..]);
        let divided = outcome(&chunks, limits);
        assert_eq!(divided, expected, "{:?} {limits:?} cut into {}", data.escape_ascii().to_string(), chunks.len());
        if expected.starts_with("feed") || expected.starts_with("finish") {
            failed += 1;
        }
    }
    assert!(failed > rounds / 10, "the random bodies fail too seldom: {failed}");
}
