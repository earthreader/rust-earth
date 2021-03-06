use std::default::Default;
use std::io;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, FixedOffset};

use parser::base::{DecodeResult, XmlElement, XmlName};
use schema::{FromSchemaReader, Mergeable};

use util::set_default;

use super::{ATOM_XMLNS, Generator, Metadata, Text};

/// All metadata for `Feed` excepting `Feed.entries`.
/// It corresponds to `atom:source` element of :rfc:`4287#section-4.2.10`
/// (section 4.2.10).
#[derive(Default)]
pub struct Source {
    pub metadata: Metadata,

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
        Source::new_inherited(id, title, updated_at)
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
            _ => { return self.metadata.match_child(name, child); }
        }
        Ok(())
    }
}

impl_mergeable!(Source, metadata, subtitle, generator, logo, icon);
