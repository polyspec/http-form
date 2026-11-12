//! The state machine that reads a `multipart/form-data` body in chunks (HY-131) and the rules of its parts
//! (HY-113).
//!
//! The reader finds the delimiters and gives the header lines and the content of each part to its sink. Besides one
//! chunk it holds at most the delimiter or the header lines of one part, and it reads a chunk in place: the data of a
//! file reaches the sink as a slice of the chunk. The result does not depend on the chunks (HY-131.6).

use memchr::memmem::Finder;

use crate::header::{header_value, parameter, parameters};
use crate::{FormError, Limit};

/// The largest header of a part in bytes (HY-113.16).
pub const HEADER_LIMIT: usize = 8192;

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
#[derive(Debug)]
pub enum ReadError<E> {
    Form(FormError),
    Sink(E),
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

/// What the part that is read is.
enum Part {
    /// A part that is neither a value nor a file (HY-113.6).
    None,
    Value { name: String, content: Vec<u8> },
    File { size: u64 },
}

/// Reads a `multipart/form-data` body in chunks.
pub struct MultipartReader {
    delimiter: Vec<u8>,
    // The searchers hold SIMD values with an alignment above 16 bytes, which the allocator of a runtime that holds
    // the reader, such as PHP's or Python's, need not give; a box takes them from the Rust allocator.
    separator: Box<Finder<'static>>,
    header_end: Box<Finder<'static>>,
    buffer: Vec<u8>,
    state: State,
    first: bool,
    limits: Limits,
    value_bytes: u64,
    part_count: u64,
    part: Part,
    error: Option<FormError>,
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
            header_end: Box::new(Finder::new(b"\r\n\r\n").into_owned()),
            buffer: Vec::new(),
            state: State::Preamble,
            first: true,
            limits,
            value_bytes: 0,
            part_count: 0,
            part: Part::None,
            error: None,
        }
    }

    /// Reads the next chunk and returns what it found as owned values. After an error every call returns that error,
    /// and after the `--` that ends the body the rest is ignored (HY-131.4, HY-131.5).
    pub fn feed(&mut self, chunk: &[u8]) -> Result<Vec<Event>, FormError> {
        let mut events = Vec::new();
        let result = self.feed_with(chunk, |piece| {
            events.push(Event::from(piece));
            Ok::<(), std::convert::Infallible>(())
        });
        match result {
            Ok(()) => Ok(events),
            Err(ReadError::Form(error)) => Err(error),
            Err(ReadError::Sink(never)) => match never {},
        }
    }

    /// Reads the next chunk and gives each piece to the sink as it finds it. A failure of the sink fails the body.
    pub fn feed_with<E>(
        &mut self,
        chunk: &[u8],
        mut sink: impl FnMut(Piece<'_>) -> Result<(), E>,
    ) -> Result<(), ReadError<E>> {
        if let Some(error) = &self.error {
            return Err(ReadError::Form(error.clone()));
        }
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
            if let ReadError::Form(error) = error {
                self.error = Some(error.clone());
            } else {
                self.error = Some(FormError::Invalid);
            }
        }
        result
    }

    /// Ends the body: a body that did not end with the `--` after a delimiter fails (HY-131.5).
    pub fn finish(&mut self) -> Result<(), FormError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        if self.state != State::Done {
            self.state = State::Failed;
            self.buffer = Vec::new();
            self.error = Some(FormError::Invalid);
            return Err(FormError::Invalid);
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
                    return Ok(at + length - length.min(separator - 1));
                }
                State::Delimiter => {
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
                        }
                        _ => return Err(ReadError::Form(FormError::Invalid)),
                    }
                }
                State::Headers => {
                    if length < 2 {
                        return Ok(at);
                    }
                    let empty = rest.starts_with(b"\r\n");
                    let end = if empty { Some(0) } else { self.header_end.find(rest) };
                    let Some(end) = end else {
                        return if length >= HEADER_LIMIT + 4 { Err(ReadError::Form(FormError::Invalid)) } else { Ok(at) };
                    };
                    if end > HEADER_LIMIT {
                        return Err(ReadError::Form(FormError::Invalid));
                    }
                    let lines: Vec<&[u8]> = if empty { Vec::new() } else { split_lines(&rest[..end]) };
                    if lines.iter().any(|line| line.starts_with(&self.delimiter)) {
                        return Err(ReadError::Form(FormError::Invalid));
                    }
                    at += if empty { 2 } else { end + 4 };
                    self.state = State::Content;
                    self.part_started(&lines, sink)?;
                }
                State::Content => {
                    if let Some(index) = self.separator.find(rest) {
                        if index > 0 {
                            self.part_data(&rest[..index], sink)?;
                        }
                        self.part_ended(sink)?;
                        at += index + separator;
                        self.state = State::Delimiter;
                        continue;
                    }
                    // A separator that the next chunk completes starts with a CR among the last bytes: keep the bytes
                    // from the first such CR.
                    let window = length - length.min(separator - 1);
                    let keep = memchr::memchr(b'\r', &rest[window..]).map_or(0, |found| length - (window + found));
                    if length > keep {
                        self.part_data(&rest[..length - keep], sink)?;
                    }
                    return Ok(at + length - keep);
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
        self.part_count += 1;
        if let Some(limit) = self.limits.parts {
            if self.part_count > limit {
                return Err(ReadError::Form(FormError::Exceeded(Limit::Parts)));
            }
        }
        let Some(disposition) = header_value(lines, b"content-disposition") else {
            return Ok(());
        };
        let (kind, found) = parameters(disposition);
        let Some(raw_name) = parameter(&found, b"name") else {
            return Ok(());
        };
        if kind != b"form-data" {
            return Ok(());
        }
        let file = match parameter(&found, b"filename") {
            Some(filename) => {
                let content_type = header_value(lines, b"content-type").unwrap_or(b"");
                match (utf8(filename), utf8(content_type)) {
                    (Some(filename), Some(content_type)) => Some((filename, content_type)),
                    _ => return Err(ReadError::Form(FormError::Invalid)),
                }
            }
            None => None,
        };
        let name = utf8(raw_name).ok_or(ReadError::Form(FormError::Invalid))?;
        match file {
            None => {
                // The name counts as its bytes were sent.
                self.value_bytes += raw_name.len() as u64;
                self.check_value()?;
                self.part = Part::Value { name, content: Vec::new() };
            }
            Some((filename, content_type)) => {
                self.part = Part::File { size: 0 };
                sink(Piece::FileStart { name, filename, content_type }).map_err(ReadError::Sink)?;
            }
        }
        Ok(())
    }

    fn part_data<E>(
        &mut self,
        data: &[u8],
        sink: &mut impl FnMut(Piece<'_>) -> Result<(), E>,
    ) -> Result<(), ReadError<E>> {
        match &mut self.part {
            Part::None => Ok(()),
            Part::Value { content, .. } => {
                self.value_bytes += data.len() as u64;
                content.extend_from_slice(data);
                self.check_value()
            }
            Part::File { size } => {
                *size += data.len() as u64;
                if let Some(limit) = self.limits.file {
                    if *size > limit {
                        return Err(ReadError::Form(FormError::Exceeded(Limit::File)));
                    }
                }
                sink(Piece::FileData(data)).map_err(ReadError::Sink)
            }
        }
    }

    fn part_ended<E>(&mut self, sink: &mut impl FnMut(Piece<'_>) -> Result<(), E>) -> Result<(), ReadError<E>> {
        match std::mem::replace(&mut self.part, Part::None) {
            Part::None => Ok(()),
            Part::Value { name, content } => {
                let value = String::from_utf8(content).map_err(|_| ReadError::Form(FormError::Invalid))?;
                sink(Piece::Field { name, value }).map_err(ReadError::Sink)
            }
            Part::File { .. } => sink(Piece::FileEnd).map_err(ReadError::Sink),
        }
    }

    fn check_value<E>(&self) -> Result<(), ReadError<E>> {
        match self.limits.value {
            Some(limit) if self.value_bytes > limit => Err(ReadError::Form(FormError::Exceeded(Limit::Value))),
            _ => Ok(()),
        }
    }
}

fn utf8(bytes: &[u8]) -> Option<String> {
    std::str::from_utf8(bytes).ok().map(str::to_owned)
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
