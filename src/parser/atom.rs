use std::default::Default;
use std::from_str::from_str;

use chrono::{DateTime, FixedOffset};
use xml;
use xml::common::{Name, Attribute};

use super::base::{NestedEventReader, DecodeResult, AttributeNotFound, SchemaError};
use super::base::events::{EndDocument, Element, Characters};
use feed;
use codecs;
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
    let mut parser = xml::EventReader::new(xml);
    let mut events = NestedEventReader::new(&mut parser);
    let mut result = None;
    for_each!(event in events.next() {
        match event {
            Element { name, attributes, children, .. } => {
                let atom_xmlns = ATOM_XMLNS_SET.iter().find(|&&atom_xmlns| {
                    name.namespace_ref().map_or(false, |n| n == atom_xmlns)
                }).unwrap();
                let xml_base = get_xml_base(attributes.as_slice()).unwrap_or(feed_url);
                let session = AtomSession { xml_base: xml_base.into_string(),
                                            element_ns: atom_xmlns.into_string() };
                let feed_data = parse_feed(children, feed_url, need_entries, session);
                result = Some(feed_data);
            }
            EndDocument => { fail!(); }
            _ => { }
        }
    })
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

macro_rules! parse_schema (
    { $parser:expr, $session:expr, $result:ty; $($var:ident as $attr:ident by $func:expr;)* } => {
        define_variables!($($var),*);
        parse_fields! { $parser; $($var as $attr by $func;)* };
        let result = build_data!($result_ty);
        Ok(result)
    }
)

macro_rules! define_variables (
    ($($var:ident: $plurality:ident),+) => {
        let gen_names!($($var),+) = gen_defaults!($($var: $plurality),*);
    }
)

macro_rules! gen_names (
    ($($var:ident),+) => ( ($( mut $var ),+) )
)

macro_rules! gen_defaults (
    ($($var:ident: $plurality:ident),+) => ( ($( gen_default!($var, $plurality) ),+) )
)

macro_rules! gen_default (
    ($var:ident, multiple)         => ( Vec::new() );
    ($var:ident, $plurality:ident) => ( Default::default() )
)

macro_rules! parse_fields (
    { ($parser:expr, $session:expr) $($attr:expr => $var:ident: $plurality:ident by $func:expr;)* } => {
        for_each!(event in $parser.next() {
            match event {
                Element { name, attributes, children, .. } => match name {
                    $(
                        Name { ref local_name, .. }
                        if $attr == local_name.as_slice() => {
                            let result = try!($func(children, attributes.as_slice(), $session.clone()));
                            assign_field!($var: $plurality, result);
                        }
                        )*
                        _name => { }
                },
                _otherwise => { }
            }
        })
    }
)

macro_rules! assign_field (
    ($var:ident: multiple, $value:expr) => ( $var.push($value) );
    ($var:ident: $p:ident, $value:expr) => ( $var = Some($value) )
)

fn parse_feed<B: Buffer>(mut parser: NestedEventReader<B>, feed_url: &str, need_entries: bool, session: AtomSession) -> DecodeResult<feed::Feed> {
    define_variables!(
        id: required,
        title: required,
        links: multiple,
        updated_at: required,
        authors: multiple,
        contributors: multiple,
        categories: multiple,
        rights: optional,
        subtitle: optional,
        generator: optional,
        logo: optional,
        icon: optional,
        entries: multiple
    );

    for_each!(event in parser.next() {
        match event {
            Element { name, attributes, children, .. } => match name {
                Name { ref local_name, .. }
                if ["id", "icon", "logo"].contains(&local_name.as_slice()) => {
                    let result = try!(parse_icon(children, attributes.as_slice(), session.clone()));
                    match local_name.as_slice() {
                        "id" => id = Some(result),
                        "icon" => icon = Some(result),
                        "logo" => logo = Some(result),
                        x => unexpected!(x),
                    }
                }
                Name { ref local_name, .. }
                if ["title", "rights", "subtitle"].contains(&local_name.as_slice()) => {
                    let result = try!(parse_text_construct(children, attributes.as_slice(), session.clone()));
                    match local_name.as_slice() {
                        "title" => title = Some(result),
                        "rights" => rights = Some(result),
                        "subtitle" => subtitle = Some(result),
                        x => unexpected!(x),
                    }
                }
                Name { ref local_name, .. }
                if ["author", "contributor"].contains(&local_name.as_slice()) => {
                    match try!(parse_person_construct(children, attributes.as_slice(), session.clone())) {
                        Some(result) => match local_name.as_slice() {
                            "author" => authors.push(result),
                            "contributor" => contributors.push(result),
                            x => unexpected!(x),
                        },
                        None => { }
                    }
                }
                Name { ref local_name, .. }
                if "link" == local_name.as_slice() => {
                    let result = try!(parse_link(children, attributes.as_slice(), session.clone()));
                    links.push(result);
                }
                Name { ref local_name, .. }
                if ["updated", "modified"].contains(&local_name.as_slice()) => {
                    let result = try!(parse_datetime(children, attributes.as_slice(), session.clone()));
                    updated_at = Some(result);
                }
                Name { ref local_name, .. }
                if "category" == local_name.as_slice() => {
                    let result = try!(parse_category(children, attributes.as_slice(), session.clone()));
                    categories.push(result);
                }
                Name { ref local_name, .. }
                if "generator" == local_name.as_slice() => {
                    let result = try!(parse_generator(children, attributes.as_slice(), session.clone()));
                    generator = Some(result);
                }
                ref name if need_entries && name_matches(name, Some(session.element_ns.as_slice()), "entry") => {
                    let result = try!(parse_entry(children, attributes.as_slice(), session.clone()));
                    entries.push(result);
                }

                _name => { }
            },

            _otherwise => { }
        }
    })

    let mut feed = feed::Feed::new(
        id.unwrap_or_else(|| feed_url.to_string()),
        title.unwrap(),
        updated_at.unwrap());
    feed.source.metadata.links = feed::LinkList(links);
    feed.source.metadata.authors = authors;
    feed.source.metadata.contributors = contributors;
    feed.source.metadata.categories = categories;
    feed.source.metadata.rights = rights;
    feed.source.subtitle = subtitle;
    feed.source.generator = generator;
    feed.source.logo = logo;
    feed.source.icon = icon;
    feed.entries = entries;
    Ok(feed)
}

fn parse_entry<B: Buffer>(mut parser: NestedEventReader<B>, _attributes: &[Attribute], session: AtomSession) -> DecodeResult<feed::Entry> {
    define_variables!(
        id: required,
        title: required,
        links: multiple,
        updated_at: required,
        authors: multiple,
        contributors: multiple,
        categories: multiple,
        rights: optional,
        published_at: optional,
        summary: optional,
        content: optional,
        source: optional
    );

    parse_fields! { (parser, session)
        "id"          => id: required by parse_icon;
        "title"       => title: required by parse_text_construct;
        "link"        => links: multiple by parse_link;
        "updated"     => updated_at: required by parse_datetime;
        "modified"    => updated_at: required by parse_datetime;
        "author"      => authors: multiple by parse_person_construct;
        "contributor" => contributors: multiple by parse_person_construct;
        "category"    => categories: multiple by parse_category;
        "rights"      => rights: optional by parse_text_construct;
        "published"   => published_at: optional by parse_datetime;
        "summary"     => summary: optional by parse_text_construct;
        "content"     => content: optional by parse_content;
        "source"      => source: optional by parse_source;
    }

    let mut entry = feed::Entry::new(
        id.unwrap(),
        title.unwrap(),
        updated_at.unwrap());
    entry.metadata.links = feed::LinkList(links);
    entry.metadata.authors.push_all_move(authors.move_iter().filter_map(|v| v).collect());
    entry.metadata.contributors.push_all_move(contributors.move_iter().filter_map(|v| v).collect());
    entry.metadata.categories.push_all_move(categories);
    entry.metadata.rights = rights;
    entry.published_at = published_at;
    entry.summary = summary;
    entry.content = content;
    entry.source = source;
    Ok(entry)
}

fn parse_source<B: Buffer>(mut parser: NestedEventReader<B>, _attributes: &[Attribute], session: AtomSession) -> DecodeResult<feed::Source> {
    define_variables!(
        id: required,
        title: required,
        links: multiple,
        updated_at: required,
        authors: multiple,
        contributors: multiple,
        categories: multiple,
        rights: optional,
        subtitle: optional,
        generator: optional,
        logo: optional,
        icon: optional
    );

    parse_fields! { (parser, session)
        "id" => id: required by parse_icon;
        "title" => title: required by parse_text_construct;
        "link" => links: multiple by parse_link;
        "updated" => updated_at: required by parse_datetime;
        "author" => authors: multiple by parse_person_construct;
        "contributor" => contributors: multiple by parse_person_construct;
        "category" => categories: multiple by parse_category;
        "rights" => rights: optional by parse_text_construct;
        "subtitle" => subtitle: optional by parse_text_construct;
        "generator" => generator: optional by parse_generator;
        "logo" => logo: optional by parse_icon;
        "icon" => icon: optional by parse_icon;
    }

    let mut source = feed::Source::new(
        id.unwrap(),
        title.unwrap(),
        updated_at.unwrap());
    source.metadata.links = feed::LinkList(links);
    source.metadata.authors.push_all_move(authors.move_iter().filter_map(|v| v).collect());
    source.metadata.contributors.push_all_move(contributors.move_iter().filter_map(|v| v).collect());
    source.metadata.categories.push_all_move(categories);
    source.metadata.rights = rights;
    source.subtitle = subtitle;
    source.generator = generator;
    source.logo = logo;
    source.icon = icon;
    Ok(source)
}

fn reset_xml_base(attributes: &[Attribute], session: &mut AtomSession) {
    for new_base in get_xml_base(attributes.as_slice()).move_iter() {
        session.xml_base = new_base.into_string();
    }
}

fn read_whole_text<B: Buffer>(mut parser: NestedEventReader<B>) -> DecodeResult<String> {
    let mut text = String::new();
    for_each!(event in parser.next() {
        match event {
            Characters(s) => { text.push_str(s.as_slice()); }
            _ => { }
        }
    })
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

fn parse_icon<B: Buffer>(parser: NestedEventReader<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<String> {
    reset_xml_base(attributes, &mut session);
    Ok(session.xml_base.append(try!(read_whole_text(parser)).as_slice()))
}

fn parse_text_construct<B: Buffer>(parser: NestedEventReader<B>, attributes: &[Attribute], _session: AtomSession) -> DecodeResult<feed::Text> {
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

fn parse_person_construct<B: Buffer>(mut parser: NestedEventReader<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<Option<feed::Person>> {
    reset_xml_base(attributes, &mut session);
    let mut person_name = Default::default();
    let mut uri = Default::default();
    let mut email = Default::default();

    for_each!(event in parser.next() {
        match event {
            Element { name, children, .. } => {
                if name_matches(&name, Some(session.element_ns.as_slice()), "name") {
                    person_name = Some(try!(read_whole_text(children)));
                } else if name_matches(&name, Some(session.element_ns.as_slice()), "uri") {
                    uri = Some(try!(read_whole_text(children)));
                } else if name_matches(&name, Some(session.element_ns.as_slice()), "email") {
                    email = Some(try!(read_whole_text(children)));
                }
            }
            _ => { }
        }
    })
    let name = match person_name {
        Some(n) => n,
        None => match uri.clone().or_else(|| email.clone()) {
            Some(v) => { v }
            None => { return Ok(None); }
        }
    };
    Ok(Some(feed::Person { name: name, uri: uri, email: email }))
}

fn parse_link<B: Buffer>(_parser: NestedEventReader<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<feed::Link> {
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

fn parse_datetime<B: Buffer>(parser: NestedEventReader<B>, _attributes: &[Attribute], _session: AtomSession) -> DecodeResult<DateTime<FixedOffset>> {
    match codecs::RFC3339.decode(try!(read_whole_text(parser)).as_slice()) {
        Ok(v) => Ok(v),
        Err(e) => Err(SchemaError(e)),
    }
}

fn parse_category<B: Buffer>(_parser: NestedEventReader<B>, attributes: &[Attribute], _session: AtomSession) -> DecodeResult<feed::Category> {
    Ok(feed::Category {
        term: f!(attributes, "term"),
        scheme_uri: find_from_attr(attributes, "scheme").map(|v| v.to_string()),
        label: find_from_attr(attributes, "label").map(|v| v.to_string()),
    })
}

fn parse_generator<B: Buffer>(parser: NestedEventReader<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<feed::Generator> {
    reset_xml_base(attributes, &mut session);
    Ok(feed::Generator {
        uri: find_from_attr(attributes, "uri").map(|v| v.to_string()),  // TODO
        version: find_from_attr(attributes, "version").map(|v| v.to_string()),
        value: try!(read_whole_text(parser)),
    })
}

fn parse_content<B: Buffer>(parser: NestedEventReader<B>, attributes: &[Attribute], mut session: AtomSession) -> DecodeResult<feed::Content> {
    reset_xml_base(attributes, &mut session);
    Ok(feed::Content {
        text: try!(parse_text_construct(parser, attributes, session.clone())),
        source_uri: find_from_attr(attributes, "src").map(|v| v.to_string()),  // TODO
    })
}
