use std::default::Default;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, FixedOffset};

use super::{Generator, Metadata, Text};

#[derive(Default)]
pub struct Source {
    pub metadata: Metadata,

    pub subtitle: Option<Text>,
    pub generator: Option<Generator>,
    pub logo: Option<String>,
    pub icon: Option<String>,
}

impl Deref for Source {
    type Target = Metadata;
    fn deref(&self) -> &Metadata { &self.metadata }
}

impl DerefMut for Source {
    fn deref_mut(&mut self) -> &mut Metadata { &mut self.metadata }
}

impl Source {
    pub fn new_inherited(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Source {
        Source {
            metadata: Metadata::new_inherited(id, title, updated_at),
            ..Default::default()
        }
    }

    pub fn new(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Source {
        Source::new(id, title, updated_at)
    }
}
