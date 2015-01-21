use super::{Category, Content, Generator, Entry, Feed, Mark, Metadata,
            Link, Person, Source, Text, TextType};

use std::borrow::ToOwned;
use std::default::Default;
use std::str::FromStr;

use chrono::{DateTime, FixedOffset};

use codecs;
use parser::base::{DecodeResult, DecodeError, XmlElement, XmlName};
use parser::base::NestedEvent::Nested;
use schema::{Codec, DocumentElement, FromSchemaReader, Mergeable};

use feed::{set_default};


/// The XML namespace name used for Atom (RFC 4287).
const ATOM_XMLNS: &'static str = "http://www.w3.org/2005/Atom";

/// The XML namespace name used for Earth Reader `Mark` metadata.
const MARK_XMLNS: &'static str = "http://earthreader.org/mark/";


impl DocumentElement for Feed {
    fn tag() -> &'static str { "feed" }
    fn xmlns() -> Option<&'static str> { Some(ATOM_XMLNS) }
}

impl DocumentElement for Entry {
    fn tag() -> &'static str { "entry" }
    fn xmlns() -> Option<&'static str> { Some(ATOM_XMLNS) }
}


impl FromSchemaReader for Feed {
    fn match_child<B: Buffer>(&mut self, name: &XmlName,
                              child: XmlElement<B>) -> DecodeResult<()> {
        match &name.local_name[] {
            "entry" => {
                let mut entry: Entry = Default::default();
                try!(entry.read_from(child));
                self.entries.push(entry);
            }
            _ => { return self.source.match_child(name, child); }
        }
        Ok(())
    }
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


impl FromSchemaReader for Text {
    fn read_from<B: Buffer>(&mut self, element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        self.type_ = match element.get_attr("type") {
            Ok("text/plaln") | Ok("text") => TextType::Text,
            Ok("text/html") | Ok("html") => TextType::Html,
            Ok(_) => { TextType::Text },  // TODO
            Err(DecodeError::AttributeNotFound(_)) => Default::default(),
            Err(e) => { return Err(e); }
        };
        self.value = element.read_whole_text().unwrap();
        Ok(())
    }
}

impl FromSchemaReader for Option<Person> {
    fn read_from<B: Buffer>(&mut self, mut element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        *self = None;
        loop {
            match element.children.next() {
                Some(Nested { name, element: child }) => {
                    try!(self.match_child(&name, child))
                }
                None => { break; }
                Some(_) => { }     
            }
        }
        if self.as_ref().map_or(true, |p| p.name.is_empty()) {
            *self = None;
        }
        Ok(())
    }

    fn match_child<B: Buffer>(&mut self, name: &XmlName,
                              element: XmlElement<B>)
                              -> DecodeResult<()>
    {
        match &name.local_name[] {
            "name" => {
                let name = try!(element.read_whole_text());
                set_default(self).name = name;
            }
            "uri" => {
                let uri = Some(try!(element.read_whole_text()));
                set_default(self).uri = uri;
            }
            "email" => {
                let email = Some(try!(element.read_whole_text()));
                set_default(self).email = email;
            }
            _ => { return Err(DecodeError::NoResult); }
        }
        Ok(())
    }
}

impl FromSchemaReader for Link {
    fn read_from<B: Buffer>(&mut self, element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        self.uri = try!(element.get_attr("href")).to_owned();
        self.relation = element.get_attr("rel")
                               .unwrap_or("alternate").to_owned();
        self.mimetype = element.get_attr("type").ok()
                               .map(ToOwned::to_owned);
        self.language = element.get_attr("hreflang").ok()
                               .map(ToOwned::to_owned);
        self.title = element.get_attr("title").ok()
                            .map(ToOwned::to_owned);
        self.byte_size = element.get_attr("length").ok()
                                .and_then(FromStr::from_str);
        Ok(())
    }
}

impl FromSchemaReader for Category {
    fn read_from<B: Buffer>(&mut self, element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        self.term = try!(element.get_attr("term")).to_owned();
        self.scheme_uri = element.get_attr("scheme").ok()
                                 .map(|v| v.to_string());
        self.label = element.get_attr("label").ok().map(|v| v.to_string());
        Ok(())
    }
}

impl FromSchemaReader for Generator {
    fn read_from<B: Buffer>(&mut self, element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        self.uri = element.get_attr("uri").ok().map(|v| v.to_string()); // TODO
        self.version = element.get_attr("version").ok().map(|v| v.to_string());
        self.value = try!(element.read_whole_text());
        Ok(())
    }
}

impl FromSchemaReader for Content {
    fn read_from<B: Buffer>(&mut self, element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        self.source_uri = element.get_attr("src").ok().map(|v| v.to_string());
        try!(self.text.read_from(element));  // TODO
        Ok(())
    }
}

impl FromSchemaReader for Mark {
    fn read_from<B: Buffer>(&mut self, element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        self.updated_at = {
            let updated_at = try!(element.get_attr("updated"));
            Some(try!(codecs::RFC3339.decode(updated_at)))
        };
        let content = try!(element.read_whole_text());
        let codec: codecs::Boolean = Default::default();
        self.marked = try!(codec.decode(&content[]));
        Ok(())
    }        
}

pub fn parse_datetime<B: Buffer>(element: XmlElement<B>)
                                 -> DecodeResult<DateTime<FixedOffset>>
{
    match codecs::RFC3339.decode(&*try!(element.read_whole_text())) {
        Ok(v) => Ok(v),
        Err(e) => Err(DecodeError::SchemaError(e)),
    }
}


impl Mergeable for Entry {
    fn merge_entities(mut self, other: Entry) -> Entry {
        self.read = self.read.merge_entities(other.read);
        self.starred = self.starred.merge_entities(other.starred);
        self
    }
}
