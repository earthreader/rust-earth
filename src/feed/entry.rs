#![unstable]

use std::borrow::Cow;
use std::default::Default;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, FixedOffset};

use parser::base::{DecodeResult, XmlElement, XmlName};
use schema::{DocumentElement, Entity, FromSchemaReader, Mergeable};

use util::set_default;

use super::{ATOM_XMLNS, MARK_XMLNS, Content, Mark, Metadata, Source, Text,
            parse_datetime};

/// Represent an individual entry, acting as a container for metadata and data
/// associated with the entry.  It corresponds to `atom:entry` element of
/// :rfc:`4287#section-4.1.2` (section 4.1.2).
#[derive(Default)]
pub struct Entry {
    pub metadata: Metadata,

    /// The datetime value with a fixed timezone offset, indicating an instant
    /// in time associated with an event early in the life cycle of the entry.
    /// Typically, `published_at` will be associated with the initial creation
    /// or first availability of the resource.
    /// It corresponds to `atom:published` element of :rfc:`4287#section-4.2.9`
    /// (section 4.2.9).
    pub published_at: Option<DateTime<FixedOffset>>,

    /// The text field that conveys a short summary, abstract, or excerpt of
    /// the entry.  It corresponds to ``atom:summary`` element of
    /// :rfc:`4287#section-4.2.13` (section 4.2.13).
    pub summary: Option<Text>,

    /// It either contains or links to the content of the entry.
    /// It corresponds to ``atom:content`` element of :rfc:`4287#section-4.1.3`
    /// (section 4.1.3).
    pub content: Option<Content>,

    /// If an entry is copied from one feed into another feed, then the source
    /// feed's metadata may be preserved within the copied entry by adding
    /// `source` if it is not already present in the entry, and including some
    /// or all of the source feed's metadata as the `source`'s data.
    ///
    /// It is designed to allow the aggregation of entries from different feeds
    /// while retaining information about an entry's source feed.
    ///
    /// It corresponds to ``atom:source`` element of :rfc:`4287#section-4.2.10`
    /// (section 4.2.10).
    pub source: Option<Source>,

    /// Whether and when it's read or unread.
    pub read: Mark,

    /// Whether and when it's starred or unstarred.
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

impl Entity for Entry {
    type OwnedId = String;
    type BorrowedId = str;
    fn entity_id(&self) -> Cow<String, str> {
        self.metadata.entity_id()
    }
}

impl_mergeable!(Entry, read, starred);
