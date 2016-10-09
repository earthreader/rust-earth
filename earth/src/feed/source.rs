use std::io;

use chrono::{DateTime, FixedOffset};

use parser::base::{DecodeResult, XmlElement, XmlName};
use schema::{FromSchemaReader, Mergeable};
use util::{default_datetime, set_default};

use super::{ATOM_XMLNS, Category, Generator, Link, Person, Text};
use super::metadata::match_metadata_child;

/// All metadata for `Feed` excepting `Feed.entries`.
/// It corresponds to `atom:source` element of :rfc:`4287#section-4.2.10`
/// (section 4.2.10).
pub struct Source {
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

    /// A text that conveys a human-readable description or subtitle for a
    /// feed.  It corresponds to `atom:subtitle` element of
    /// :rfc:`4287#section-4.2.12` (section 4.2.12).
    pub subtitle: Option<Text>,

    /// Identify the agent used to generate a feed, for debugging and other
    /// purposes.  It corresponds to `atom:generator` element of
    /// :rfc:`4287#section-4.2.4` (section 4.2.4).
    pub generator: Option<Generator>,

    /// URI that identifies an image that provides visual identification for a
    /// feed.  It corresponds to `atom:logo` element of :rfc:`4287#section-4.2.8` (section 4.2.8).
    pub logo: Option<String>,

    /// URI that identifies an image that provides iconic visual identification
    /// for a feed.  It corresponds to `atom:icon` element of
    /// :rfc:`4287#section-4.2.5` (section 4.2.5).
    pub icon: Option<String>,
}

impl_metadata!(Source);

impl Source {
    pub fn new<I, T, U>(id: I, title: T, updated_at: U) -> Source
        where I: Into<String>, T: Into<Text>, U: Into<DateTime<FixedOffset>>
    {
        Source {
            id: id.into(),
            title: title.into(),
            links: Default::default(),
            updated_at: updated_at.into(),
            authors: Default::default(),
            contributors: Default::default(),
            categories: Default::default(),
            rights: Default::default(),
            subtitle: Default::default(),
            generator: Default::default(),
            logo: Default::default(),
            icon: Default::default(),
        }
    }
}

impl Default for Source {
    fn default() -> Source {
        Source::new("", Text::default(), default_datetime())
    }
}

impl FromSchemaReader for Source {
    fn match_child<B: io::BufRead>(&mut self, name: &XmlName,
                                   child: XmlElement<B>) -> DecodeResult<()> {
        match (name.namespace_ref(), &name.local_name[..]) {
            (Some(ATOM_XMLNS), "subtitle") => {
                *set_default(&mut self.subtitle) =
                    try!(FromSchemaReader::build_from(child));
            }
            (Some(ATOM_XMLNS), "generator") => {
                *set_default(&mut self.generator) =
                    try!(FromSchemaReader::build_from(child));
            }
            (Some(ATOM_XMLNS), "logo") => {
                *set_default(&mut self.logo) =
                    try!(child.read_whole_text());
            }
            (Some(ATOM_XMLNS), "icon") => {
                *set_default(&mut self.icon) =
                    try!(child.read_whole_text());
            }
            _ => { return match_metadata_child(self, name, child); }
        }
        Ok(())
    }
}

impl_mergeable!(Source, id, title, links, updated_at, authors, contributors,
                categories, rights, subtitle, generator, logo, icon);
