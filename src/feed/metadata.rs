#![unstable]

use std::default::Default;

use chrono::{DateTime, FixedOffset};

use parser::base::{DecodeResult, XmlElement, XmlName};
use schema::FromSchemaReader;
use util::set_default;

use super::{ATOM_XMLNS, Category, LinkList, Person, Text, parse_datetime};

/// Common metadata shared by `Source`, `Entry`, and `Feed`.
pub struct Metadata {
    /// The URI that conveys a permanent, universally unique identifier for an
    /// entry or feed.  It corresponds to `atom:id` element of :rfc:`4287#section-4.2.6` (section 4.2.6).
    pub id: String,

    /// The human-readable title for an entry or feed.
    /// It corresponds to `atom:title` element of :rfc:`4287#section-4.2.14` (section 4.2.14).
    pub title: Text,

    /// The list of :class:`Link` objects that define a reference from an entry
    /// or feed to a web resource.  It corresponds to `atom:link` element of
    /// :rfc:`4287#section-4.2.7` (section 4.2.7).
    pub links: LinkList,

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
}

impl Metadata {
    pub fn new_inherited(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Metadata {
        Metadata {
            id: id,
            title: title,
            updated_at: updated_at,
            ..Default::default()
        }            
    }
}

impl Default for Metadata {
    fn default() -> Metadata {
        use chrono::{DateTime, NaiveDateTime};
        let default_datetime = DateTime::from_utc(
            NaiveDateTime::from_num_seconds_from_unix_epoch(0, 0),
            FixedOffset::east(0)
        );
        Metadata {
            id: Default::default(),
            title: Default::default(),
            links: Default::default(),
            updated_at: default_datetime,
            authors: Default::default(),
            contributors: Default::default(),
            categories: Default::default(),
            rights: Default::default(),
        }
    }
}

impl FromSchemaReader for Metadata {
    fn match_child<B: Buffer>(&mut self, name: &XmlName,
                              child: XmlElement<B>) -> DecodeResult<()> {
        match (name.namespace_as_ref(), &name.local_name[]) {
            (Some(ATOM_XMLNS), "id") => {
                self.id = try!(child.read_whole_text());
            }
            (Some(ATOM_XMLNS), "title") => {
                try!(self.title.read_from(child));
            }
            (Some(ATOM_XMLNS), "link") => {
                self.links.push(try!(FromSchemaReader::build_from(child)));
            }
            (Some(ATOM_XMLNS), "updated") => {
                self.updated_at = try!(parse_datetime(child));
            }
            (Some(ATOM_XMLNS), "modified") => {
                self.updated_at = try!(parse_datetime(child));
            }
            (Some(ATOM_XMLNS), "author") => {
                match try!(FromSchemaReader::build_from(child)) {
                    Some(p) => self.authors.push(p),
                    None => { }
                }
            }
            (Some(ATOM_XMLNS), "contributor") => {
                match try!(FromSchemaReader::build_from(child)) {
                    Some(p) => self.contributors.push(p),
                    None => { }
                }
            }
            (Some(ATOM_XMLNS), "category") => {
                self.categories.push(try!(FromSchemaReader::build_from(child)));
            }
            (Some(ATOM_XMLNS), "rights") => {
                *set_default(&mut self.rights) = try!(FromSchemaReader::build_from(child));
            }
            _ => { }
        }
        Ok(())
    }
}
