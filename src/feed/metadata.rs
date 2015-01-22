#![unstable]

use std::default::Default;

use chrono::{DateTime, FixedOffset};

use parser::base::{DecodeResult, XmlElement, XmlName};
use schema::FromSchemaReader;
use util::set_default;

use super::{ATOM_XMLNS, Category, LinkList, Person, Text, parse_datetime};

pub struct Metadata {
    pub id: String,
    pub title: Text,
    pub links: LinkList,
    pub updated_at: DateTime<FixedOffset>,
    pub authors: Vec<Person>,
    pub contributors: Vec<Person>,
    pub categories: Vec<Category>,
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
