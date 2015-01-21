use std::default::Default;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, FixedOffset};

use parser::base::{DecodeResult, XmlElement, XmlName};
use schema::{DocumentElement, FromSchemaReader, Mergeable};

use util::set_default;

use super::{ATOM_XMLNS, MARK_XMLNS, Content, Mark, Metadata, Source, Text,
            parse_datetime};

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

impl DocumentElement for Entry {
    fn tag() -> &'static str { "entry" }
    fn xmlns() -> Option<&'static str> { Some(ATOM_XMLNS) }
}

impl FromSchemaReader for Entry {
    fn match_child<B: Buffer>(&mut self, name: &XmlName,
                              child: XmlElement<B>) -> DecodeResult<()> {
        match (name.namespace_as_ref(), &name.local_name[]) {
            (Some(ATOM_XMLNS), "published") => {
                self.published_at = Some(try!(parse_datetime(child)));
            }
            (Some(ATOM_XMLNS), "summary") => {
                *set_default(&mut self.summary) =
                    try!(FromSchemaReader::build_from(child));
            }
            (Some(ATOM_XMLNS), "content") => {
                *set_default(&mut self.content) =
                    try!(FromSchemaReader::build_from(child));
            }
            (Some(ATOM_XMLNS), "source") => {
                *set_default(&mut self.source) =
                    try!(FromSchemaReader::build_from(child));
            }
            (Some(MARK_XMLNS), "read") => {
                self.read = try!(FromSchemaReader::build_from(child));
            }
            (Some(MARK_XMLNS), "starred") => {
                self.starred = try!(FromSchemaReader::build_from(child));
            }
            _ => { return self.metadata.match_child(name, child); }
        }
        Ok(())
    }
}

impl Mergeable for Entry {
    fn merge_entities(mut self, other: Entry) -> Entry {
        self.read = self.read.merge_entities(other.read);
        self.starred = self.starred.merge_entities(other.starred);
        self
    }
}
