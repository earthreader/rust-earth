use chrono::{DateTime, FixedOffset};

use xml;
use xml::common::{Name, Attribute};
use xml::reader::events::{EndDocument, StartElement, Characters};

use super::base::{XmlDecoder, DecodeResult, AttributeNotFound, SchemaError};
use feed;
use schema;
use schema::Codec;

static ATOM_XMLNS_SET: [&'static str, ..2] = [
    "http://www.w3.org/2005/Atom",
    "http://purl.org/atom/ns#",
];

static XML_XMLNS: &'static str = "http://www.w3.org/XML/1998/namespace";

pub struct CrawlerHint;

#[deriving(Clone)]
struct AtomSession {
    xml_base: String,
    element_ns: String,
}

fn get_xml_base<'a>(attributes: &'a [Attribute]) -> Option<&'a str> {
    attributes.iter().find(|&attr| {
        attr.name.namespace_ref().map_or(false, |ns| ns == XML_XMLNS)
    }).map(|attr| attr.value.as_slice())
}

fn name_matches<'a>(name: &'a Name, namespace: Option<&'a str>, local_name: &str) -> bool {
    name.namespace.as_ref().map(|n| n.as_slice()) == namespace && name.local_name.as_slice() == local_name
}

fn parse_feed<B: Buffer>(parser: &mut XmlDecoder<B>, feed_url: &str, need_entries: bool, session: AtomSession) -> DecodeResult<feed::Element> {
    let mut feed = feed::Element::new("feed".to_string());
    let mut entries = Vec::new();
    try!(parser.each_child(|p| {
        p.read_event(|p, event| {
            match event {
                &StartElement { ref name, ref attributes, .. }
                if ["id", "icon", "logo"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_icon(p, attributes.as_slice(), session.clone()));
                    feed.fields.insert(name.local_name.to_string(), feed::Str(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if ["title", "rights", "subtitle"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_text_construct(p, attributes.as_slice()));
                    feed.fields.insert(name.local_name.to_string(), feed::Elem(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if ["author", "contributor"].contains(&name.local_name.as_slice()) => {
                    match try!(parse_person_construct(p, attributes.as_slice(), session.clone())) {
                        Some(result) => {
                            feed.fields.insert(name.local_name.to_string().append("s"), feed::Elem(result));
                        }
                        None => { }
                    }
                }
                &StartElement { ref name, ref attributes, .. }
                if "link" == name.local_name.as_slice() => {
                    let result = try!(parse_link(attributes.as_slice(), session.clone()));
                    feed.fields.insert("links".to_string(), feed::Elem(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if ["updated", "modified"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_datetime(p));
                    feed.fields.insert("updated_at".to_string(), feed::DateTime(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if "category" == name.local_name.as_slice() => {
                    let result = try!(parse_category(attributes.as_slice()));
                    feed.fields.insert("categories".to_string(), feed::Elem(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if "generator" == name.local_name.as_slice() => {
                    let result = try!(parse_generator(p, attributes.as_slice(), session.clone()));
                    feed.fields.insert(name.local_name.to_string(), feed::Elem(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if need_entries && name_matches(name, Some(session.element_ns.as_slice()), "entry") => {
                    let result = try!(parse_entry(p, session.clone()));
                    entries.push(feed::Elem(result));
                }

                _otherwise => { }
            }
            Ok(())
        })
    }));
    feed.fields.find_or_insert("id".to_string(), feed::Str(feed_url.to_string()));
    feed.fields.insert("entries".to_string(), feed::Seq(entries));
    Ok(feed)
}

fn parse_entry<B: Buffer>(parser: &mut XmlDecoder<B>, session: AtomSession) -> DecodeResult<feed::Element> {
    let mut entry = feed::Element::new("entry".to_string());
    try!(parser.each_child(|p| {
        p.read_event(|p, event| {
            match event {
                &StartElement { ref name, ref attributes, .. }
                if "source" == name.local_name.as_slice() => {
                    let result = try!(parse_source(p, session.clone()));
                    entry.fields.insert(name.local_name.to_string(), feed::Elem(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if "id" == name.local_name.as_slice() => {
                    let result = try!(parse_icon(p, attributes.as_slice(), session.clone()));
                    entry.fields.insert(name.local_name.to_string(), feed::Str(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if ["title", "rights", "summary"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_text_construct(p, attributes.as_slice()));
                    entry.fields.insert(name.local_name.to_string(), feed::Elem(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if ["author", "contributor"].contains(&name.local_name.as_slice()) => {
                    match try!(parse_person_construct(p, attributes.as_slice(), session.clone())) {
                        Some(result) => {
                            entry.fields.insert(name.local_name.to_string().append("s"), feed::Elem(result));
                        }
                        None => { }
                    }
                }
                &StartElement { ref name, ref attributes, .. }
                if "link" == name.local_name.as_slice() => {
                    let result = try!(parse_link(attributes.as_slice(), session.clone()));
                    entry.fields.insert("links".to_string(), feed::Elem(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if ["updated", "published"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_datetime(p));
                    let field_name = if name.local_name.as_slice() == "published" {
                        "published_at"
                    } else {
                        "updated_at"
                    };
                    entry.fields.insert(field_name.to_string(), feed::DateTime(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if "category" == name.local_name.as_slice() => {
                    let result = try!(parse_category(attributes.as_slice()));
                    entry.fields.insert("categories".to_string(), feed::Elem(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if "content" == name.local_name.as_slice() => {
                    let result = try!(parse_content(p, attributes.as_slice(), session.clone()));
                    entry.fields.insert(name.local_name.to_string(), feed::Elem(result));
                }

                _otherwise => { }
            }
            Ok(())
        })
    }));
    Ok(entry)
}

fn parse_source<B: Buffer>(parser: &mut XmlDecoder<B>, session: AtomSession) -> DecodeResult<feed::Element> {
    let mut source = feed::Element::new("source".to_string());
    try!(parser.each_child(|p| {
        p.read_event(|p, event| {
            match event {
                &StartElement { ref name, ref attributes, .. }
                if ["id", "icon", "logo"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_icon(p, attributes.as_slice(), session.clone()));
                    source.fields.insert(name.local_name.to_string(), feed::Str(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if ["title", "rights", "subtitle"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_text_construct(p, attributes.as_slice()));
                    source.fields.insert(name.local_name.to_string(), feed::Elem(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if ["author", "contributor"].contains(&name.local_name.as_slice()) => {
                    match try!(parse_person_construct(p, attributes.as_slice(), session.clone())) {
                        Some(result) => {
                            source.fields.insert(name.local_name.to_string().append("s"), feed::Elem(result));
                        }
                        None => { }
                    }
                }
                &StartElement { ref name, ref attributes, .. }
                if "link" == name.local_name.as_slice() => {
                    let result = try!(parse_link(attributes.as_slice(), session.clone()));
                    source.fields.insert("links".to_string(), feed::Elem(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if "updated" == name.local_name.as_slice() => {
                    let result = try!(parse_datetime(p));
                    source.fields.insert("updated_at".to_string(), feed::DateTime(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if "category" == name.local_name.as_slice() => {
                    let result = try!(parse_category(attributes.as_slice()));
                    source.fields.insert("categories".to_string(), feed::Elem(result));
                }
                &StartElement { ref name, ref attributes, .. }
                if "generator" == name.local_name.as_slice() => {
                    let result = try!(parse_generator(p, attributes.as_slice(), session.clone()));
                    source.fields.insert(name.local_name.to_string(), feed::Elem(result));
                }

                _otherwise => { }
            }
            Ok(())
        })
    }));
    Ok(source)
}

fn reset_xml_base(attributes: &[Attribute], session: &mut AtomSession) {
    for new_base in get_xml_base(attributes.as_slice()).move_iter() {
        session.xml_base = new_base.into_string();
    }
}

fn read_whole_text<B: Buffer>(parser: &mut XmlDecoder<B>) -> DecodeResult<String> {
    let mut text = String::new();
    try!(parser.each_child(|p| {
        p.read_event(|_p, event| {
            match event {
                &Characters(ref s) => { text.push_str(s.as_slice()); }
                _ => { }
            }
            Ok(())
        })
    }));
    Ok(text)
}

fn parse_icon<B: Buffer>(parser: &mut XmlDecoder<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<String> {
    reset_xml_base(attributes, &mut session);
    Ok(session.xml_base.append(try!(read_whole_text(parser)).as_slice()))
}

fn parse_text_construct<B: Buffer>(parser: &mut XmlDecoder<B>, attributes: &[Attribute]) -> DecodeResult<feed::Element> {
    let mut text = feed::Element::new("text".to_string());
    let text_type = attributes.iter().find(|&attr| attr.name.local_name.as_slice() == "type");
    let text_type = match text_type.map(|e| e.value.as_slice()) {
        Some("text/plaln") => "text",
        Some("text/html") => "html",
        _ => "text",
    };
    if ["text", "html"].contains(&text_type) {
        text.fields.insert("value".to_string(), feed::Str(try!(read_whole_text(parser))));
    } else if text_type == "xhtml" {
        text.fields.insert("value".to_string(), feed::Str("".to_string()));  // TODO
    }
    text.fields.insert("type".to_string(), feed::Str(text_type.to_string()));
    Ok(text)
}

fn parse_person_construct<B: Buffer>(parser: &mut XmlDecoder<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<Option<feed::Element>> {
    reset_xml_base(attributes, &mut session);
    let mut person = feed::Element::new("person".to_string());
    try!(parser.each_child(|p| {
        p.read_event(|p, event| {
            match event {
                &StartElement { ref name, .. }
                if name_matches(name, Some(session.element_ns.as_slice()), "name") => {
                    person.fields.insert("name".to_string(), feed::Str(try!(read_whole_text(p))));
                }
                &StartElement { ref name, .. }
                if name_matches(name, Some(session.element_ns.as_slice()), "uri") => {
                    person.fields.insert("uri".to_string(), feed::Str(try!(read_whole_text(p))));
                }
                &StartElement { ref name, .. }
                if name_matches(name, Some(session.element_ns.as_slice()), "email") => {
                    person.fields.insert("email".to_string(), feed::Str(try!(read_whole_text(p))));
                }
                _ => { }
            }
            Ok(())
        })
    }));
    let name = "name".to_string();
    if !person.fields.contains_key(&name) {
        let v = match person.fields.find(&"uri".to_string())
                           .or_else(|| person.fields.find(&"email".to_string())) {
            Some(v) => { v.clone() }
            None => { return Ok(None); }
        };
        person.fields.insert(name, v);
    }
    Ok(Some(person))
}

fn find_from_attr<'a>(attr: &'a [Attribute], key: &str) -> Option<&'a str> {
    attr.iter()
        .find(|&attr| attr.name.local_name.as_slice() == key)
        .map(|e| e.value.as_slice())
}

macro_rules! f (
    ($attr:expr, $k:expr) => (
        match find_from_attr($attr, $k) {
            Some(v) => { v.to_string() }
            None => { return Err(AttributeNotFound($k.to_string())); }
        }
    )
)

fn parse_link(attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<feed::Element> {
    reset_xml_base(attributes, &mut session);
    let mut link = feed::Element::new("link".to_string());
    link.fields.insert("uri".to_string(), feed::Str(f!(attributes, "href")));
    link.fields.insert("mimetype".to_string(), feed::Str(f!(attributes, "type")));
    link.fields.insert("language".to_string(), feed::Str(f!(attributes, "hreflang")));
    link.fields.insert("title".to_string(), feed::Str(f!(attributes, "title")));
    link.fields.insert("byte_size".to_string(), feed::Str(f!(attributes, "length")));
    for rel in find_from_attr(attributes, "rel").move_iter() {
        link.fields.insert("relation".to_string(), feed::Str(rel.to_string()));
    }
    Ok(link)
}

fn parse_datetime<B: Buffer>(parser: &mut XmlDecoder<B>) -> DecodeResult<DateTime<FixedOffset>> {
    match schema::RFC3339.decode(try!(read_whole_text(parser)).as_slice()) {
        Ok(v) => Ok(v),
        Err(e) => Err(SchemaError(e)),
    }
}

fn parse_category(attributes: &[Attribute]) -> DecodeResult<feed::Element> {
    let mut category = feed::Element::new("category".to_string());
    category.fields.insert("term".to_string(), feed::Str(f!(attributes, "term")));
    category.fields.insert("scheme_uri".to_string(), feed::Str(f!(attributes, "scheme")));
    category.fields.insert("label".to_string(), feed::Str(f!(attributes, "label")));
    Ok(category)
}

fn parse_generator<B: Buffer>(parser: &mut XmlDecoder<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<feed::Element> {
    reset_xml_base(attributes, &mut session);
    let mut generator = feed::Element::new("generator".to_string());
    generator.fields.insert("value".to_string(), feed::Str(try!(read_whole_text(parser))));
    for uri in find_from_attr(attributes, "uri").move_iter() {
        generator.fields.insert("uri".to_string(), feed::Str(uri.to_string()));  // TODO
    }
    generator.fields.insert("version".to_string(), feed::Str(f!(attributes, "version")));
    Ok(generator)
}

fn parse_content<B: Buffer>(parser: &mut XmlDecoder<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<feed::Element> {
    reset_xml_base(attributes, &mut session);
    let mut content = feed::Element::new("content".to_string());
    content.fields.insert("value".to_string(), feed::Str(try!(read_whole_text(parser))));
    for content_type in find_from_attr(attributes, "type").move_iter() {
        content.fields.insert("type".to_string(), feed::Str(content_type.to_string()));
    }
    for src in find_from_attr(attributes, "src").move_iter() {
        content.fields.insert("source_uri".to_string(), feed::Str(src.to_string()));  // TODO
    }
    Ok(content)
}

pub fn parse_atom<B: Buffer>(xml: B, feed_url: &str, need_entries: bool) -> DecodeResult<(feed::Element, Option<CrawlerHint>)> {
    let mut parser = XmlDecoder::new(xml::EventReader::new(xml));
    let mut result = None;
    try!(parser.each_child(|p| {
        p.read_event(|p, event| match event {
            &StartElement { ref name, ref attributes, .. } => {
                let atom_xmlns = ATOM_XMLNS_SET.iter().find(|&&atom_xmlns| {
                    name.namespace_ref().map_or(false, |n| n == atom_xmlns)
                }).unwrap();
                let xml_base = get_xml_base(attributes.as_slice()).unwrap_or(feed_url);
                let session = AtomSession { xml_base: xml_base.into_string(),
                                            element_ns: atom_xmlns.into_string() };
                let feed_data = parse_feed(p, feed_url, need_entries, session);
                result = Some(feed_data);
                Ok(())
            }
            &EndDocument => { fail!(); }
            _ => { Ok(()) }
        })
    }));
    match result {
        Some(Ok(r)) => Ok((r, None)),
        Some(Err(e)) => Err(e),
        None => Err(super::base::NoResult),
    }
}
