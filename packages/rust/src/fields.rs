//! The fields and files of a body as lists of a name and its values in request order (HY-56.7, HY-57, HY-113.2), the
//! form of a body, and the collector that makes it from the pieces of a reader.

use std::collections::HashMap;

use crate::{Error, Event, Piece, UploadedFile};

/// An ordered map from each name, in the order of its first occurrence, to its values in request order: the builder of
/// [`Fields`] and [`Files`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Named<T> {
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
}

/// Text values: each name, in the order of its first value, with its values in request order (HY-56.7, HY-57); the
/// contract type `list<tuple<text, list<text>>>` (design T-2).
pub type Fields = Vec<(String, Vec<String>)>;

/// Files: each name, in the order of its first file, with its files in request order (HY-113.2); the contract type
/// `list<tuple<text, list<uploaded_file>>>` (design T-2).
pub type Files = Vec<(String, Vec<UploadedFile>)>;

/// A file part that started and did not end.
#[derive(Debug, Clone)]
struct Open {
    name: String,
    filename: String,
    content_type: String,
    content: Vec<u8>,
}

/// The text fields and the files of a body: a record of the contract (design T-4).
#[derive(Debug, Clone, Default)]
pub struct Form {
    /// The text fields (HY-57).
    pub fields: Fields,
    /// The files (HY-113).
    pub files: Files,
}

/// Collects the pieces or the events of a reader into a [`Form`] whose files are values.
#[derive(Debug, Default)]
pub struct Collector {
    fields: Named<String>,
    files: Named<UploadedFile>,
    open: Option<Open>,
}

impl Collector {
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

    /// The form of the pieces; a file that started and did not end is `internal` (H-8), because a reader ends every
    /// file before the end of a body.
    pub fn into_form(self) -> Result<Form, Error> {
        if self.open.is_some() {
            return Err(Error::Internal("a file started and did not end"));
        }
        Ok(Form { fields: self.fields.into_entries(), files: self.files.into_entries() })
    }
}

/// True when a file has more bytes than `limit`, and false when every file has at most `limit` bytes or there is no
/// file (HY-114.9).
pub fn file_over(files: &[(String, Vec<UploadedFile>)], limit: u64) -> bool {
    files.iter().flat_map(|(_, files)| files.iter()).any(|file| file.size() > limit)
}
