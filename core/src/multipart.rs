//! The state machine that reads a `multipart/form-data` body in chunks (HY-131) and the rules of its parts
//! (HY-113).
//!
//! The reader finds the delimiters and gives the header lines and the content of each part to its parts policy. Besides
//! one chunk it holds at most the delimiter or the header lines of one part. The result does not depend on the chunks
//! (HY-131.6).

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

/// What the reader found in a chunk. A server writes the data of a file as it arrives (HY-131.13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A text value, whole.
    Field { name: String, value: String },
    /// The start of a file part.
    FileStart { name: String, filename: String, content_type: String },
    /// The next bytes of the file that started last.
    FileData(Vec<u8>),
    /// The end of the file that started last.
    FileEnd,
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
    separator: Vec<u8>,
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
            separator,
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

    /// Reads the next chunk of the body and returns what it found. After an error every call returns that error, and
    /// after the `--` that ends the body the rest is ignored (HY-131.4, HY-131.5).
    pub fn feed(&mut self, chunk: &[u8]) -> Result<Vec<Event>, FormError> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        if self.state == State::Done {
            return Ok(Vec::new());
        }
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        loop {
            match self.advance(&mut events) {
                Ok(true) => continue,
                Ok(false) => break,
                Err(error) => {
                    self.state = State::Failed;
                    self.buffer = Vec::new();
                    self.part = Part::None;
                    self.error = Some(error.clone());
                    return Err(error);
                }
            }
        }
        Ok(events)
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

    fn drop_read(&mut self, read: usize, state: State) -> bool {
        self.buffer.drain(..read);
        self.state = state;
        true
    }

    /// Takes one step and returns false when it needs another chunk or has ended.
    fn advance(&mut self, events: &mut Vec<Event>) -> Result<bool, FormError> {
        let length = self.buffer.len();
        match self.state {
            State::Preamble => {
                if self.first {
                    if length < self.delimiter.len() {
                        if self.delimiter.starts_with(&self.buffer) {
                            return Ok(false);
                        }
                    } else if self.buffer.starts_with(&self.delimiter) {
                        self.first = false;
                        return Ok(self.drop_read(self.delimiter.len(), State::Delimiter));
                    }
                    self.first = false;
                }
                if let Some(index) = search(&self.buffer, &self.separator) {
                    return Ok(self.drop_read(index + self.separator.len(), State::Delimiter));
                }
                let keep = length.min(self.separator.len() - 1);
                self.buffer.drain(..length - keep);
                Ok(false)
            }
            State::Delimiter => {
                if length < 2 {
                    return Ok(false);
                }
                match &self.buffer[..2] {
                    b"--" => {
                        self.buffer = Vec::new();
                        self.state = State::Done;
                        Ok(false)
                    }
                    b"\r\n" => Ok(self.drop_read(2, State::Headers)),
                    _ => Err(FormError::Invalid),
                }
            }
            State::Headers => {
                if length < 2 {
                    return Ok(false);
                }
                let empty = self.buffer.starts_with(b"\r\n");
                let end = if empty { Some(0) } else { search(&self.buffer, b"\r\n\r\n") };
                let Some(end) = end else {
                    return if length >= HEADER_LIMIT + 4 { Err(FormError::Invalid) } else { Ok(false) };
                };
                if end > HEADER_LIMIT {
                    return Err(FormError::Invalid);
                }
                let header = self.buffer[..end].to_vec();
                let lines: Vec<&[u8]> = if empty { Vec::new() } else { split_lines(&header) };
                if lines.iter().any(|line| line.starts_with(&self.delimiter)) {
                    return Err(FormError::Invalid);
                }
                self.drop_read(if empty { 2 } else { end + 4 }, State::Content);
                self.part_started(&lines, events)?;
                Ok(true)
            }
            State::Content => {
                if let Some(index) = search(&self.buffer, &self.separator) {
                    if index > 0 {
                        let data = self.buffer[..index].to_vec();
                        self.part_data(data, events)?;
                    }
                    self.part_ended(events)?;
                    return Ok(self.drop_read(index + self.separator.len(), State::Delimiter));
                }
                // A separator that the next chunk completes starts with a CR among the last bytes: keep the bytes from
                // the first such CR.
                let window = length - length.min(self.separator.len() - 1);
                let keep = self.buffer[window..].iter().position(|&byte| byte == b'\r').map_or(0, |at| length - (window + at));
                if length > keep {
                    let data: Vec<u8> = self.buffer.drain(..length - keep).collect();
                    self.part_data(data, events)?;
                }
                Ok(false)
            }
            State::Done | State::Failed => Ok(false),
        }
    }

    fn part_started(&mut self, lines: &[&[u8]], events: &mut Vec<Event>) -> Result<(), FormError> {
        self.part = Part::None;
        self.part_count += 1;
        if let Some(limit) = self.limits.parts {
            if self.part_count > limit {
                return Err(FormError::Exceeded(Limit::Parts));
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
                    _ => return Err(FormError::Invalid),
                }
            }
            None => None,
        };
        let name = utf8(raw_name).ok_or(FormError::Invalid)?;
        match file {
            None => {
                // The name counts as its bytes were sent.
                self.value_bytes += raw_name.len() as u64;
                self.check_value()?;
                self.part = Part::Value { name, content: Vec::new() };
            }
            Some((filename, content_type)) => {
                self.part = Part::File { size: 0 };
                events.push(Event::FileStart { name, filename, content_type });
            }
        }
        Ok(())
    }

    fn part_data(&mut self, data: Vec<u8>, events: &mut Vec<Event>) -> Result<(), FormError> {
        match &mut self.part {
            Part::None => Ok(()),
            Part::Value { content, .. } => {
                self.value_bytes += data.len() as u64;
                content.extend_from_slice(&data);
                self.check_value()
            }
            Part::File { size } => {
                *size += data.len() as u64;
                if let Some(limit) = self.limits.file {
                    if *size > limit {
                        return Err(FormError::Exceeded(Limit::File));
                    }
                }
                events.push(Event::FileData(data));
                Ok(())
            }
        }
    }

    fn part_ended(&mut self, events: &mut Vec<Event>) -> Result<(), FormError> {
        match std::mem::replace(&mut self.part, Part::None) {
            Part::None => Ok(()),
            Part::Value { name, content } => {
                let value = String::from_utf8(content).map_err(|_| FormError::Invalid)?;
                events.push(Event::Field { name, value });
                Ok(())
            }
            Part::File { .. } => {
                events.push(Event::FileEnd);
                Ok(())
            }
        }
    }

    fn check_value(&self) -> Result<(), FormError> {
        match self.limits.value {
            Some(limit) if self.value_bytes > limit => Err(FormError::Exceeded(Limit::Value)),
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
    while let Some(at) = search(&header[start..], b"\r\n") {
        lines.push(&header[start..start + at]);
        start += at + 2;
    }
    lines.push(&header[start..]);
    lines
}

/// The first position of the needle in the bytes.
pub(crate) fn search(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if haystack.len() < needle.len() {
        return None;
    }
    let first = needle[0];
    let mut from = 0;
    while from + needle.len() <= haystack.len() {
        let at = from + haystack[from..=haystack.len() - needle.len()].iter().position(|&byte| byte == first)?;
        if &haystack[at..at + needle.len()] == needle {
            return Some(at);
        }
        from = at + 1;
    }
    None
}
