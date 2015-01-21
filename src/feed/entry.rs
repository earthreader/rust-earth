use std::default::Default;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, FixedOffset};

use super::{Content, Mark, Metadata, Source, Text};

#[derive(Default)]
pub struct Entry {
    pub metadata: Metadata,

    pub published_at: Option<DateTime<FixedOffset>>,
    pub summary: Option<Text>,
    pub content: Option<Content>,
    pub source: Option<Source>,
    pub read: Mark,
    pub starred: Mark,
}

impl Deref for Entry {
    type Target = Metadata;
    fn deref(&self) -> &Metadata { &self.metadata }
}

impl DerefMut for Entry {
    fn deref_mut(&mut self) -> &mut Metadata { &mut self.metadata }
}

impl Entry {
    pub fn new_inherited(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Entry {
        Entry {
            metadata: Metadata::new_inherited(id, title, updated_at),
            ..Default::default()
        }
    }

    pub fn new(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Entry {
        Entry::new(id, title, updated_at)
    }
}
