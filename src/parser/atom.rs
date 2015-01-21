use std::borrow::{IntoCow, ToOwned};
use std::default::Default;
use std::str::FromStr;
use std::string::CowString;

use chrono::{DateTime, FixedOffset};
use xml;
use xml::name::OwnedName;
use xml::attribute::OwnedAttribute;

use super::base::{NestedEventReader, DecodeResult};
use super::base::DecodeError::{AttributeNotFound, SchemaError};
use super::base::NestedEvent::{EndDocument, Element, Characters};
use feed;
use codecs;
use schema::Codec;

static ATOM_XMLNS_SET: [&'static str; 2] = [
    "http://www.w3.org/2005/Atom",
    "http://purl.org/atom/ns#",
];

static XML_XMLNS: &'static str = "http://www.w3.org/XML/1998/namespace";

pub struct CrawlerHint;

#[derive(Clone)]
struct AtomSession<'a> {
    xml_base: CowString<'a>,
    element_ns: CowString<'a>,
}

impl<'a> AtomSession<'a> {
    fn reset_xml_base(&mut self, attributes: &[OwnedAttribute]) {
        if let Some(new_base) = get_xml_base(&attributes[]) {
            self.xml_base = new_base.to_owned().into_cow();
        }
    }
}

pub fn parse_atom<B: Buffer>(xml: B, feed_url: &str, need_entries: bool) -> DecodeResult<(feed::Feed, Option<CrawlerHint>)> {
    let mut parser = xml::EventReader::new(xml);
    let mut events = NestedEventReader::new(&mut parser);
    let mut result = None;
    for_each!(event in events.next() {
        match event {
            Element { name, attributes, children, .. } => {
                let atom_xmlns = ATOM_XMLNS_SET.iter().find(|&&atom_xmlns| {
                    name.namespace_as_ref().map_or(false, |n| n == atom_xmlns)
                }).unwrap();
                let xml_base = get_xml_base(&*attributes).unwrap_or(feed_url);
                let session = AtomSession { xml_base: xml_base.into_cow(),
                                            element_ns: atom_xmlns.into_cow() };
                let feed_data = parse_feed(children, feed_url, need_entries, session);
                result = Some(feed_data);
            }
            EndDocument => { panic!(); }
            _ => { }
        }
    });
    match result {
        Some(Ok(r)) => Ok((r, None)),
        Some(Err(e)) => Err(e),
        None => Err(super::base::DecodeError::NoResult),
    }
}

fn get_xml_base(attributes: &[OwnedAttribute]) -> Option<&str> {
    attributes.iter().find(|&attr| {
        attr.name.namespace_as_ref().map_or(false, |ns| ns == XML_XMLNS)
    }).map(|attr| &*attr.value)
}

fn name_matches(name: &OwnedName, namespace: Option<&str>, local_name: &str) -> bool {
    &name.local_name[] == local_name &&
        match (name.namespace_as_ref(), namespace) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false
        }
}

macro_rules! parse_fields {
    { ($target:ident, $parser:expr, $session:expr)
       $($attr:pat => $var:ident : $plurality:ident by $func:expr;)* } => {
        for_each!(event in $parser.next() {
            if let Element { name, attributes, children, .. } = event {
                parse_field! {
                    ($target, &name.local_name[],
                     children, &attributes[], $session)
                    $($attr => $var : $plurality by $func ;)*
                }
            }
        })
    }
}

macro_rules! parse_field {
    { ($target:ident, $name:expr, $parser:expr, $attributes:expr, $session:expr)
       $($attr:pat => $var:ident : $plurality:ident by $func:expr;)* } => {
        match $name {
            $(
                $attr => {
                    let result = try!($func($parser, $attributes,
                                            $session.clone()));
                    assign_field!($plurality : $target.$var, result);
                }
            )*
            _name => { }
        }
    }
}

macro_rules! assign_field {
    (required     : $var:expr, $value:expr) => ( $var = $value );
    (multiple     : $var:expr, $value:expr) => ( $var.push($value) );
    (multiple_opt : $var:expr, $value:expr) => ( $value.map(|v| $var.push(v)) );
    ($_p:ident    : $var:expr, $value:expr) => ( $var = Some($value) )
}

fn parse_feed<B: Buffer>(mut parser: NestedEventReader<B>, feed_url: &str,
                         need_entries: bool, session: AtomSession)
                         -> DecodeResult<feed::Feed>
{
    let mut feed: feed::Feed = Default::default();
    for_each!(event in parser.next() {
        if let Element { name, attributes, children, .. } = event {
            if need_entries && name_matches(&name, Some(&session.element_ns[]),
                                            "entry") {
                let result = try!(parse_entry(children, &attributes[],
                                              session.clone()));
                feed.entries.push(result);
                continue;
            }
            parse_field! {
                (feed, &name.local_name[], children, &attributes[], session)
                "id"          => id:         required by parse_icon;
                "title"       => title:      required by parse_text_construct;
                "link"        => links:      multiple by parse_link;
                "updated"     => updated_at: required by parse_datetime;
                "author"      => authors: multiple_opt
                                 by parse_person_construct;
                "contributor" => contributors: multiple_opt
                                 by parse_person_construct;
                "category"    => categories: multiple by parse_category;
                "rights"      => rights:     optional by parse_text_construct;
                "subtitle"    => subtitle:   optional by parse_text_construct;
                "generator"   => generator:  optional by parse_generator;
                "logo"        => logo:       optional by parse_icon;
                "icon"        => icon:       optional by parse_icon;
            }
        }
    });

    if feed.id.is_empty() {
        feed.id = feed_url.to_string();
    }

    Ok(feed)
}

fn parse_entry<B: Buffer>(mut parser: NestedEventReader<B>,
                          _attributes: &[OwnedAttribute],
                          session: AtomSession) -> DecodeResult<feed::Entry> {
    let mut entry: feed::Entry = Default::default();
    parse_fields! { (entry, parser, session)
        "id"          => id:           required     by parse_icon;
        "title"       => title:        required     by parse_text_construct;
        "link"        => links:        multiple     by parse_link;
        "updated"     => updated_at:   required     by parse_datetime;
        "modified"    => updated_at:   required     by parse_datetime;
        "author"      => authors:      multiple_opt by parse_person_construct;
        "contributor" => contributors: multiple_opt by parse_person_construct;
        "category"    => categories:   multiple     by parse_category;
        "rights"      => rights:       optional     by parse_text_construct;
        "published"   => published_at: optional     by parse_datetime;
        "summary"     => summary:      optional     by parse_text_construct;
        "content"     => content:      optional     by parse_content;
        "source"      => source:       optional     by parse_source;
    }
    Ok(entry)
}

fn parse_source<B: Buffer>(mut parser: NestedEventReader<B>,
                           _attributes: &[OwnedAttribute],
                           session: AtomSession) -> DecodeResult<feed::Source> {
    let mut source: feed::Source = Default::default();
    parse_fields! { (source, parser, session)
        "id"          => id:           required     by parse_icon;
        "title"       => title:        required     by parse_text_construct;
        "link"        => links:        multiple     by parse_link;
        "updated"     => updated_at:   required     by parse_datetime;
        "author"      => authors:      multiple_opt by parse_person_construct;
        "contributor" => contributors: multiple_opt by parse_person_construct;
        "category"    => categories:   multiple     by parse_category;
        "rights"      => rights:       optional     by parse_text_construct;
        "subtitle"    => subtitle:     optional     by parse_text_construct;
        "generator"   => generator:    optional     by parse_generator;
        "logo"        => logo:         optional     by parse_icon;
        "icon"        => icon:         optional     by parse_icon;
    }
    Ok(source)
}

fn read_whole_text<B: Buffer>(mut parser: NestedEventReader<B>) -> DecodeResult<String> {
    let mut text = String::new();
    for_each!(event in parser.next() {
        match event {
            Characters(s) => { text.push_str(&*s); }
            _ => { }
        }
    });
    Ok(text)
}

fn find_from_attr<'a>(attr: &'a [OwnedAttribute], key: &str) -> Option<&'a str> {
    attr.iter()
        .find(|&attr| attr.name.local_name == key)
        .map(|e| &*e.value)
}

macro_rules! f {
    ($attr:expr, $k:expr) => (
        match find_from_attr($attr, $k) {
            Some(v) => { v.to_string() }
            None => { return Err(AttributeNotFound($k.to_string())); }
        }
    )
}

fn parse_icon<B: Buffer>(parser: NestedEventReader<B>, attributes: &[OwnedAttribute], mut session: AtomSession) -> DecodeResult<String> {
    session.reset_xml_base(attributes);
    let xml_base = session.xml_base.into_owned();
    Ok(xml_base + &try!(read_whole_text(parser))[])
}

fn parse_text_construct<B: Buffer>(parser: NestedEventReader<B>, attributes: &[OwnedAttribute], _session: AtomSession) -> DecodeResult<feed::Text> {
    let text_type = match find_from_attr(attributes, "type") {
        Some("text/plaln") | Some("text") => feed::TextType::Text,
        Some("text/html") | Some("html") => feed::TextType::Html,
        Some(_) => { feed::TextType::Text },  // TODO
        None => feed::TextType::Text,
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

fn parse_person_construct<B: Buffer>(mut parser: NestedEventReader<B>, attributes: &[OwnedAttribute], mut session: AtomSession) -> DecodeResult<Option<feed::Person>> {
    session.reset_xml_base(attributes);
    let mut person_name = Default::default();
    let mut uri = Default::default();
    let mut email = Default::default();

    for_each!(event in parser.next() {
        match event {
            Element { name, children, .. } => {
                if name_matches(&name, Some(&*session.element_ns), "name") {
                    person_name = Some(try!(read_whole_text(children)));
                } else if name_matches(&name, Some(&*session.element_ns), "uri") {
                    uri = Some(try!(read_whole_text(children)));
                } else if name_matches(&name, Some(&*session.element_ns), "email") {
                    email = Some(try!(read_whole_text(children)));
                }
            }
            _ => { }
        }
    });
    let name = match person_name {
        Some(n) => n,
        None => match uri.clone().or_else(|| email.clone()) {
            Some(v) => { v }
            None => { return Ok(None); }
        }
    };
    Ok(Some(feed::Person { name: name, uri: uri, email: email }))
}

fn parse_link<B: Buffer>(_parser: NestedEventReader<B>, attributes: &[OwnedAttribute], mut session: AtomSession) -> DecodeResult<feed::Link> {
    session.reset_xml_base(attributes);
    Ok(feed::Link {
        uri: f!(attributes, "href"),
        relation: find_from_attr(attributes, "rel").unwrap_or("alternate").to_string(),
        mimetype: find_from_attr(attributes, "type").map(|v| v.to_string()),
        language: find_from_attr(attributes, "hreflang").map(|v| v.to_string()),
        title: find_from_attr(attributes, "title").map(|v| v.to_string()),
        byte_size: find_from_attr(attributes, "length").and_then(FromStr::from_str),
    })
}

fn parse_datetime<B: Buffer>(parser: NestedEventReader<B>, _attributes: &[OwnedAttribute], _session: AtomSession) -> DecodeResult<DateTime<FixedOffset>> {
    match codecs::RFC3339.decode(&*try!(read_whole_text(parser))) {
        Ok(v) => Ok(v),
        Err(e) => Err(SchemaError(e)),
    }
}

fn parse_category<B: Buffer>(_parser: NestedEventReader<B>, attributes: &[OwnedAttribute], _session: AtomSession) -> DecodeResult<feed::Category> {
    Ok(feed::Category {
        term: f!(attributes, "term"),
        scheme_uri: find_from_attr(attributes, "scheme").map(|v| v.to_string()),
        label: find_from_attr(attributes, "label").map(|v| v.to_string()),
    })
}

fn parse_generator<B: Buffer>(parser: NestedEventReader<B>, attributes: &[OwnedAttribute], mut session: AtomSession) -> DecodeResult<feed::Generator> {
    session.reset_xml_base(attributes);
    Ok(feed::Generator {
        uri: find_from_attr(attributes, "uri").map(|v| v.to_string()),  // TODO
        version: find_from_attr(attributes, "version").map(|v| v.to_string()),
        value: try!(read_whole_text(parser)),
    })
}

fn parse_content<B: Buffer>(parser: NestedEventReader<B>, attributes: &[OwnedAttribute], mut session: AtomSession) -> DecodeResult<feed::Content> {
    session.reset_xml_base(attributes);
    Ok(feed::Content {
        text: try!(parse_text_construct(parser, attributes, session.clone())),
        source_uri: find_from_attr(attributes, "src").map(|v| v.to_string()),  // TODO
    })
}
