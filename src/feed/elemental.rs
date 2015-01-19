use std::fmt;

use chrono::{DateTime, FixedOffset};

use schema::{Mergeable};

pub enum TextType { Text, Html }

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

#[derive(Default)]
pub struct Mark {
    pub marked: bool,
    pub updated_at: Option<DateTime<FixedOffset>>,
}

impl fmt::Show for Mark {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "Mark {{ marked: {:?}, updated_at: {:?} }}", self.marked, self.updated_at)
    }
}

impl Mergeable for Mark {
    fn merge_entities(self, other: Mark) -> Mark {
        use std::cmp::Ordering::Less;
        match self.updated_at.cmp(&other.updated_at) {
            Less => other,
            _    => self,
        }
    }
}
