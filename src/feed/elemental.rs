use std::borrow::ToOwned;
use std::default::Default;
use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};
use std::fmt;

use chrono::{DateTime, FixedOffset};

use schema::{Mergeable};

#[derive(Copy, PartialEq, Eq, Show)]
pub enum TextType { Text, Html }

impl Default for TextType {
    fn default() -> TextType { TextType::Text }
}

#[derive(Default, PartialEq, Show)]
pub struct Text {
    pub type_: TextType,
    pub value: String,
}

impl Text {
    pub fn new<T, S: ?Sized>(type_: TextType, value: T) -> Text
        where T: Deref<Target=S>, S: ToOwned<String>
    {
        Text { type_: type_, value: value.to_owned() }
    }

    pub fn text<T, S: ?Sized>(value: T) -> Text
        where T: Deref<Target=S>, S: ToOwned<String>
    {
        Text::new(TextType::Text, value.to_owned())
    }

    pub fn html<T, S: ?Sized>(value: T) -> Text
        where T: Deref<Target=S>, S: ToOwned<String>
    {
        Text::new(TextType::Html, value.to_owned())
    }
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

#[derive(Default)]
pub struct LinkList(pub Vec<Link>);

impl Deref for LinkList {
    type Target = Vec<Link>;
    fn deref(&self) -> &Vec<Link> { &self.0 }
}

impl DerefMut for LinkList {
    fn deref_mut(&mut self) -> &mut Vec<Link> { &mut self.0 }
}

impl FromIterator<Link> for LinkList {
    fn from_iter<T: Iterator<Item=Link>>(iterator: T) -> Self {
        LinkList(FromIterator::from_iter(iterator))
    }
}

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

impl fmt::String for Person {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(fmt, "{}", self.name));
        if let Some(ref uri) = self.uri {
            try!(write!(fmt, " <{}>", uri));
        } else if let Some(ref email) = self.email {
            try!(write!(fmt, " <{}>", email));
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::{Person};

    macro_rules! w {
        ($expr:expr) => { format!("{}", $expr) }
    }

    #[test]
    fn test_person_str() {
        assert_eq!(w!(Person { name: "Hong Minhee".to_string(),
                               uri: None, email: None }),
                   "Hong Minhee");
        assert_eq!(w!(Person { name: "Hong Minhee".to_string(),
                               uri: Some("http://dahlia.kr/".to_string()),
                               email: None }),
                   "Hong Minhee <http://dahlia.kr/>");
        let email = concat!("\x6d\x69\x6e\x68\x65\x65\x40\x64",
                            "\x61\x68\x6c\x69\x61\x2e\x6b\x72");
        assert_eq!(w!(Person { name: "Hong Minhee".to_string(),
                               uri: None,
                               email: Some(email.to_string()) }),
                   format!("Hong Minhee <{}>", email));
        assert_eq!("홍민희 <http://dahlia.kr/>", w!(
            Person {
                name: "홍민희".to_string(),
                uri: Some("http://dahlia.kr/".to_string()),
                email: Some(email.to_string()),
            }));
    }
}
