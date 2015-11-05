use std::default::Default;
use std::io;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, FixedOffset};

use parser::base::{DecodeResult, XmlElement, XmlName};
use schema::{DocumentElement, FromSchemaReader, Mergeable};

use super::{ATOM_XMLNS, Entry, Source, Text};


/// Atom feed document, acting as a container for metadata and data associated
/// with the feed.
///
/// It corresponds to ``atom:feed`` element of :rfc:`4287#section-4.1.1`
/// (section 4.1.1).
#[derive(Default)]
pub struct Feed {
    pub source: Source,

    /// The list of `Entry` values that represent an individual entry, acting
    /// as a container for metadata and data associated with the entry.
    /// It corresponds to ``atom:entry`` element of :rfc:`4287#section-4.1.2`
    /// (section 4.1.2).
    pub entries: Vec<Entry>,
}

impl Deref for Feed {
    type Target = Source;
    fn deref(&self) -> &Source { &self.source }
}

impl DerefMut for Feed {
    fn deref_mut(&mut self) -> &mut Source { &mut self.source }
}

impl Feed {
    pub fn new_inherited(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Feed {
        Feed {
            source: Source::new_inherited(id, title, updated_at),
            entries: Default::default(),
        }
    }

    pub fn new<T>(id: T, title: Text,
                  updated_at: DateTime<FixedOffset>) -> Feed
        where T: Into<String>
    {
        Feed::new_inherited(id.into(), title, updated_at)
    }
}

impl DocumentElement for Feed {
    fn tag() -> &'static str { "feed" }
    fn xmlns() -> Option<&'static str> { Some(ATOM_XMLNS) }
}

impl FromSchemaReader for Feed {
    fn match_child<B: io::BufRead>(&mut self, name: &XmlName,
                              child: XmlElement<B>) -> DecodeResult<()> {
        match &name.local_name[..] {
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

impl_mergeable!(Feed, source, entries);


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
        for_each!(event in events.next() {
            match event.unwrap() {
                Nested { name: _, element } =>
                    FromSchemaReader::read_from(&mut feed, element).unwrap(),
                _ => { }
            }
        });
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
