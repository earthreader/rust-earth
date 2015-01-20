use std::borrow::ToOwned;
use std::default::Default;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, FixedOffset, UTC};

use schema::{Mergeable};

pub use self::elemental::{TextType, Text, Person, Link, LinkList, Category, Content, Generator, Mark};

pub mod elemental;


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

impl Mergeable for Entry {
    fn merge_entities(mut self, other: Entry) -> Entry {
        self.read = self.read.merge_entities(other.read);
        self.starred = self.starred.merge_entities(other.starred);
        self
    }
}
