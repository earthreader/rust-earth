use std::borrow::ToOwned;
use std::default::Default;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, FixedOffset};
use xml;

use parser::base::NestedEventReader;
use parser::base::NestedEvent::Nested;
use schema::FromSchemaReader;

pub use self::elemental::{TextType, Text, Person, Link, LinkList, Category, Content, Generator, Mark};
pub use self::elemental::link::LinkIteratorExt;

pub mod elemental;
pub mod schema;


#[derive(Default)]
pub struct Feed {
    pub source: Source,

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

    pub fn new<T, S: ?Sized>(id: T, title: Text,
                             updated_at: DateTime<FixedOffset>) -> Feed
        where T: Deref<Target=S>, S: ToOwned<String>
    {
        Feed::new_inherited(id.to_owned(), title, updated_at)
    }
}

pub fn read_feed<B: Buffer>(buf: B) -> Feed {
    let mut parser = xml::EventReader::new(buf);
    let mut events = NestedEventReader::new(&mut parser);
    let mut feed: Feed = Default::default();
    for_each!(event in events.next() {
        match event {
            Nested { name: _, element } =>
                FromSchemaReader::read_from(&mut feed, element).unwrap(),
            _ => { }
        }
    });
    feed
}

#[derive(Default)]
pub struct Entry {
    pub metadata: Metadata,

    pub published_at: Option<DateTime<FixedOffset>>,
    pub summary: Option<Text>,
    pub content: Option<Content>,
    pub source: Option<Source>,
    pub read: Mark,
    pub starred: Mark,
}

impl Deref for Entry {
    type Target = Metadata;
    fn deref(&self) -> &Metadata { &self.metadata }
}

impl DerefMut for Entry {
    fn deref_mut(&mut self) -> &mut Metadata { &mut self.metadata }
}

impl Entry {
    pub fn new_inherited(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Entry {
        Entry {
            metadata: Metadata::new_inherited(id, title, updated_at),
            ..Default::default()
        }
    }

    pub fn new(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Entry {
        Entry::new(id, title, updated_at)
    }
}

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

pub struct Metadata {
    pub id: String,
    pub title: Text,
    pub links: LinkList,
    pub updated_at: DateTime<FixedOffset>,
    pub authors: Vec<Person>,
    pub contributors: Vec<Person>,
    pub categories: Vec<Category>,
    pub rights: Option<Text>,
}

impl Metadata {
    pub fn new_inherited(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Metadata {
        Metadata {
            id: id,
            title: title,
            updated_at: updated_at,
            ..Default::default()
        }            
    }
}

impl Default for Metadata {
    fn default() -> Metadata {
        use chrono::{NaiveDateTime, Offset};
        let default_datetime = FixedOffset::east(0).from_local_datetime(
            &NaiveDateTime::from_num_seconds_from_unix_epoch(0, 0)
        ).earliest().unwrap();
        Metadata {
            id: Default::default(),
            title: Default::default(),
            links: Default::default(),
            updated_at: default_datetime,
            authors: Default::default(),
            contributors: Default::default(),
            categories: Default::default(),
            rights: Default::default(),
        }
    }
}

pub fn get_mut_or_set<T, F>(opt: &mut Option<T>, f: F) -> &mut T
    where F: Fn() -> T
{
    if let Some(v) = opt.as_mut() {
        return v;
    }
    unsafe {
        let opt: *mut Option<T> = opt;
        let opt: &mut Option<T> = opt.as_mut().unwrap();
        *opt = Some(f());
        opt.as_mut().unwrap()
    }
}

pub fn set_default<T: Default>(opt: &mut Option<T>) -> &mut T {
    get_mut_or_set(opt, Default::default)
}


#[cfg(test)]
mod test {
    use super::{Feed, read_feed};
    use super::elemental::{Link, Person, Text};

    use chrono::{Offset, UTC};
    
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

    #[test]
    fn test_feed_read() {
        let feed = fx_feed();
        assert_eq!(feed.title, Text::text("Example Feed"));
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
        assert_eq!(feed.rights, Some(Text::text("Public Domain")));
        let ref entries = feed.entries;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title,
                   Text::text("Atom-Powered Robots Run Amok"));
        assert_eq!(&entries[0].links[],
                   [Link::new("http://example.org/2003/12/13/atom03")]);
        assert_eq!(entries[0].id,
                   "urn:uuid:1225c695-cfb8-4ebb-aaaa-80da344efa6a");
        assert_eq!(entries[0].updated_at,
                   UTC.ymd(2003, 12, 13).and_hms(18, 30, 2));
        assert_eq!(entries[0].summary, Some(Text::text("Some text.")));
        assert_eq!(&entries[0].authors[], [Person::new("Jane Doe")]);
        assert_eq!(entries[1].title, Text::text("Danger, Will Robinson!"));
        assert_eq!(&entries[1].links[],
                   [Link::new("http://example.org/2003/12/13/lost")]);
        assert_eq!(entries[1].id,
                   "urn:uuid:b12f2c10-ffc1-11d9-8cd6-0800200c9a66");
        assert_eq!(entries[1].updated_at,
                   UTC.ymd(2003, 12, 13).and_hms(18, 30, 2));
        assert_eq!(entries[1].summary, Some(Text::text("Don't Panic!")));
    }
}
