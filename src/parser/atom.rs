use std::borrow::{IntoCow, ToOwned};
use std::default::Default;
use std::str::FromStr;
use std::string::CowString;

use chrono::{DateTime, FixedOffset};
use xml;

use super::base::{NestedEventReader, DecodeResult,
                  XmlAttribute, XmlElement, XmlName};
use super::base::DecodeError::{AttributeNotFound, SchemaError};
use super::base::NestedEvent::{EndDocument, Nested};
use feed;
use codecs;
use schema::Codec;

static ATOM_XMLNS_SET: [&'static str; 2] = [
    "http://www.w3.org/2005/Atom",
    "http://purl.org/atom/ns#",
];

static XML_XMLNS: &'static str = "http://www.w3.org/XML/1998/namespace";

#[allow(missing_copy_implementations)]
pub struct CrawlerHint;

#[derive(Clone)]
struct AtomSession<'a> {
    xml_base: CowString<'a>,
    element_ns: CowString<'a>,
}

impl<'a> AtomSession<'a> {
    fn reset_xml_base(&mut self, attributes: &[XmlAttribute]) {
        if let Some(new_base) = get_xml_base(&attributes[]) {
            self.xml_base = new_base.to_owned().into_cow();
        }
    }
}

pub fn parse_atom<B: Buffer>(xml: B, feed_url: &str, need_entries: bool)
                             -> DecodeResult<(feed::Feed, Option<CrawlerHint>)>
{
    let mut parser = xml::EventReader::new(xml);
    let mut events = NestedEventReader::new(&mut parser);
    let mut result = None;
    for_each!(event in events.next() {
        match event {
            Nested { name, element } => {
                let atom_xmlns = ATOM_XMLNS_SET.iter().find(|&&atom_xmlns| {
                    name.namespace_as_ref().map_or(false, |n| n == atom_xmlns)
                }).unwrap();
                let session = {
                    let xml_base = get_xml_base(&element.attributes[])
                        .unwrap_or(feed_url);
                    AtomSession {
                        xml_base: xml_base.to_owned().into_cow(),
                        element_ns: atom_xmlns.into_cow()
                    }
                };
                let feed_data = parse_feed(element, feed_url,
                                           need_entries, session);
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

fn get_xml_base(attributes: &[XmlAttribute]) -> Option<&str> {
    attributes.iter().find(|&attr| {
        attr.name.namespace_as_ref().map_or(false, |ns| ns == XML_XMLNS)
    }).map(|attr| &*attr.value)
}

fn name_matches(name: &XmlName, namespace: Option<&str>, local_name: &str) -> bool {
    &name.local_name[] == local_name &&
        match (name.namespace_as_ref(), namespace) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false
        }
}

macro_rules! parse_fields {
    { ($target:ident, $elem:expr, $session:expr)
       $($attr:pat => $var:ident : $plurality:ident by $func:expr;)* } => {
        for_each!(event in $elem.children.next() {
            if let Nested { name, element } = event {
                parse_field! {
                    ($target, name, element, $session)
                    $($attr => $var : $plurality by $func ;)*
                }
            }
        })
    }
}

macro_rules! parse_field {
    { ($target:ident, $name:expr, $elem:expr, $session:expr)
       $($attr:pat => $var:ident : $plurality:ident by $func:expr;)* } => ({
        match &$name.local_name[] {
            $(
                $attr => {
                    let result = try!($func($elem, $session.clone()));
                    assign_field!($plurality : $target.$var, result);
                }
            )*
            _name => { }
        }
    })
}

macro_rules! assign_field {
    (required     : $var:expr, $value:expr) => ( $var = $value );
    (multiple     : $var:expr, $value:expr) => ( $var.push($value) );
    (multiple_opt : $var:expr, $value:expr) => ( $value.map(|v| $var.push(v)) );
    ($_p:ident    : $var:expr, $value:expr) => ( $var = Some($value) )
}

fn parse_feed<B: Buffer>(mut element: XmlElement<B>, feed_url: &str,
                         need_entries: bool, session: AtomSession)
                         -> DecodeResult<feed::Feed> {
    let mut feed: feed::Feed = Default::default();
    for_each!(event in element.children.next() {
        if let Nested { name, element: child } = event {
            if need_entries && name_matches(&name,
                                            Some(&session.element_ns[]),
                                            "entry") {
                let result = try!(parse_entry(child, session.clone()));
                feed.entries.push(result);
                continue;
            }
            parse_field! {
                (feed, name, child, session)
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

fn parse_entry<B: Buffer>(mut element: XmlElement<B>, session: AtomSession)
                          -> DecodeResult<feed::Entry> {
    let mut entry: feed::Entry = Default::default();
    parse_fields! { (entry, element, session)
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

fn parse_source<B: Buffer>(mut element: XmlElement<B>,
                           session: AtomSession) -> DecodeResult<feed::Source> {
    let mut source: feed::Source = Default::default();
    parse_fields! { (source, element, session)
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

fn parse_icon<B: Buffer>(element: XmlElement<B>, mut session: AtomSession)
                         -> DecodeResult<String> {
    session.reset_xml_base(&element.attributes[]);
    let xml_base = session.xml_base.into_owned();
    Ok(xml_base + &try!(element.read_whole_text())[])
}

fn parse_text_construct<B: Buffer>(element: XmlElement<B>,
                                   _session: AtomSession)
                                   -> DecodeResult<feed::Text>
{
    let text_type = match element.get_attr("type") {
        Ok("text/plaln") | Ok("text") => "text",
        Ok("text/html") | Ok("html") => "html",
        Ok("application/xhtml+xml") | Ok("xhtml") => "xhtml",
        Ok(_) => "text",
        Err(AttributeNotFound(_)) => "text",
        Err(e) => { return Err(e); }
    };
    let text = feed::Text::new(text_type, try!(element.read_whole_text()));
    Ok(text)
}

fn parse_person_construct<B: Buffer>(mut element: XmlElement<B>,
                                     mut session: AtomSession)
                                     -> DecodeResult<Option<feed::Person>> {
    session.reset_xml_base(&element.attributes[]);
    let mut person_name = Default::default();
    let mut uri = Default::default();
    let mut email = Default::default();

    for_each!(event in element.children.next() {
        match event {
            Nested { name, element: elem } => {
                let ns = &session.element_ns[];
                if name_matches(&name, Some(ns), "name") {
                    person_name = Some(try!(elem.read_whole_text()));
                } else if name_matches(&name, Some(ns), "uri") {
                    uri = Some(try!(elem.read_whole_text()));
                } else if name_matches(&name, Some(ns), "email") {
                    email = Some(try!(elem.read_whole_text()));
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

fn parse_link<B: Buffer>(element: XmlElement<B>, mut session: AtomSession)
                         -> DecodeResult<feed::Link> {
    session.reset_xml_base(&element.attributes[]);
    Ok(feed::Link {
        uri: try!(element.get_attr("href")).to_string(),
        relation: element.get_attr("rel").unwrap_or("alternate").to_string(),
        mimetype: element.get_attr("type").ok().map(|v| v.to_string()),
        language: element.get_attr("hreflang").ok().map(|v| v.to_string()),
        title: element.get_attr("title").ok().map(|v| v.to_string()),
        byte_size: element.get_attr("length").ok().and_then(FromStr::from_str),
    })
}

fn parse_datetime<B: Buffer>(element: XmlElement<B>, _session: AtomSession)
                             -> DecodeResult<DateTime<FixedOffset>> {
    match codecs::RFC3339.decode(&*try!(element.read_whole_text())) {
        Ok(v) => Ok(v),
        Err(e) => Err(SchemaError(e)),
    }
}

fn parse_category<B: Buffer>(element: XmlElement<B>, _session: AtomSession)
                             -> DecodeResult<feed::Category> {
    Ok(feed::Category {
        term: try!(element.get_attr("term")).to_string(),
        scheme_uri: element.get_attr("scheme").ok().map(|v| v.to_string()),
        label: element.get_attr("label").ok().map(|v| v.to_string()),
    })
}

fn parse_generator<B: Buffer>(element: XmlElement<B>, mut session: AtomSession)
                              -> DecodeResult<feed::Generator> {
    session.reset_xml_base(&element.attributes[]);
    let uri = element.get_attr("uri").ok().map(|v| v.to_string());  // TODO
    let version = element.get_attr("version").ok().map(|v| v.to_string());
    Ok(feed::Generator {
        uri: uri,
        version: version,
        value: try!(element.read_whole_text()),
    })
}

fn parse_content<B: Buffer>(element: XmlElement<B>, mut session: AtomSession)
                            -> DecodeResult<feed::Content> {
    session.reset_xml_base(&element.attributes[]);
    let source_uri = element.get_attr("src").ok().map(|v| v.to_string());
    Ok(feed::Content {
        text: try!(parse_text_construct(element, session.clone())),
        source_uri: source_uri,  // TODO
    })
}
