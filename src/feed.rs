use std::default::Default;

use chrono::{DateTime, FixedOffset};

use schema::{Mergeable};

pub use self::elemental::text_type;
pub use self::elemental::{TextType, Text, Person, Link, LinkList, Category, Content, Generator, Mark};


pub struct Feed {
    pub source: Source,

    pub entries: Vec<Entry>,
}

impl Feed {
    pub fn new_inherited(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Feed {
        Feed {
            source: Source::new_inherited(id, title, updated_at),
            entries: Default::default(),
        }
    }

    pub fn new(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Feed {
        Feed::new(id, title, updated_at)
    }
}

pub trait FeedMethods {
    fn get_feed<'a>(&'a self) -> &'a Feed;
    fn mut_feed<'a>(&'a mut self) -> &'a mut Feed;

    fn get_entries<'a>(&'a self) -> &'a [Entry] { self.get_feed().entries.as_slice() }
    fn mut_entries<'a>(&'a mut self) -> &'a mut Vec<Entry> { &mut self.mut_feed().entries }
}

impl FeedMethods for Feed {
    fn get_feed<'a>(&'a self) -> &'a Feed { self }
    fn mut_feed<'a>(&'a mut self) -> &'a mut Feed { self }
}

pub struct Entry {
    pub metadata: Metadata,

    pub published_at: Option<DateTime<FixedOffset>>,
    pub summary: Option<Text>,
    pub content: Option<Content>,
    pub source: Option<Source>,
    pub read: Mark,
    pub starred: Mark,
}

impl Entry {
    pub fn new_inherited(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Entry {
        Entry {
            metadata: Metadata::new_inherited(id, title, updated_at),
            published_at: Default::default(),
            summary: Default::default(),
            content: Default::default(),
            source: Default::default(),
            read: Default::default(),
            starred: Default::default(),
        }
    }

    pub fn new(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Entry {
        Entry::new(id, title, updated_at)
    }
}

pub trait EntryMethods {
    fn get_entry<'a>(&'a self) -> &'a Entry;

    fn get_published_at<'a>(&'a self) -> Option<&'a DateTime<FixedOffset>> { self.get_entry().published_at.as_ref() }
    fn get_summary<'a>(&'a self) -> Option<&'a Text> { self.get_entry().summary.as_ref() }
    fn get_content<'a>(&'a self) -> Option<&'a Content> { self.get_entry().content.as_ref() }
    fn get_source<'a>(&'a self) -> Option<&'a Source> { self.get_entry().source.as_ref() }
    fn get_read<'a>(&'a self) -> &'a Mark { &self.get_entry().read }
    fn get_starred<'a>(&'a self) -> &'a Mark { &self.get_entry().starred }
}

impl EntryMethods for Entry {
    fn get_entry<'a>(&'a self) -> &'a Entry { self }
}

pub struct Source {
    pub metadata: Metadata,

    pub subtitle: Option<Text>,
    pub generator: Option<Generator>,
    pub logo: Option<String>,
    pub icon: Option<String>,
}

impl Source {
    pub fn new_inherited(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Source {
        Source {
            metadata: Metadata::new_inherited(id, title, updated_at),
            subtitle: Default::default(),
            generator: Default::default(),
            logo: Default::default(),
            icon: Default::default(),
        }
    }

    pub fn new(id: String, title: Text, updated_at: DateTime<FixedOffset>) -> Source {
        Source::new(id, title, updated_at)
    }
}

pub trait SourceMethods {
    fn get_source<'a>(&'a self) -> &'a Source;

    fn get_subtitle<'a>(&'a self) -> Option<&'a Text> { self.get_source().subtitle.as_ref() }
    fn get_generator<'a>(&'a self) -> Option<&'a Generator> { self.get_source().generator.as_ref() }
    fn get_logo<'a>(&'a self) -> Option<&'a str> { self.get_source().logo.as_ref().map(|v| v.as_slice()) }
    fn get_icon<'a>(&'a self) -> Option<&'a str> { self.get_source().icon.as_ref().map(|v| v.as_slice()) }
}

impl SourceMethods for Source {
    fn get_source<'a>(&'a self) -> &'a Source { self }
}

impl SourceMethods for Feed {
    fn get_source<'a>(&'a self) -> &'a Source { &self.source }
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
            links: LinkList(Default::default()),
            updated_at: updated_at,
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

pub trait MetadataMethods {
    fn get_metadata<'a>(&'a self) -> &'a Metadata;

    fn get_id<'a>(&'a self) -> &'a str { self.get_metadata().id.as_slice() }
    fn get_title<'a>(&'a self) -> &'a Text { &self.get_metadata().title }
    fn get_links<'a>(&'a self) -> &'a LinkList { &self.get_metadata().links }
    fn get_updated_at<'a>(&'a self) -> &'a DateTime<FixedOffset> { &self.get_metadata().updated_at }
    fn get_authors<'a>(&'a self) -> &'a [Person] { self.get_metadata().authors.as_slice() }
    fn get_contributors<'a>(&'a self) -> &'a [Person] { self.get_metadata().contributors.as_slice() }
    fn get_categories<'a>(&'a self) -> &'a [Category] { self.get_metadata().categories.as_slice() }
    fn get_rights<'a>(&'a self) -> Option<&'a Text> { self.get_metadata().rights.as_ref() }
}

impl MetadataMethods for Metadata {
    fn get_metadata<'a>(&'a self) -> &'a Metadata { self }
}

impl MetadataMethods for Source {
    fn get_metadata<'a>(&'a self) -> &'a Metadata { &self.metadata }
}

impl MetadataMethods for Entry {
    fn get_metadata<'a>(&'a self) -> &'a Metadata { &self.metadata }
}

impl MetadataMethods for Feed {
    fn get_metadata<'a>(&'a self) -> &'a Metadata { &self.get_source().metadata }
}

// ----------------------------------------------------------------------------

mod elemental {
    use std::fmt;

    use chrono::{DateTime, FixedOffset};

    use schema::{Mergeable};

    pub use self::text_type::TextType;

    pub mod text_type {
        pub enum TextType { Text, Html }
    }

    pub struct Text {
        pub type_: TextType,
        pub value: String,
    }

    pub struct Person {
        pub name: String,
        pub uri: Option<String>,
        pub email: Option<String>,
    }

    pub struct Link {
        pub uri: String,
        pub relation: String,
        pub mimetype: Option<String>,
        pub language: Option<String>,
        pub title: Option<String>,
        pub byte_size: Option<u64>,
    }

    pub struct LinkList(pub Vec<Link>);

    pub struct Category {
        pub term: String,
        pub scheme_uri: Option<String>,
        pub label: Option<String>,
    }

    pub struct Content {
        pub text: Text,

        pub source_uri: Option<String>,
    }

    pub struct Generator {
        pub uri: Option<String>,
        pub version: Option<String>,
        pub value: String,
    }

    #[deriving(Default)]
    pub struct Mark {
        pub marked: bool,
        pub updated_at: Option<DateTime<FixedOffset>>,
    }

    impl fmt::Show for Mark {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::FormatError> {
            write!(fmt, "Mark {{ marked: {}, updated_at: {} }}", self.marked, self.updated_at)
        }
    }

    impl Mergeable for Mark {
        fn merge_entities(self, other: Mark) -> Mark {
            match self.updated_at.cmp(&other.updated_at) {
                Less => other,
                _    => self,
            }
        }
    }

}
