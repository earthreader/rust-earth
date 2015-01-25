#![unstable]

use std::borrow::ToOwned;
use std::default::Default;
use std::fmt;
use std::mem::swap;
use std::ops::Deref;

use html::ForHtml;
use parser::base::{DecodeResult, DecodeError, XmlElement, XmlName};
use parser::base::NestedEvent::Nested;
use sanitizer::escape;
use schema::{FromSchemaReader, Mergeable};
use util::{merge_vec, set_default};

/// Person construct defined in RFC 4287 (section 3.2).
///
/// RFC: <https://tools.ietf.org/html/rfc4287#section-3.2>
#[unstable]
#[derive(PartialEq, Eq, Hash, Show)]
pub struct Person {
    /// The human-readable name for the person.  It corresponds to
    /// `atom:name` element of [RFC 4287 (section 3.2.1)][rfc-person-1].
    ///
    /// [rfc-person-1]: https://tools.ietf.org/html/rfc4287#section-3.2.1
    pub name: String,

    /// The optional URI associated with the person.  It corresponds to
    /// `atom:uri` element of [RFC 4287 (section 3.2.2)][rfc-person-2].
    ///
    /// [rfc-person-2]: https://tools.ietf.org/html/rfc4287#section-3.2.2
    pub uri: Option<String>,

    /// The optional email address associated with the person.  It
    /// corresponds to ``atom:email`` element of [RFC 4287 (section 3.2.3)
    /// ][rfc-person-3].
    ///
    /// [rfc-person-3]: https://tools.ietf.org/html/rfc4287#section-3.2.3
    pub email: Option<String>,
}

impl Person {
    pub fn new<T, S: ?Sized>(name: T) -> Person
        where T: Deref<Target=S>, S: ToOwned<String>
    {
        Person { name: name.to_owned(), uri: None, email: None }
    }
}

impl Default for Person {
    fn default() -> Person { Person::new("") }
}

impl fmt::String for Person {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "{}", self.name));
        if let Some(ref r) = self.uri.as_ref().or(self.email.as_ref()) {
            try!(write!(f, " <{}>", r));
        }
        Ok(())
    }
}

impl<'a> fmt::String for ForHtml<'a, Person> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = escape(&self.name[], true);
        let hyperlink = match (self.uri.as_ref(), self.email.as_ref()) {
            (Some(uri), _) => {
                try!(write!(f, "<a href=\"{}\">",
                            escape(&uri[], true)));
                true
            }
            (None, Some(email)) => {
                try!(write!(f, "<a href=\"mailto:{}\">",
                            escape(&email[], true)));
                true
            }
            (None, None) => { false }
        };
        try!(write!(f, "{}", name));
        if hyperlink {
            try!(write!(f, "</a>"));
        }
        Ok(())
    }
}

impl FromSchemaReader for Option<Person> {
    fn read_from<B: Buffer>(&mut self, mut element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        *self = None;
        loop {
            match element.children.next() {
                Some(Nested { name, element: child }) => {
                    try!(self.match_child(&name, child))
                }
                None => { break; }
                Some(_) => { }     
            }
        }
        if self.as_ref().map_or(true, |p| p.name.is_empty()) {
            *self = None;
        }
        Ok(())
    }

    fn match_child<B: Buffer>(&mut self, name: &XmlName,
                              element: XmlElement<B>)
                              -> DecodeResult<()>
    {
        match &name.local_name[] {
            "name" => {
                let name = try!(element.read_whole_text());
                set_default(self).name = name;
            }
            "uri" => {
                let uri = Some(try!(element.read_whole_text()));
                set_default(self).uri = uri;
            }
            "email" => {
                let email = Some(try!(element.read_whole_text()));
                set_default(self).email = email;
            }
            _ => { return Err(DecodeError::NoResult); }
        }
        Ok(())
    }
}

impl Mergeable for Vec<Person> {
    fn merge_with(&mut self, mut other: Vec<Person>) {
        swap(self, &mut other);
        merge_vec(self, other.into_iter());
    }
}


#[cfg(test)]
mod test {
    use super::{Person};

    use html::ToHtml;

    #[test]
    fn test_person_str() {
        assert_eq!(Person { name: "Hong Minhee".to_string(),
                            uri: None, email: None }.to_string(),
                   "Hong Minhee");
        assert_eq!(Person { name: "Hong Minhee".to_string(),
                            uri: Some("http://dahlia.kr/".to_string()),
                            email: None }.to_string(),
                   "Hong Minhee <http://dahlia.kr/>");
        let email = concat!("\x6d\x69\x6e\x68\x65\x65\x40\x64",
                            "\x61\x68\x6c\x69\x61\x2e\x6b\x72");
        assert_eq!(Person { name: "Hong Minhee".to_string(),
                            uri: None,
                            email: Some(email.to_string()) }.to_string(),
                   format!("Hong Minhee <{}>", email));
        assert_eq!("홍민희 <http://dahlia.kr/>",
                   Person {
                       name: "홍민희".to_string(),
                       uri: Some("http://dahlia.kr/".to_string()),
                       email: Some(email.to_string()),
                   }.to_string());
    }

    #[test]
    fn test_person_html() {
        assert_html!(Person::new("Hong \"Test\" Minhee"),
                     "Hong &quot;Test&quot; Minhee");
        assert_html!(Person { name: "Hong Minhee".to_string(),
                              uri: Some("http://dahlia.kr/".to_string()),
                              email: None },
                     "<a href=\"http://dahlia.kr/\">Hong Minhee</a>");
        let email = concat!("\x6d\x69\x6e\x68\x65\x65\x40\x64",
                            "\x61\x68\x6c\x69\x61\x2e\x6b\x72");
        assert_html!(Person { name: "Hong Minhee".to_string(),
                              uri: None,
                              email: Some(email.to_string()) },
                     format!("<a href=\"mailto:{}\">Hong Minhee</a>",
                             email));
        assert_html!(Person { name: "홍민희".to_string(),
                              uri: Some("http://dahlia.kr/".to_string()),
                              email: Some(email.to_string()) },
                     "<a href=\"http://dahlia.kr/\">홍민희</a>");
    }
}
