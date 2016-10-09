use std::borrow::Cow;
use std::io;

use chrono::{DateTime, FixedOffset};

use parser::base::{DecodeResult, XmlElement, XmlName};
use schema::{DocumentElement, Entity, FromSchemaReader, Mergeable};
use util::{default_datetime, set_default};

use super::{ATOM_XMLNS, MARK_XMLNS, Category, Content, Link, Mark, Person,
            Source, Text, parse_datetime};
use super::metadata::match_metadata_child;

/// Represent an individual entry, acting as a container for metadata and data
/// associated with the entry.  It corresponds to `atom:entry` element of
/// :rfc:`4287#section-4.1.2` (section 4.1.2).
pub struct Entry {
    /// The URI that conveys a permanent, universally unique identifier for an
    /// entry or feed.  It corresponds to `atom:id` element of :rfc:`4287#section-4.2.6` (section 4.2.6).
    pub id: String,

    /// The human-readable title for an entry or feed.
    /// It corresponds to `atom:title` element of :rfc:`4287#section-4.2.14` (section 4.2.14).
    pub title: Text,

    /// The list of :class:`Link` objects that define a reference from an entry
    /// or feed to a web resource.  It corresponds to `atom:link` element of
    /// :rfc:`4287#section-4.2.7` (section 4.2.7).
    pub links: Vec<Link>,

    /// The datetime value with a fixed timezone offset, indicating the most
    /// recent instant in time when the entry was modified in a way the
    /// publisher considers significant.  Therefore, not all modifications
    /// necessarily result in a changed `updated_at` value.
    /// It corresponds to `atom:updated` element of :rfc:`4287#section-4.2.15` (section 4.2.15).
    pub updated_at: DateTime<FixedOffset>,

    /// The list of `Person` values which indicates the author of the entry or
    /// feed.  It corresponds to `atom:author` element of :rfc:`4287#section-4.2.1` (section 4.2.1).
    pub authors: Vec<Person>,

    /// The list of `Person` values which indicates a person or other entity
    /// who contributed to the entry or feed.  It corresponds to
    /// `atom:contributor` element of :rfc:`4287#section-4.2.3` (section 4.2.3).
    pub contributors: Vec<Person>,

    /// The list of `Category` values that conveys information about categories
    /// associated with an entry or feed.  It corresponds to `atom:category`
    /// element of :rfc:`4287#section-4.2.2` (section 4.2.2).
    pub categories: Vec<Category>,

    /// The text field that conveys information about rights held in and of an
    /// entry or feed.  It corresponds to `atom:rights` element of
    /// :rfc:`4287#section-4.2.10` (section 4.2.10).
    pub rights: Option<Text>,

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

impl_metadata!(Entry);

impl Entry {
    pub fn new(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Entry {
        Entry {
            id: id,
            title: title,
            links: Default::default(),
            updated_at: updated_at,
            authors: Default::default(),
            contributors: Default::default(),
            categories: Default::default(),
            rights: Default::default(),
            published_at: Default::default(),
            summary: Default::default(),
            content: Default::default(),
            source: Default::default(),
            read: Default::default(),
            starred: Default::default(),
        }
    }
}

impl Default for Entry {
    fn default() -> Entry {
        Entry::new(Default::default(), Default::default(), default_datetime())
    }
}

impl DocumentElement for Entry {
    fn tag() -> &'static str { "entry" }
    fn xmlns() -> Option<&'static str> { Some(ATOM_XMLNS) }
}

impl FromSchemaReader for Entry {
    fn match_child<B: io::BufRead>(&mut self, name: &XmlName,
                                   child: XmlElement<B>) -> DecodeResult<()> {
        match (name.namespace_ref(), &name.local_name[..]) {
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
            _ => { return match_metadata_child(self, name, child); }
        }
        Ok(())
    }
}

impl Entity for Entry {
    type Id = str;
    fn entity_id(&self) -> Cow<str> { From::from(self.id.as_ref()) }
}

impl_mergeable!(Entry, id, title, links, updated_at, authors,
                contributors, categories, rights, read, starred);
