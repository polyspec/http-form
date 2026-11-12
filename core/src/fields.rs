//! Ordered maps from a name to its values in request order (HY-56, HY-57).

use std::collections::HashMap;

use crate::{Event, UploadedFile};

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

    /// The values of a name in request order; an absent name has none.
    pub fn get(&self, name: &str) -> &[T] {
        self.index.get(name).map(|&position| self.entries[position].1.as_slice()).unwrap_or(&[])
    }

    /// The last value of a name.
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

/// The text fields and the files of a body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Form {
    pub fields: Fields,
    pub files: Files,
    open: Option<(String, String, String, Vec<u8>)>,
}

impl Form {
    /// A form of fields and files.
    pub fn new(fields: Fields, files: Files) -> Self {
        Form { fields, files, open: None }
    }

    /// Adds the events of a reader: a field becomes a value, and the start, the data and the end of a file become
    /// one file whose content is a value.
    pub fn collect(&mut self, events: Vec<Event>) {
        for event in events {
            match event {
                Event::Field { name, value } => self.fields.push(name, value),
                Event::FileStart { name, filename, content_type } => {
                    self.open = Some((name, filename, content_type, Vec::new()))
                }
                Event::FileData(bytes) => {
                    if let Some(open) = self.open.as_mut() {
                        open.3.extend_from_slice(&bytes)
                    }
                }
                Event::FileEnd => {
                    if let Some((name, filename, content_type, content)) = self.open.take() {
                        self.files.push(name.clone(), UploadedFile::from_value(name, filename, content_type, content))
                    }
                }
            }
        }
    }

    /// The size in bytes of the largest file, or 0 when there is none (HY-114).
    pub fn largest_file(&self) -> u64 {
        self.files
            .entries()
            .iter()
            .flat_map(|(_, files)| files.iter())
            .map(UploadedFile::size)
            .max()
            .unwrap_or(0)
    }
}
