use std::io;

use chrono::{DateTime, FixedOffset};

use parser::base::{DecodeResult, XmlElement, XmlName};
use schema::{DocumentElement, FromSchemaReader, Mergeable};
use util::{default_datetime, set_default};

use super::{ATOM_XMLNS, Category, Entry, Generator, Link, Person, Text};
use super::metadata::match_metadata_child;


/// Atom feed document, acting as a container for metadata and data associated
/// with the feed.
///
/// It corresponds to ``atom:feed`` element of :rfc:`4287#section-4.1.1`
/// (section 4.1.1).
pub struct Feed {
    /// The URI that conveys a permanent, universally unique identifier for an
    /// entry or feed.  It corresponds to `atom:id` element of :rfc:`4287#section-4.2.6` (section 4.2.6).
    pub id: String,

    /// The human-readable title for an entry or feed.
    /// It corresponds to `atom:title` element of :rfc:`4287#section-4.2.14` (section 4.2.14).
    pub title: Text,

    /// The list of :class:`Link` objects that define a reference from an entry
    /// or feed to a web resource.  It corresponds to `atom:link` element of
    /// :rfc:`4287#section-4.2.7` (section 4.2.7).
    pub links: Vec<Link>,

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

    /// The list of `Entry` values that represent an individual entry, acting
    /// as a container for metadata and data associated with the entry.
    /// It corresponds to ``atom:entry`` element of :rfc:`4287#section-4.1.2`
    /// (section 4.1.2).
    pub entries: Vec<Entry>,
}

impl_metadata!(Feed);

impl Feed {
    pub fn new<I, T, U>(id: I, title: T, updated_at: U) -> Feed
        where I: Into<String>, T: Into<Text>, U: Into<DateTime<FixedOffset>>
    {
        Feed {
            id: id.into(),
            title: title.into(),
            links: Default::default(),
            updated_at: updated_at.into(),
            authors: Default::default(),
            contributors: Default::default(),
            categories: Default::default(),
            rights: Default::default(),
            subtitle: Default::default(),
            generator: Default::default(),
            logo: Default::default(),
            icon: Default::default(),
            entries: Default::default(),
        }
    }
}

impl Default for Feed {
    fn default() -> Feed {
        Feed::new("", Text::default(), default_datetime())
    }
}

impl DocumentElement for Feed {
    fn tag() -> &'static str { "feed" }
    fn xmlns() -> Option<&'static str> { Some(ATOM_XMLNS) }
}

impl FromSchemaReader for Feed {
    fn match_child<B: io::BufRead>(&mut self, name: &XmlName,
                              child: XmlElement<B>) -> DecodeResult<()> {
        match(name.namespace_ref(), &name.local_name[..]) {
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
            (Some(ATOM_XMLNS), "entry") => {
                let mut entry: Entry = Default::default();
                try!(entry.read_from(child));
                self.entries.push(entry);
            }
            _ => { return match_metadata_child(self, name, child); }
        }
        Ok(())
    }
}

impl_mergeable!(Feed, id, title, links, updated_at, authors, contributors,
                categories, rights, subtitle, generator, logo, icon, entries);


#[cfg(test)]
mod test {
    use super::Feed;

    use std::default::Default;
    use std::io;

    use chrono::{TimeZone, UTC};
    use xml;

    use feed::{Link, Person, Text};
    use parser::base::NestedEventReader;
    use parser::base::NestedEvent::Nested;
    use schema::FromSchemaReader;

    fn fx_feed() -> Feed {
        read_feed(r##"
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:mark="http://earthreader.org/mark/">
            <title>Example Feed</title>
            <link href="http://example.org/"/>
            <updated>2003-12-13T18:30:02Z</updated>
            <author><name>John Doe</name></author>
            <author><name>Jane Doe</name></author>
            <id>urn:uuid:60a76c80-d399-11d9-b93C-0003939e0af6</id>
            <category term="technology"/>
            <category term="business"/>
            <rights>Public Domain</rights>
            <entry>
                <title>Atom-Powered Robots Run Amok</title>
                <link href="http://example.org/2003/12/13/atom03"/>
                <id>urn:uuid:1225c695-cfb8-4ebb-aaaa-80da344efa6a</id>
                <updated>2003-12-13T18:30:02Z</updated>
                <summary>Some text.</summary>
                <author><name>Jane Doe</name></author>
                <mark:read updated="2013-11-06T14:36:00Z">true</mark:read>
            </entry>
            <entry>
                <title>Danger, Will Robinson!</title>
                <link href="http://example.org/2003/12/13/lost"/>
                <id>urn:uuid:b12f2c10-ffc1-11d9-8cd6-0800200c9a66</id>
                <updated>2003-12-13T18:30:02Z</updated>
                <summary>Don't Panic!</summary>
            </entry>
        </feed>
        "## // "
        .as_bytes())
    }

    fn read_feed<B: io::BufRead>(buf: B) -> Feed {
        let mut parser = xml::EventReader::new(buf);
        let mut events = NestedEventReader::new(&mut parser);
        let mut feed: Feed = Default::default();
        while let Some(event) = events.next() {
            match event.unwrap() {
                Nested { name: _, element } =>
                    FromSchemaReader::read_from(&mut feed, element).unwrap(),
                _ => { }
            }
        }
        feed
    }

    #[test]
    fn test_feed_read() {
        let feed = fx_feed();
        assert_eq!(feed.title, Text::plain("Example Feed"));
        assert_eq!(feed.links.len(), 1);
        let ref link = feed.links[0];
        assert_eq!(link.relation, "alternate");
        assert_eq!(link.uri, "http://example.org/");
        assert_eq!(feed.updated_at, UTC.ymd(2003, 12, 13).and_hms(18, 30, 2));
        let ref authors = feed.authors;
        assert_eq!(feed.authors.len(), 2);
        assert_eq!(authors[0].name, "John Doe");
        assert_eq!(authors[1].name, "Jane Doe");
        assert_eq!(feed.id, "urn:uuid:60a76c80-d399-11d9-b93C-0003939e0af6");
        let ref categories = feed.categories;
        assert_eq!(categories.len(), 2);
        assert_eq!(categories[0].term, "technology");
        assert_eq!(categories[1].term, "business");
        assert_eq!(feed.rights, Some(Text::plain("Public Domain")));
        let ref entries = feed.entries;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title,
                   Text::plain("Atom-Powered Robots Run Amok"));
        assert_eq!(&entries[0].links[..],
                   [Link::new("http://example.org/2003/12/13/atom03")]);
        assert_eq!(entries[0].id,
                   "urn:uuid:1225c695-cfb8-4ebb-aaaa-80da344efa6a");
        assert_eq!(entries[0].updated_at,
                   UTC.ymd(2003, 12, 13).and_hms(18, 30, 2));
        assert_eq!(entries[0].summary, Some(Text::plain("Some text.")));
        assert_eq!(&entries[0].authors[..], [Person::new("Jane Doe")]);
        assert_eq!(entries[1].title, Text::plain("Danger, Will Robinson!"));
        assert_eq!(&entries[1].links[..],
                   [Link::new("http://example.org/2003/12/13/lost")]);
        assert_eq!(entries[1].id,
                   "urn:uuid:b12f2c10-ffc1-11d9-8cd6-0800200c9a66");
        assert_eq!(entries[1].updated_at,
                   UTC.ymd(2003, 12, 13).and_hms(18, 30, 2));
        assert_eq!(entries[1].summary, Some(Text::plain("Don't Panic!")));
    }
}
