//! The state machine that reads a `multipart/form-data` body in chunks (HY-131) and the rules of its parts
//! (HY-113).
//!
//! The reader finds the delimiters and gives the header lines and the content of each part to its sink. Besides one
//! chunk it holds at most the delimiter or the header lines of one part, and it reads a chunk in place: the data of a
//! file reaches the sink as a slice of the chunk.
//!
//! The result does not depend on the chunks (HY-131.6), also when the body fails: every failure is found at the byte
//! that reveals it, the first byte after which the bytes read so far determine it, and when one chunk reveals more than
//! one failure the reader gives the one whose byte comes first (spec F-8). A byte that may start a delimiter is held
//! until a later byte shows whether it does, so it counts for a limit, and for the UTF-8 of a value, at that later
//! byte.

use std::convert::Infallible;

use memchr::memmem::Finder;

use crate::header::{parameter, part_header, PartHeader};
use crate::Error;

/// The largest header of a part in bytes (HY-113.16), as the contract type `uint` (design T-1).
pub const HEADER_LIMIT: u64 = 8192;

/// `HEADER_LIMIT` as an index of the bytes that the reader holds.
const HEADER_BYTES: usize = 8192;
const _: () = assert!(HEADER_LIMIT == 8192 && HEADER_BYTES == 8192);

/// The empty line that ends the header lines of a part.
const HEADER_END: &[u8] = b"\r\n\r\n";

/// The limits of a body read in chunks; `None` is no limit (HY-131.7 to HY-131.12).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Limits {
    /// The largest number of bytes of the names and the contents of the text values together.
    pub value: Option<u64>,
    /// The largest number of parts, also of parts that are neither a value nor a file.
    pub parts: Option<u64>,
    /// The largest number of bytes of one file.
    pub file: Option<u64>,
}

/// What the reader found, as it gives it to a sink: the data of a file is a slice of the chunk.
#[derive(Debug, PartialEq, Eq)]
pub enum Piece<'a> {
    /// A text value, whole.
    Field { name: String, value: String },
    /// The start of a file part.
    FileStart { name: String, filename: String, content_type: String },
    /// The next bytes of the file that started last.
    FileData(&'a [u8]),
    /// The end of the file that started last.
    FileEnd,
}

/// What the reader found, as owned values (HY-131.13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Field { name: String, value: String },
    FileStart { name: String, filename: String, content_type: String },
    FileData(Vec<u8>),
    FileEnd,
}

impl From<Piece<'_>> for Event {
    fn from(piece: Piece<'_>) -> Self {
        match piece {
            Piece::Field { name, value } => Event::Field { name, value },
            Piece::FileStart { name, filename, content_type } => Event::FileStart { name, filename, content_type },
            Piece::FileData(bytes) => Event::FileData(bytes.to_vec()),
            Piece::FileEnd => Event::FileEnd,
        }
    }
}

/// A failure of the body or of the sink that received a piece.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadError<E> {
    /// The body failed; every later call gives the same error.
    Form(Error),
    /// The sink failed with this error.
    Sink(E),
    /// The sink failed in an earlier call, which gave its error; the reader reads nothing more (H-6).
    SinkFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Preamble,
    Delimiter,
    Headers,
    Content,
    Done,
    Failed,
}

/// Why the reader stopped.
#[derive(Debug, Clone)]
enum Stopped {
    Form(Error),
    Sink,
}

/// What the part that is read is.
enum Part {
    /// No part: before the header lines of a part are read (F-16: every part is a value or a file).
    None,
    /// A text value, and the length of its content that is known to be the start of UTF-8 text.
    Value { name: String, content: Vec<u8>, valid: usize },
    File { size: u64 },
}

/// A number of bytes as `u64`, which holds every `usize` of the supported targets (H-22).
pub(crate) fn count(length: usize) -> Result<u64, Error> {
    u64::try_from(length).map_err(|_| Error::Internal("a length does not fit 64 bits"))
}

/// Reads a `multipart/form-data` body in chunks.
pub struct MultipartReader {
    delimiter: Vec<u8>,
    // The searchers hold SIMD values with an alignment above 16 bytes, which the allocator of a runtime that holds
    // the reader, such as PHP's or Python's, need not give; a box takes them from the Rust allocator (plan B-6).
    separator: Box<Finder<'static>>,
    header_end: Box<Finder<'static>>,
    buffer: Vec<u8>,
    state: State,
    first: bool,
    limits: Limits,
    value_bytes: u64,
    part_count: u64,
    part: Part,
    stopped: Option<Stopped>,
}

impl MultipartReader {
    /// A reader of a body with the boundary (HY-113.11) and the limits.
    pub fn new(boundary: &[u8], limits: Limits) -> Self {
        let mut delimiter = b"--".to_vec();
        delimiter.extend_from_slice(boundary);
        let mut separator = b"\r\n".to_vec();
        separator.extend_from_slice(&delimiter);
        MultipartReader {
            delimiter,
            separator: Box::new(Finder::new(&separator).into_owned()),
            header_end: Box::new(Finder::new(HEADER_END).into_owned()),
            buffer: Vec::new(),
            state: State::Preamble,
            first: true,
            limits,
            value_bytes: 0,
            part_count: 0,
            part: Part::None,
            stopped: None,
        }
    }

    /// Reads the next chunk and returns what it found as owned values. After an error every call returns that error,
    /// and after the `--` that ends the body the rest is ignored (HY-131.4, HY-131.5).
    pub fn feed(&mut self, chunk: &[u8]) -> Result<Vec<Event>, Error> {
        let mut events = Vec::new();
        let result = self.feed_with(chunk, |piece| {
            events.push(Event::from(piece));
            Ok::<(), Infallible>(())
        });
        match result {
            Ok(()) => Ok(events),
            Err(ReadError::Form(error)) => Err(error),
            Err(ReadError::Sink(never)) => match never {},
            Err(ReadError::SinkFailed) => Err(Error::Internal("a sink that cannot fail failed")),
        }
    }

    /// Reads the next chunk and gives each piece to the sink as it finds it. A failure of the sink stops the reader,
    /// which then gives `SinkFailed` (H-6); a failure of the body is the error of every later call (HY-131.11).
    pub fn feed_with<E>(
        &mut self,
        chunk: &[u8],
        mut sink: impl FnMut(Piece<'_>) -> Result<(), E>,
    ) -> Result<(), ReadError<E>> {
        match &self.stopped {
            Some(Stopped::Form(error)) => return Err(ReadError::Form(error.clone())),
            Some(Stopped::Sink) => return Err(ReadError::SinkFailed),
            None => {}
        }
        // HY-131.4: the bytes after the `--` that ends the body are an ignored input (spec I-2).
        if self.state == State::Done {
            return Ok(());
        }
        let result = if self.buffer.is_empty() {
            self.run(chunk, &mut sink).map(|read| self.buffer.extend_from_slice(&chunk[read..]))
        } else {
            let mut joined = std::mem::take(&mut self.buffer);
            joined.extend_from_slice(chunk);
            self.run(&joined, &mut sink).map(|read| {
                joined.drain(..read);
                self.buffer = joined;
            })
        };
        if let Err(error) = &result {
            self.state = State::Failed;
            self.buffer = Vec::new();
            self.part = Part::None;
            self.stopped = Some(match error {
                ReadError::Form(error) => Stopped::Form(error.clone()),
                ReadError::Sink(_) | ReadError::SinkFailed => Stopped::Sink,
            });
        }
        result
    }

    /// Ends the body: a body that did not end with the `--` after a delimiter fails (HY-131.5).
    pub fn finish(&mut self) -> Result<(), ReadError<Infallible>> {
        match &self.stopped {
            Some(Stopped::Form(error)) => return Err(ReadError::Form(error.clone())),
            Some(Stopped::Sink) => return Err(ReadError::SinkFailed),
            None => {}
        }
        if self.state != State::Done {
            self.state = State::Failed;
            self.buffer = Vec::new();
            self.part = Part::None;
            self.stopped = Some(Stopped::Form(Error::Invalid));
            return Err(ReadError::Form(Error::Invalid));
        }
        Ok(())
    }

    /// True when the body ended with the `--` after a delimiter (HY-131.4).
    pub fn closed(&self) -> bool {
        self.state == State::Done
    }

    /// The number of bytes that the reader holds between two chunks.
    pub fn buffered(&self) -> usize {
        self.buffer.len()
    }

    /// The number of parts that were read, also those that are neither a value nor a file (HY-131.12).
    pub fn parts(&self) -> u64 {
        self.part_count
    }

    /// Reads the bytes in place and returns how many it read; the rest is held for the next chunk.
    fn run<E>(
        &mut self,
        data: &[u8],
        sink: &mut impl FnMut(Piece<'_>) -> Result<(), E>,
    ) -> Result<usize, ReadError<E>> {
        let separator = self.separator.needle().len();
        let mut at = 0;
        loop {
            let rest = &data[at..];
            let length = rest.len();
            match self.state {
                State::Preamble => {
                    if self.first {
                        if length < self.delimiter.len() {
                            if self.delimiter.starts_with(rest) {
                                return Ok(at);
                            }
                        } else if rest.starts_with(&self.delimiter) {
                            self.first = false;
                            at += self.delimiter.len();
                            self.state = State::Delimiter;
                            continue;
                        }
                        self.first = false;
                    }
                    if let Some(index) = self.separator.find(rest) {
                        at += index + separator;
                        self.state = State::Delimiter;
                        continue;
                    }
                    // HY-131.3: the bytes before the first delimiter are an ignored input (spec I-1).
                    return Ok(at + length - pending(rest, self.separator.needle()));
                }
                State::Delimiter => {
                    // HY-131.4, HY-131.5: `--` or CRLF; any other first byte fails at once.
                    match rest.first() {
                        None => return Ok(at),
                        Some(b'-' | b'\r') => {}
                        Some(_) => return Err(ReadError::Form(Error::Invalid)),
                    }
                    if length < 2 {
                        return Ok(at);
                    }
                    match &rest[..2] {
                        b"--" => {
                            self.state = State::Done;
                            return Ok(data.len());
                        }
                        b"\r\n" => {
                            at += 2;
                            self.state = State::Headers;
                            // HY-131.12: a part counts from the CRLF after its delimiter.
                            self.part_count =
                                self.part_count.checked_add(1).ok_or(ReadError::Form(Error::Internal("parts")))?;
                            if let Some(limit) = self.limits.parts {
                                if self.part_count > limit {
                                    return Err(ReadError::Form(Error::PartLimit));
                                }
                            }
                        }
                        _ => return Err(ReadError::Form(Error::Invalid)),
                    }
                }
                State::Headers => {
                    if length < 2 {
                        return Ok(at);
                    }
                    let empty = rest.starts_with(b"\r\n");
                    let end = if empty { Some(0) } else { self.header_end.find(rest) };
                    let Some(end) = end else {
                        return if header_too_long(rest) { Err(ReadError::Form(Error::HeaderLimit)) } else { Ok(at) };
                    };
                    if end > HEADER_BYTES {
                        return Err(ReadError::Form(Error::HeaderLimit));
                    }
                    let lines: Vec<&[u8]> = if empty { Vec::new() } else { split_lines(&rest[..end]) };
                    at += if empty { 2 } else { end + HEADER_END.len() };
                    self.state = State::Content;
                    self.part_started(&lines, sink)?;
                }
                State::Content => {
                    if let Some(index) = self.separator.find(rest) {
                        self.part_data(rest, index, sink)?;
                        self.part_ended(sink)?;
                        at += index + separator;
                        self.state = State::Delimiter;
                        continue;
                    }
                    let passed = length - pending(rest, self.separator.needle());
                    self.part_data(rest, passed, sink)?;
                    return Ok(at + passed);
                }
                State::Done | State::Failed => return Ok(data.len()),
            }
        }
    }

    fn part_started<E>(
        &mut self,
        lines: &[&[u8]],
        sink: &mut impl FnMut(Piece<'_>) -> Result<(), E>,
    ) -> Result<(), ReadError<E>> {
        self.part = Part::None;
        // F-16: a part has one `Content-Disposition` of the type `form-data` with a `name`, at most one
        // `Content-Type` and no line without `:`, or it fails the body.
        let parsed = part_header(lines);
        // H-5: the name of a value counts for the value limit at the end of the header lines, the byte that also
        // reveals any failure of HY-42 of the header lines; at one byte a limit comes before HY-42.
        if let Ok(header) = &parsed {
            if parameter(&header.parameters, b"filename").is_none() {
                self.add_value_bytes(count(header.name.len()).map_err(ReadError::Form)?)?;
            }
        }
        // HY-131.5: a header line that starts with the delimiter fails the body.
        if lines.iter().any(|line| line.starts_with(&self.delimiter)) {
            return Err(ReadError::Form(Error::Invalid));
        }
        let PartHeader { name: raw_name, parameters: found, content_type } = parsed.map_err(ReadError::Form)?;
        let name = utf8(&raw_name).ok_or(ReadError::Form(Error::Invalid))?;
        // HY-113.7: `filename*` is not read; it is an ignored input (spec I-4).
        match parameter(&found, b"filename") {
            Some(filename) => {
                let content_type = match content_type {
                    Some(value) => value,
                    None => b"",
                };
                // HY-113.5: the filename and the content type of a file are UTF-8.
                let (Some(filename), Some(content_type)) = (utf8(filename), utf8(content_type)) else {
                    return Err(ReadError::Form(Error::Invalid));
                };
                self.part = Part::File { size: 0 };
                sink(Piece::FileStart { name, filename, content_type }).map_err(ReadError::Sink)
            }
            None => {
                self.part = Part::Value { name, content: Vec::new(), valid: 0 };
                Ok(())
            }
        }
    }

    /// Takes `rest[..end]` as content of the part. `rest` is the rest of the bytes of the call, so that the byte that
    /// reveals a failure among them can be found.
    fn part_data<E>(
        &mut self,
        rest: &[u8],
        end: usize,
        sink: &mut impl FnMut(Piece<'_>) -> Result<(), E>,
    ) -> Result<(), ReadError<E>> {
        if end == 0 {
            return Ok(());
        }
        let data = &rest[..end];
        let length = count(data.len()).map_err(ReadError::Form)?;
        let needle = self.separator.needle();
        match &mut self.part {
            // F-16: content is read only after the header lines of a value or a file.
            Part::None => Err(ReadError::Form(Error::Internal("content of no part"))),
            Part::Value { content, valid, .. } => {
                let over = match self.limits.value {
                    Some(limit) => past(limit, self.value_bytes, data.len()),
                    None => None,
                };
                let before = content.len();
                content.extend_from_slice(data);
                let invalid = match utf8_prefix(&content[*valid..]) {
                    Ok(known) => {
                        *valid += known;
                        None
                    }
                    Err(at) => Some(
                        (*valid + at)
                            .checked_sub(before)
                            .ok_or(ReadError::Form(Error::Internal("a UTF-8 failure before the new bytes")))?,
                    ),
                };
                match (over, invalid) {
                    (Some(over), Some(invalid)) => {
                        // H-5: the failure whose byte comes first; at one byte the limit comes first.
                        if reveal(rest, needle, over) <= reveal(rest, needle, invalid) {
                            Err(ReadError::Form(Error::ValueLimit))
                        } else {
                            Err(ReadError::Form(Error::Invalid))
                        }
                    }
                    (Some(_), None) => Err(ReadError::Form(Error::ValueLimit)),
                    (None, Some(_)) => Err(ReadError::Form(Error::Invalid)),
                    (None, None) => {
                        self.value_bytes = self.value_bytes.checked_add(length).ok_or(ReadError::Form(Error::ValueLimit))?;
                        Ok(())
                    }
                }
            }
            Part::File { size } => {
                if let Some(limit) = self.limits.file {
                    if past(limit, *size, data.len()).is_some() {
                        return Err(ReadError::Form(Error::FileLimit));
                    }
                }
                *size = size.checked_add(length).ok_or(ReadError::Form(Error::FileLimit))?;
                sink(Piece::FileData(data)).map_err(ReadError::Sink)
            }
        }
    }

    fn part_ended<E>(&mut self, sink: &mut impl FnMut(Piece<'_>) -> Result<(), E>) -> Result<(), ReadError<E>> {
        match std::mem::replace(&mut self.part, Part::None) {
            Part::None => Err(ReadError::Form(Error::Internal("the end of no part"))),
            Part::Value { name, content, valid } => {
                // HY-57.5: a value whose content ends inside a UTF-8 sequence fails at the end of its content.
                if valid != content.len() {
                    return Err(ReadError::Form(Error::Invalid));
                }
                let value = String::from_utf8(content).map_err(|_| ReadError::Form(Error::Internal("UTF-8")))?;
                sink(Piece::Field { name, value }).map_err(ReadError::Sink)
            }
            Part::File { .. } => sink(Piece::FileEnd).map_err(ReadError::Sink),
        }
    }

    fn add_value_bytes<E>(&mut self, bytes: u64) -> Result<(), ReadError<E>> {
        self.value_bytes = self.value_bytes.checked_add(bytes).ok_or(ReadError::Form(Error::ValueLimit))?;
        match self.limits.value {
            Some(limit) if self.value_bytes > limit => Err(ReadError::Form(Error::ValueLimit)),
            Some(_) | None => Ok(()),
        }
    }
}

/// The index in `length` new bytes of the first byte past `limit`, when `used` bytes were counted before.
fn past(limit: u64, used: u64, length: usize) -> Option<usize> {
    let allowed = limit.saturating_sub(used);
    match usize::try_from(allowed) {
        Ok(allowed) if allowed < length => Some(allowed),
        Ok(_) | Err(_) => None,
    }
}

/// The length of the longest end of `bytes` that is the start of `needle` and shorter than it: the bytes that a later
/// byte may still make a delimiter.
fn pending(bytes: &[u8], needle: &[u8]) -> usize {
    let window = bytes.len() - bytes.len().min(needle.len() - 1);
    for found in memchr::memchr_iter(needle[0], &bytes[window..]) {
        if needle.starts_with(&bytes[window + found..]) {
            return bytes.len() - (window + found);
        }
    }
    0
}

/// The index of the byte that reveals `index` as content: the byte at which the last start of `needle` that covers
/// `index` stops matching, or `index` when no start of `needle` covers it.
fn reveal(bytes: &[u8], needle: &[u8], index: usize) -> usize {
    let mut revealed = index;
    let low = index.saturating_sub(needle.len() - 1);
    for found in memchr::memchr_iter(needle[0], &bytes[low..=index]) {
        let start = low + found;
        let matched = bytes[start..].iter().zip(needle).take_while(|(byte, expected)| byte == expected).count();
        if start + matched > index {
            revealed = revealed.max(start + matched);
        }
    }
    revealed
}

/// The length of the start of `bytes` that is UTF-8 text or the start of one more character, or the index of the
/// byte after which the bytes are no start of UTF-8 text (HY-57.5).
fn utf8_prefix(bytes: &[u8]) -> Result<usize, usize> {
    match std::str::from_utf8(bytes) {
        Ok(_) => Ok(bytes.len()),
        Err(error) => match error.error_len() {
            None => Ok(error.valid_up_to()),
            Some(length) => {
                let start = error.valid_up_to();
                // A byte that starts no UTF-8 sequence reveals the failure itself; a sequence that a byte cuts off
                // is revealed by that byte.
                if matches!(bytes[start], 0x80..=0xC1 | 0xF5..=0xFF) {
                    Err(start)
                } else {
                    Err(start + length)
                }
            }
        },
    }
}

/// True when the header lines that `bytes` starts with are longer than `HEADER_LIMIT` whatever bytes follow: no empty
/// line starts at an index up to `HEADER_LIMIT`, and none can start there with later bytes (HY-113.16).
fn header_too_long(bytes: &[u8]) -> bool {
    if bytes.len() <= HEADER_BYTES {
        return false;
    }
    let low = bytes.len().saturating_sub(HEADER_END.len() - 1);
    (low..=HEADER_BYTES).all(|start| !HEADER_END.starts_with(&bytes[start..]))
}

fn utf8(bytes: &[u8]) -> Option<String> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Some(text.to_owned()),
        Err(_) => None,
    }
}

fn split_lines(header: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0;
    for found in memchr::memmem::find_iter(header, b"\r\n") {
        if found >= start {
            lines.push(&header[start..found]);
            start = found + 2;
        }
    }
    lines.push(&header[start..]);
    lines
}
