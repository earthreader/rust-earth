use std::default::Default;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, FixedOffset};

use parser::base::{DecodeResult, XmlElement, XmlName};
use schema::FromSchemaReader;

use util::set_default;

use super::{ATOM_XMLNS, Generator, Metadata, Text};

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

impl FromSchemaReader for Source {
    fn match_child<B: Buffer>(&mut self, name: &XmlName,
                              child: XmlElement<B>) -> DecodeResult<()> {
        match (name.namespace_as_ref(), &name.local_name[]) {
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
            _ => { return self.metadata.match_child(name, child); }
        }
        Ok(())
    }
}
