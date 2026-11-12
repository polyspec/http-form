//! Ordered maps from a name to its values in request order (HY-56, HY-57, HY-113.2) and the form of a body.

use std::collections::HashMap;

use crate::{Error, Event, Piece, UploadedFile};

/// An ordered map from each name, in the order of its first occurrence, to its values in request order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named<T> {
    entries: Vec<(String, Vec<T>)>,
    index: HashMap<String, usize>,
}

impl<T> Default for Named<T> {
    fn default() -> Self {
        Named { entries: Vec::new(), index: HashMap::new() }
    }
}

impl<T> Named<T> {
    /// Adds a value under a name, after the values that the name already has.
    pub fn push(&mut self, name: String, value: T) {
        match self.index.get(&name) {
            Some(&position) => self.entries[position].1.push(value),
            None => {
                self.index.insert(name.clone(), self.entries.len());
                self.entries.push((name, vec![value]));
            }
        }
    }

    /// The names with their values, in the order of the first occurrence of each name.
    pub fn entries(&self) -> &[(String, Vec<T>)] {
        &self.entries
    }

    /// Consumes the map into its entries.
    pub fn into_entries(self) -> Vec<(String, Vec<T>)> {
        self.entries
    }

    /// The values of a name in request order; a name without a value has none (HY-56).
    pub fn get(&self, name: &str) -> &[T] {
        match self.index.get(name) {
            Some(&position) => &self.entries[position].1,
            None => &[],
        }
    }

    /// The last value of a name, or `None` for a name without a value (HY-113.2).
    pub fn last(&self, name: &str) -> Option<&T> {
        self.get(name).last()
    }

    /// The number of names.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when no name has a value.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Text values by name (HY-56, HY-57).
pub type Fields = Named<String>;

/// Files by name (HY-113).
pub type Files = Named<UploadedFile>;

/// A file part that started and did not end.
#[derive(Debug, Clone)]
struct Open {
    name: String,
    filename: String,
    content_type: String,
    content: Vec<u8>,
}

/// The text fields and the files of a body.
#[derive(Debug, Clone, Default)]
pub struct Form {
    pub fields: Fields,
    pub files: Files,
    open: Option<Open>,
}

impl Form {
    /// A form of fields and files.
    pub fn new(fields: Fields, files: Files) -> Self {
        Form { fields, files, open: None }
    }

    /// Adds a piece of a reader, as `collect` adds an event. A piece out of the order of a reader is `internal`
    /// (H-8).
    pub fn take(&mut self, piece: Piece<'_>) -> Result<(), Error> {
        match piece {
            Piece::FileData(bytes) => self.data(bytes),
            Piece::Field { name, value } => {
                self.fields.push(name, value);
                Ok(())
            }
            Piece::FileStart { name, filename, content_type } => self.start(name, filename, content_type),
            Piece::FileEnd => self.end(),
        }
    }

    /// Adds the events of a reader: a field becomes a value, and the start, the data and the end of a file become
    /// one file whose content is a value. An event out of the order of a reader is `internal` (H-8).
    pub fn collect(&mut self, events: Vec<Event>) -> Result<(), Error> {
        for event in events {
            match event {
                Event::Field { name, value } => self.fields.push(name, value),
                Event::FileStart { name, filename, content_type } => self.start(name, filename, content_type)?,
                Event::FileData(bytes) => self.data(&bytes)?,
                Event::FileEnd => self.end()?,
            }
        }
        Ok(())
    }

    fn start(&mut self, name: String, filename: String, content_type: String) -> Result<(), Error> {
        if self.open.is_some() {
            return Err(Error::Internal("a file started before the end of the file before it"));
        }
        self.open = Some(Open { name, filename, content_type, content: Vec::new() });
        Ok(())
    }

    fn data(&mut self, bytes: &[u8]) -> Result<(), Error> {
        match self.open.as_mut() {
            Some(open) => {
                open.content.extend_from_slice(bytes);
                Ok(())
            }
            None => Err(Error::Internal("data of a file came without its start")),
        }
    }

    fn end(&mut self) -> Result<(), Error> {
        let Some(open) = self.open.take() else {
            return Err(Error::Internal("the end of a file came without its start"));
        };
        let file = UploadedFile::from_value(open.name.clone(), open.filename, open.content_type, open.content)?;
        self.files.push(open.name, file);
        Ok(())
    }

    /// True when a file of the form has more bytes than `limit`, and false when every file has at most `limit` bytes
    /// or the form has no file (HY-114.9).
    pub fn file_over(&self, limit: u64) -> bool {
        self.files.entries().iter().flat_map(|(_, files)| files.iter()).any(|file| file.size() > limit)
    }
}
