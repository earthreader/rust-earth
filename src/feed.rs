use std::default::Default;
use std::fmt;

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
