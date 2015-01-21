use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};

pub use self::text::{Text, TextType};
pub use self::person::Person;
pub use self::link::Link;
pub use self::mark::Mark;


#[unstable]
pub mod text {
    use std::borrow::ToOwned;
    use std::default::Default;
    use std::ops::Deref;
    use std::fmt;

    use html::{Html};
    use sanitizer::{clean_html, escape, sanitize_html};

    /// The type of the text. It corresponds to :rfc:`4287#section-3.1.1` (section 3.1.1).
    ///
    /// Note: It currently does not support `xhtml`.
    #[unstable]
    #[derive(Copy, PartialEq, Eq, Show)]
    pub enum TextType { Text, Html }

    impl Default for TextType {
        fn default() -> TextType { TextType::Text }
    }

    /// Text construct defined in :rfc:`4287#section-3.1` (section 3.1).
    ///
    /// RFC: <https://tools.ietf.org/html/rfc4287#section-3.1>
    #[unstable]
    #[derive(Default, PartialEq, Eq, Show)]
    pub struct Text {
        pub type_: TextType,

        /// The content of the text.  Interpretation for this has to differ
        /// according to its `type_`.  It corresponds to :rfc:`4287#section-3.1.1.1` (section 3.1.1.1) if `type_` is `TextType::Text`, and :rfc:`4287#section-3.1.1.2` (section 3.1.1.2) if `type_` is `TextType::Html`.
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

        /// Get the secure HTML string of the text.  If it's a plain text, this
        /// returns entity-escaped HTML string, and if it's a HTML text,
        /// `value` is sanitized.
        ///
        /// ```ignore
        /// # use earth::feed::Text;
        /// let text = Text::text("<Hello>");
        /// let html = Text::html("<script>alert(1);</script><p>Hello</p>");
        /// assert_eq!(format!("{}", text.sanitized_html(None)), "&lt;Hello&gt;");
        /// assert_eq!(format!("{}", html.sanitized_html(None)), "<p>Hello</p>");
        /// ```
        #[unstable = "incomplete"]
        pub fn sanitized_html<'a>(&'a self, base_uri: Option<&'a str>) ->
            Box<fmt::String + 'a>
        {
            let value: &'a _ = &self.value[];
            match self.type_ {
                TextType::Text =>
                    Box::new(escape(value, true)) as Box<fmt::String>,
                TextType::Html =>
                    Box::new(sanitize_html(value, base_uri)) as Box<fmt::String>,
            }
        }
    }

    impl fmt::String for Text {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self.type_ {
                TextType::Text => write!(f, "{}", self.value),
                TextType::Html => write!(f, "{}", clean_html(&self.value[])),
            }
        }
    }

    impl Html for Text {
        fn fmt_html(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.sanitized_html(None))
        }
    }
}


#[unstable]
pub mod person {
    use std::borrow::ToOwned;
    use std::default::Default;
    use std::fmt;
    use std::ops::Deref;

    use html::{Html};
    use sanitizer::escape;

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

    impl Html for Person {
        fn fmt_html(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let name = escape(&self.name[], true);
            if let Some(ref r) = self.uri.as_ref().or(self.email.as_ref()) {
                let scheme = if self.email.is_some() { "mailto" } else { "" };
                try!(write!(f, "<a href=\"{2}{1}\">{0}</a>",
                            name, escape(&r[], true), scheme));
            } else {
                try!(write!(f, "{}", name));
            }
            Ok(())
        }
    }
}


#[unstable]
pub mod link {
    use std::borrow::ToOwned;
    use std::default::Default;
    use std::fmt;
    use std::iter::Filter;
    use std::ops::Deref;

    use regex::Regex;

    use html::Html;

    /// Link element defined in RFC 4287 (section 4.2.7).
    ///
    /// RFC: <https://tools.ietf.org/html/rfc4287#section-4.2.7>.
    #[unstable]
    #[derive(PartialEq, Eq, Hash, Show)]
    pub struct Link {
        /// The link's required URI.  It corresponds to `href` attribute of
        /// [RFC 4287 (section 4.2.7.1)][rfc-link-1].
        ///
        /// [rfc-link-1]: https://tools.ietf.org/html/rfc4287#section-4.2.7.1
        pub uri: String,

        /// The relation type of the link.  It corresponds to `rel` attribute
        /// of [RFC 4287 (section 4.2.7.2)][rfc-link-2].
        ///
        /// ### See also
        ///
        /// * [Existing rel values][rel-values] --- Microformats Wiki
        ///
        ///   This page contains tables of known HTML ``rel`` values from
        ///   specifications, formats, proposals, brainstorms, and non-trivial
        ///   [POSH][] usage in the wild.  In addition, dropped and rejected
        ///   values are listed at the end for comprehensiveness.
        ///
        /// [rfc-link-2]: https://tools.ietf.org/html/rfc4287#section-4.2.7.2
        /// [rel-values]: http://microformats.org/wiki/existing-rel-values
        /// [POSH]: http://microformats.org/wiki/POSH
        pub relation: String,

        /// The optional hint for the MIME media type of the linked content.
        /// It corresponds to `type` attribute of
        /// [RFC 4287 (section 4.2.7.3)][rfc-link-3].
        ///
        /// [rfc-link-3]: https://tools.ietf.org/html/rfc4287#section-4.2.7.3
        pub mimetype: Option<String>,

        /// The language of the linked content.  It corresponds to `hreflang`
        /// attribute of [RFC 4287 (section 4.2.7.4)][rfc-link-4].
        ///
        /// [rfc-link-4]: https://tools.ietf.org/html/rfc4287#section-4.2.7.4
        pub language: Option<String>,

        /// The title of the linked resource.  It corresponds to `title`
        /// attribute of [RFC 4287 (section 4.2.7.5)][rfc-link-5].
        ///
        /// [rfc-link-5]: https://tools.ietf.org/html/rfc4287#section-4.2.7.5
        pub title: Option<String>,

        /// The optional hint for the length of the linked content in octets.
        /// It corresponds to `length` attribute of [RFC 4287 (section 4.2.7.6)
        /// ][rfc-link-6].
        ///
        /// [rfc-link-6]: https://tools.ietf.org/html/rfc4287#section-4.2.7.6
        pub byte_size: Option<u64>,
    }

    impl Link {
        #[unstable]
        pub fn new<T, S: ?Sized>(uri: T) -> Link
            where T: Deref<Target=S>, S: ToOwned<String>
        {
            Link {
                uri: uri.to_owned(), relation: "alternate".to_owned(),
                mimetype: None, language: None, title: None, byte_size: None
            }   
        }

        /// Whether its `mimetype` is HTML (or XHTML).
        #[unstable]
        pub fn is_html(&self) -> bool {
            if let Some(ref mimetype) = self.mimetype {
                let pat = regex!(r#"^\s*([^;/\s]+/[^;/\s]+)\s*(?:;\s*.*)?$"#);
                if let Some(c) = pat.captures(&mimetype[]) {
                    if let Some(mimetype) = c.at(1) {
                        return ["text/html", "application/xhtml+xml"]
                            .contains(&mimetype);
                    }
                }
            }
            false
        }
    }

    impl Default for Link {
        fn default() -> Link { Link::new("") }
    }

    impl fmt::String for Link {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.uri)
        }
    }

    impl Html for Link {
        fn fmt_html(&self, f: &mut fmt::Formatter) -> fmt::Result {
            try!(write!(f, "<link rel=\"{}\"", self.relation));
            if let Some(ref mimetype) = self.mimetype {
                try!(write!(f, " type=\"{}\"", mimetype));
            }
            if let Some(ref language) = self.language {
                try!(write!(f, " hreflang=\"{}\"", language));
            }
            try!(write!(f, " href=\"{}\"", self.uri));
            if let Some(ref title) = self.title {
                try!(write!(f, " title=\"{}\"", title));
            }
            write!(f, ">")
        }
    }

    pub enum Predicate<'a> { Simple(&'a str), Regex(Regex) }
    impl<'a, 'b, 'c> Fn(&'c &'b Link) -> bool for Predicate<'a> {
        extern "rust-call" fn call(&self, args: (&'c &'b Link,)) -> bool {
            let (l,) = args;
            match (l.mimetype.as_ref(), self) {
                (None, _) => false,
                (Some(ref t), &Predicate::Simple(ref pattern)) =>
                    &t[] == *pattern,
                (Some(ref t), &Predicate::Regex(ref pattern)) =>
                    pattern.is_match(&t[]),
            }
        }
    }

    pub trait LinkIteratorExt<'a>: Iterator<Item=&'a Link> + IteratorExt {
        /// Filter links by their `mimetype` e.g.:
        ///
        /// ```
        /// # use earth::feed::{LinkList, LinkIteratorExt};
        /// # let links = LinkList(Vec::new());
        /// links.iter().filter_by_mimetype("text/html")
        /// # ;
        /// ```
        ///
        /// `pattern` can include wildcards (`*`) as well e.g.:
        ///
        /// ```
        /// # use earth::feed::{LinkList, LinkIteratorExt};
        /// # let links = LinkList(Vec::new());
        /// links.iter().filter_by_mimetype("application/xml+*")
        /// # ;
        /// ```
        fn filter_by_mimetype<'b>(self, pattern: &'b str) ->
            Filter<&'a Link, Self, Predicate<'b>>
        {
            use regex;
            if pattern.contains_char('*') {
                let mut regex_str = "^".to_string();
                let mut first = false;
                for part in pattern.split('*') {
                    if first {
                        first = true
                    } else {
                        regex_str.push_str(".+?")
                    }
                    regex_str.push_str(&regex::quote(part)[]);
                }
                regex_str.push('$');
                let regex = Regex::new(&regex_str[]);
                let regex = regex.unwrap();
                self.filter(Predicate::Regex(regex))
            } else {
                self.filter(Predicate::Simple(pattern))
            }
        }

        /*
        fn permalink(self) -> Option<&'a Link> {
            self.filter_map(|link| {
                let rel_is_alternate = link.relation == "alternate";
                if link.html.is_some() || rel_is_alternate {
                    Some((link, (link.html.as_ref(), rel_is_alternate)))
                } else {
                    None
                }
            }).max_by(|pair| pair.1).map(|pair| pair.0)
        }
        */

        fn favicon(mut self) -> Option<&'a Link> {
            for link in self {
                if link.relation.split(' ').any(|i| i == "icon") {
                    return Some(link);
                }
            }
            None
        }
    }

    impl<'a, I: Iterator<Item=&'a Link>> LinkIteratorExt<'a> for I { }
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


pub mod mark {
    use std::fmt;

    use chrono::{DateTime, FixedOffset};
    
    use schema::{Mergeable};

    /// Represent whether the entry is read, starred, or tagged by user.
    ///
    /// It's not a part of [RFC 4287 Atom standard][rfc-atom], but extension
    /// for Earth Reader.
    ///
    /// [rfc-atom]: https://tools.ietf.org/html/rfc4287
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
