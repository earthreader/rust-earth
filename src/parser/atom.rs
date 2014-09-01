use std::default::Default;
use std::from_str::from_str;

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

pub fn parse_atom<B: Buffer>(xml: B, feed_url: &str, need_entries: bool) -> DecodeResult<(feed::Feed, Option<CrawlerHint>)> {
    let mut parser = XmlDecoder::new(xml::EventReader::new(xml));
    let mut result = None;
    try!(parser.each_child(|p| {
        p.read_event(|p, event| match event {
            StartElement { ref name, ref attributes, .. } => {
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
            EndDocument => { fail!(); }
            _ => { Ok(()) }
        })
    }));
    match result {
        Some(Ok(r)) => Ok((r, None)),
        Some(Err(e)) => Err(e),
        None => Err(super::base::NoResult),
    }
}

fn get_xml_base<'a>(attributes: &'a [Attribute]) -> Option<&'a str> {
    attributes.iter().find(|&attr| {
        attr.name.namespace_ref().map_or(false, |ns| ns == XML_XMLNS)
    }).map(|attr| attr.value.as_slice())
}

fn name_matches<'a>(name: &'a Name, namespace: Option<&'a str>, local_name: &str) -> bool {
    name.namespace.as_ref().map(|n| n.as_slice()) == namespace && name.local_name.as_slice() == local_name
}

macro_rules! unexpected (
    ($name:expr) => ( fail!("Unexpected field: {}", $name) )
)

fn parse_feed<B: Buffer>(parser: &mut XmlDecoder<B>, feed_url: &str, need_entries: bool, session: AtomSession) -> DecodeResult<feed::Feed> {
    let mut id = Default::default();
    let mut title = Default::default();
    let mut links: Vec<_> = Default::default();
    let mut updated_at = Default::default();
    let mut authors: Vec<_> = Default::default();
    let mut contributors: Vec<_> = Default::default();
    let mut categories: Vec<_> = Default::default();
    let mut rights = Default::default();
    let mut subtitle = Default::default();
    let mut generator = Default::default();
    let mut logo = Default::default();
    let mut icon = Default::default();
    let mut entries: Vec<_> = Default::default();

    try!(parser.each_child(|p| {
        p.read_event(|p, event| {
            match event {
                StartElement { ref name, ref attributes, .. }
                if ["id", "icon", "logo"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_icon(p, attributes.as_slice(), session.clone()));
                    match name.local_name.as_slice() {
                        "id" => id = Some(result),
                        "icon" => icon = Some(result),
                        "logo" => logo = Some(result),
                        x => unexpected!(x),
                    }
                }
                StartElement { ref name, ref attributes, .. }
                if ["title", "rights", "subtitle"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_text_construct(p, attributes.as_slice()));
                    match name.local_name.as_slice() {
                        "title" => title = Some(result),
                        "rights" => rights = Some(result),
                        "subtitle" => subtitle = Some(result),
                        x => unexpected!(x),
                    }
                }
                StartElement { ref name, ref attributes, .. }
                if ["author", "contributor"].contains(&name.local_name.as_slice()) => {
                    match try!(parse_person_construct(p, attributes.as_slice(), session.clone())) {
                        Some(result) => match name.local_name.as_slice() {
                            "author" => authors.push(result),
                            "contributor" => contributors.push(result),
                            x => unexpected!(x),
                        },
                        None => { }
                    }
                }
                StartElement { ref name, ref attributes, .. }
                if "link" == name.local_name.as_slice() => {
                    let result = try!(parse_link(attributes.as_slice(), session.clone()));
                    links.push(result);
                }
                StartElement { ref name, ref attributes, .. }
                if ["updated", "modified"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_datetime(p));
                    updated_at = Some(result);
                }
                StartElement { ref name, ref attributes, .. }
                if "category" == name.local_name.as_slice() => {
                    let result = try!(parse_category(attributes.as_slice()));
                    categories.push(result);
                }
                StartElement { ref name, ref attributes, .. }
                if "generator" == name.local_name.as_slice() => {
                    let result = try!(parse_generator(p, attributes.as_slice(), session.clone()));
                    generator = Some(result);
                }
                StartElement { ref name, ref attributes, .. }
                if need_entries && name_matches(name, Some(session.element_ns.as_slice()), "entry") => {
                    let result = try!(parse_entry(p, session.clone()));
                    entries.push(result);
                }

                _otherwise => { }
            }
            Ok(())
        })
    }));

    let feed = feed::Feed {
        source: feed::Source {
            metadata: feed::Metadata {
                id: id.unwrap_or_else(|| feed_url.to_string()),
                title: title.unwrap(),
                links: feed::LinkList(links),
                updated_at: updated_at.unwrap(),
                authors: authors,
                contributors: contributors,
                categories: categories,
                rights: rights,
            },
            subtitle: subtitle,
            generator: generator,
            logo: logo,
            icon: icon,
        },
        entries: entries,
    };
    Ok(feed)
}

fn parse_entry<B: Buffer>(parser: &mut XmlDecoder<B>, session: AtomSession) -> DecodeResult<feed::Entry> {
    let mut id = Default::default();
    let mut title = Default::default();
    let mut links = Vec::new();
    let mut updated_at = Default::default();
    let mut authors = Vec::new();
    let mut contributors = Vec::new();
    let mut categories = Vec::new();
    let mut rights = Default::default();
    let mut published_at = Default::default();
    let mut summary = Default::default();
    let mut content = Default::default();
    let mut source = Default::default();
    let mut read = Default::default();
    let mut starred = Default::default();

    try!(parser.each_child(|p| {
        p.read_event(|p, event| {
            match event {
                StartElement { ref name, ref attributes, .. }
                if "source" == name.local_name.as_slice() => {
                    let result = try!(parse_source(p, session.clone()));
                    source = Some(result);
                }
                StartElement { ref name, ref attributes, .. }
                if "id" == name.local_name.as_slice() => {
                    let result = try!(parse_icon(p, attributes.as_slice(), session.clone()));
                    id = Some(result);
                }
                StartElement { ref name, ref attributes, .. }
                if ["title", "rights", "summary"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_text_construct(p, attributes.as_slice()));
                    match name.local_name.as_slice() {
                        "title" => title = Some(result),
                        "rights" => rights = Some(result),
                        "summary" => summary = Some(result),
                        x => unexpected!(x),
                    }
                }
                StartElement { ref name, ref attributes, .. }
                if ["author", "contributor"].contains(&name.local_name.as_slice()) => {
                    match try!(parse_person_construct(p, attributes.as_slice(), session.clone())) {
                        Some(result) => match name.local_name.as_slice() {
                            "author" => authors.push(result),
                            "contributor" => contributors.push(result),
                            x => unexpected!(x),
                        },
                        None => { }
                    }
                }
                StartElement { ref name, ref attributes, .. }
                if "link" == name.local_name.as_slice() => {
                    let result = try!(parse_link(attributes.as_slice(), session.clone()));
                    links.push(result);
                }
                StartElement { ref name, ref attributes, .. }
                if ["updated", "published", "modified"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_datetime(p));
                    let field_name = if name.local_name.as_slice() == "published" {
                        published_at = Some(result);
                    } else {
                        updated_at = Some(result);
                    };
                }
                StartElement { ref name, ref attributes, .. }
                if "category" == name.local_name.as_slice() => {
                    let result = try!(parse_category(attributes.as_slice()));
                    categories.push(result);
                }
                StartElement { ref name, ref attributes, .. }
                if "content" == name.local_name.as_slice() => {
                    let result = try!(parse_content(p, attributes.as_slice(), session.clone()));
                    content = Some(result);
                }

                _otherwise => { }
            }
            Ok(())
        })
    }));

    let entry = feed::Entry {
        metadata: feed::Metadata {
            id: id.unwrap(),
            title: title.unwrap(),
            links: feed::LinkList(links),
            updated_at: updated_at.unwrap(),
            authors: authors,
            contributors: contributors,
            categories: categories,
            rights: rights,
        },
        published_at: published_at,
        summary: summary,
        content: content,
        source: source,
        read: read,
        starred: starred,
    };
    Ok(entry)
}

fn parse_source<B: Buffer>(parser: &mut XmlDecoder<B>, session: AtomSession) -> DecodeResult<feed::Source> {
    let mut id = Default::default();
    let mut title = Default::default();
    let mut links = Vec::new();
    let mut updated_at = Default::default();
    let mut authors = Vec::new();
    let mut contributors = Vec::new();
    let mut categories = Vec::new();
    let mut rights = Default::default();
    let mut subtitle = Default::default();
    let mut generator = Default::default();
    let mut logo = Default::default();
    let mut icon = Default::default();

    try!(parser.each_child(|p| {
        p.read_event(|p, event| {
            match event {
                StartElement { ref name, ref attributes, .. }
                if ["id", "icon", "logo"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_icon(p, attributes.as_slice(), session.clone()));
                    match name.local_name.as_slice() {
                        "id" => id = Some(result),
                        "icon" => icon = Some(result),
                        "logo" => logo = Some(result),
                        x => unexpected!(x),
                    }
                }
                StartElement { ref name, ref attributes, .. }
                if ["title", "rights", "subtitle"].contains(&name.local_name.as_slice()) => {
                    let result = try!(parse_text_construct(p, attributes.as_slice()));
                    match name.local_name.as_slice() {
                        "title" => title = Some(result),
                        "rights" => rights = Some(result),
                        "subtitle" => subtitle = Some(result),
                        x => unexpected!(x),
                    }
                }
                StartElement { ref name, ref attributes, .. }
                if ["author", "contributor"].contains(&name.local_name.as_slice()) => {
                    match try!(parse_person_construct(p, attributes.as_slice(), session.clone())) {
                        Some(result) => match name.local_name.as_slice() {
                            "author" => authors.push(result),
                            "contributor" => contributors.push(result),
                            x => unexpected!(x),
                        },
                        None => { }
                    }
                }
                StartElement { ref name, ref attributes, .. }
                if "link" == name.local_name.as_slice() => {
                    let result = try!(parse_link(attributes.as_slice(), session.clone()));
                    links.push(result);
                }
                StartElement { ref name, ref attributes, .. }
                if "updated" == name.local_name.as_slice() => {
                    let result = try!(parse_datetime(p));
                    updated_at = Some(result);
                }
                StartElement { ref name, ref attributes, .. }
                if "category" == name.local_name.as_slice() => {
                    let result = try!(parse_category(attributes.as_slice()));
                    categories.push(result);
                }
                StartElement { ref name, ref attributes, .. }
                if "generator" == name.local_name.as_slice() => {
                    let result = try!(parse_generator(p, attributes.as_slice(), session.clone()));
                    generator = Some(result);
                }

                _otherwise => { }
            }
            Ok(())
        })
    }));

    let source = feed::Source {
        metadata: feed::Metadata {
            id: id.unwrap(),
            title: title.unwrap(),
            links: feed::LinkList(links),
            updated_at: updated_at.unwrap(),
            authors: authors,
            contributors: contributors,
            categories: categories,
            rights: rights,
        },
        subtitle: subtitle,
        generator: generator,
        logo: logo,
        icon: icon,
    };
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
                Characters(s) => { text.push_str(s.as_slice()); }
                _ => { }
            }
            Ok(())
        })
    }));
    Ok(text)
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

fn parse_icon<B: Buffer>(parser: &mut XmlDecoder<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<String> {
    reset_xml_base(attributes, &mut session);
    Ok(session.xml_base.append(try!(read_whole_text(parser)).as_slice()))
}

fn parse_text_construct<B: Buffer>(parser: &mut XmlDecoder<B>, attributes: &[Attribute]) -> DecodeResult<feed::Text> {
    let text_type = match find_from_attr(attributes, "type") {
        Some("text/plaln") | Some("text") => feed::text_type::Text,
        Some("text/html") | Some("html") => feed::text_type::Html,
        Some(_) => { feed::text_type::Text },  // TODO
        None => feed::text_type::Text,
    };
    let text = feed::Text {
        type_: text_type,
        value: try!(read_whole_text(parser)),
    };
    // else if text_type == "xhtml" {
    //     text.fields.insert("value".to_string(), feed::Str("".to_string()));  // TODO
    // }
    Ok(text)
}

fn parse_person_construct<B: Buffer>(parser: &mut XmlDecoder<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<Option<feed::Person>> {
    reset_xml_base(attributes, &mut session);
    let mut person_name = Default::default();
    let mut uri = Default::default();
    let mut email = Default::default();

    try!(parser.each_child(|p| {
        p.read_event(|p, event| {
            match event {
                StartElement { ref name, .. }
                if name_matches(name, Some(session.element_ns.as_slice()), "name") => {
                    person_name = Some(try!(read_whole_text(p)));
                }
                StartElement { ref name, .. }
                if name_matches(name, Some(session.element_ns.as_slice()), "uri") => {
                    uri = Some(try!(read_whole_text(p)));
                }
                StartElement { ref name, .. }
                if name_matches(name, Some(session.element_ns.as_slice()), "email") => {
                    email = Some(try!(read_whole_text(p)));
                }
                _ => { }
            }
            Ok(())
        })
    }));
    let name = match person_name {
        Some(n) => n,
        None => match uri.clone().or_else(|| email.clone()) {
            Some(v) => { v }
            None => { return Ok(None); }
        }
    };
    Ok(Some(feed::Person { name: name, uri: uri, email: email }))
}

fn parse_link(attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<feed::Link> {
    reset_xml_base(attributes, &mut session);
    Ok(feed::Link {
        uri: f!(attributes, "href"),
        relation: find_from_attr(attributes, "rel").unwrap_or("alternate").to_string(),
        mimetype: find_from_attr(attributes, "type").map(|v| v.to_string()),
        language: find_from_attr(attributes, "hreflang").map(|v| v.to_string()),
        title: find_from_attr(attributes, "title").map(|v| v.to_string()),
        byte_size: find_from_attr(attributes, "length").and_then(from_str),
    })
}

fn parse_datetime<B: Buffer>(parser: &mut XmlDecoder<B>) -> DecodeResult<DateTime<FixedOffset>> {
    match schema::RFC3339.decode(try!(read_whole_text(parser)).as_slice()) {
        Ok(v) => Ok(v),
        Err(e) => Err(SchemaError(e)),
    }
}

fn parse_category(attributes: &[Attribute]) -> DecodeResult<feed::Category> {
    Ok(feed::Category {
        term: f!(attributes, "term"),
        scheme_uri: find_from_attr(attributes, "scheme").map(|v| v.to_string()),
        label: find_from_attr(attributes, "label").map(|v| v.to_string()),
    })
}

fn parse_generator<B: Buffer>(parser: &mut XmlDecoder<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<feed::Generator> {
    reset_xml_base(attributes, &mut session);
    Ok(feed::Generator {
        uri: find_from_attr(attributes, "uri").map(|v| v.to_string()),  // TODO
        version: find_from_attr(attributes, "version").map(|v| v.to_string()),
        value: try!(read_whole_text(parser)),
    })
}

fn parse_content<B: Buffer>(parser: &mut XmlDecoder<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<feed::Content> {
    reset_xml_base(attributes, &mut session);
    Ok(feed::Content {
        text: try!(parse_text_construct(parser, attributes)),
        source_uri: find_from_attr(attributes, "src").map(|v| v.to_string()),  // TODO
    })
}
