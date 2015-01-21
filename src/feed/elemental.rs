use std;

use mimetype::MimeType;

pub use self::text::Text;
pub use self::person::Person;
pub use self::link::{Link, LinkIteratorExt, LinkList};
pub use self::category::Category;
pub use self::content::Content;
pub use self::generator::Generator;
pub use self::mark::Mark;


pub trait Blob {
    fn mimetype(&self) -> MimeType;

    fn is_text(&self) -> bool { self.mimetype().is_text() }

    fn as_bytes(&self) -> &[u8];

    fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(self.as_bytes()).ok()
    }

    /// Get the secure HTML string of the text.  If it's a plain text, this
    /// returns entity-escaped HTML string, if it's a HTML text, `value` is
    /// sanitized, and if it's a binary data, this returns base64-encoded
    /// string.
    ///
    /// ```ignore
    /// # use earth::feed::Text;
    /// let text = Text::text("<Hello>");
    /// let html = Text::html("<script>alert(1);</script><p>Hello</p>");
    /// assert_eq!(format!("{}", text.sanitized_html(None)), "&lt;Hello&gt;");
    /// assert_eq!(format!("{}", html.sanitized_html(None)), "<p>Hello</p>");
    /// ```
    fn sanitized_html<'a>(&'a self, base_uri: Option<&'a str>) ->
        Box<std::fmt::String + 'a>;
}


#[unstable]
pub mod text {
    use super::Blob;

    use std::borrow::ToOwned;
    use std::default::Default;
    use std::ops::Deref;
    use std::fmt;

    use html::{Html};
    use mimetype::MimeType;
    use sanitizer::{clean_html, escape, sanitize_html};

    /// Text construct defined in :rfc:`4287#section-3.1` (section 3.1).
    ///
    /// RFC: <https://tools.ietf.org/html/rfc4287#section-3.1>
    ///
    /// Note: It currently does not support `xhtml`.
    #[unstable]
    #[derive(PartialEq, Eq, Show)]
    pub enum Text {
        /// The plain text content.  It corresponds to :rfc:`4287#section-3.1.1.1` (section 3.1.1.1).
        ///
        /// [rfc-text-1.1]: https://tools.ietf.org/html/rfc4287#section-3.1.1.1
        Plain(String),

        /// The HTML content.  It corresponds to :rfc:`4287#section-3.1.1.2` (section 3.1.1.2).
        ///
        /// [rfc-text-1.2]: https://tools.ietf.org/html/rfc4287#section-3.1.1.2
        Html(String),
    }

    impl Text {
        pub fn new<T, S: ?Sized>(type_: &str, value: T) -> Text
            where T: Deref<Target=S>, S: ToOwned<String>
        {
            match type_ {
                "text" => Text::plain(value),
                "html" => Text::html(value),
                _ => Text::plain(value),
            }
        }

        #[deprecated = "use Text::Plain(value.to_string()) or Text::plain(value) instead"]
        pub fn text<T, S: ?Sized>(value: T) -> Text
            where T: Deref<Target=S>, S: ToOwned<String>
        {
            Text::plain(value)
        }

        pub fn plain<T, S: ?Sized>(value: T) -> Text
            where T: Deref<Target=S>, S: ToOwned<String>
        {
            Text::Plain(value.to_owned())
        }

        pub fn html<T, S: ?Sized>(value: T) -> Text
            where T: Deref<Target=S>, S: ToOwned<String>
        {
            Text::Html(value.to_owned())
        }

        /// The type of the text.  It corresponds to :rfc:`4287#section-3.1.1` (section 3.1.1).
        ///
        /// [rfc-text-1]: https://tools.ietf.org/html/rfc4287#section-3.1.1
        pub fn type_(&self) -> &'static str {
            match *self {
                Text::Plain(_) => "text",
                Text::Html(_) => "html",
            }
        }
    }

    impl Default for Text {
        fn default() -> Text {
            Text::Plain("".to_owned())
        }
    }

    impl fmt::String for Text {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                Text::Plain(ref value) => write!(f, "{}", value),
                Text::Html(ref value) => write!(f, "{}", clean_html(&value[])),
            }
        }
    }

    impl Html for Text {
        fn fmt_html(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.sanitized_html(None))
        }
    }

    impl Blob for Text {
        fn mimetype(&self) -> MimeType {
            match *self {
                Text::Plain(_) => MimeType::Text,
                Text::Html(_) => MimeType::Html,
            }
        }

        fn is_text(&self) -> bool { true }

        fn as_bytes(&self) -> &[u8] { self.as_str().unwrap().as_bytes() }

        fn as_str(&self) -> Option<&str> {
            let value = match *self {
                Text::Plain(ref value) => value,
                Text::Html(ref value) => value,
            };
            Some(&value[])
        }

        #[unstable = "incomplete"]
        fn sanitized_html<'a>(&'a self, base_uri: Option<&'a str>) ->
            Box<fmt::String + 'a>
        {
            match *self {
                Text::Plain(ref value) =>
                    Box::new(escape(&value[], true)) as Box<fmt::String>,
                Text::Html(ref value) =>
                    Box::new(sanitize_html(&value[], base_uri)) as Box<fmt::String>,
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::Text;

        use feed::elemental::Blob;

        #[ignore]
        #[test]
        fn test_text_str() {
            assert_eq!(Text::plain("Hello world").to_string(), "Hello world");
            assert_eq!(Text::plain("<p>Hello <em>world</em></p>").to_string(),
                       "<p>Hello <em>world</em></p>");
            assert_eq!(Text::html("Hello world").to_string(), "Hello world");
            assert_eq!(Text::html("<p>Hello <em>world</em></p>").to_string(),
                       "Hello world");
        }

        macro_rules! assert_sanitized {
            ($text:expr, $expected:expr) => (
                assert_eq!($text.sanitized_html(None).to_string(), $expected);
            );
            ($text:expr, $base_uri:expr, $expected:expr) => (
                assert_eq!($text.sanitized_html(Some($base_uri)).to_string(), $expected);
            )
        }

        #[ignore]
        #[test]
        fn test_get_sanitized_html() {
            let text = Text::plain("Hello world");
            assert_sanitized!(text, "Hello world");
            let text = Text::plain("Hello\nworld");
            assert_sanitized!(text, "Hello<br>\nworld");
            let text = Text::plain("<p>Hello <em>world</em></p>");
            assert_sanitized!(text, concat!("&lt;p&gt;Hello &lt;em&gt;",
                                            "world&lt;/em&gt;&lt;/p&gt;"));
            let text = Text::html("Hello world");
            assert_sanitized!(text, "Hello world");
            let text = Text::html("<p>Hello <em>world</em></p>");
            assert_sanitized!(text, "<p>Hello <em>world</em></p>");
            let text = Text::html("<p>Hello</p><script>alert(1);</script>");
            assert_sanitized!(text, "<p>Hello</p>");
            let text = Text::html("<p>Hello</p><hr noshade>");
            assert_sanitized!(text, "<p>Hello</p><hr noshade>");
            let text = Text::html("<a href=\"/abspath\">abspath</a>");
            assert_sanitized!(text, "http://localhost/path/",
                              concat!("<a href=\"http://localhost/abspath\">",
                                      "abspath</a>"));
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

    #[cfg(test)]
    mod test {
        use super::{Person};

        use html::{HtmlExt};

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

        #[ignore]
        #[test]
        fn test_person_html() {
            assert_html!(Person::new("Hong \"Test\" Minhee"),
                         "Hong &quot;Test&quot; Minhee");
            assert_html!(Person { name: "Hong Minhee".to_string(),
                                  uri: Some("http://dahlia.kr/".to_string()),
                                  email: None },
                         "<a href=\"http://dahlia.kr/\">Hong Minhee</a>");
            let email = "\x6d\x69\x6e\x68\x65\x65\x40\x64\x61\x68\x6c\x69\x61\x2e\x6b\x72";
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
}

#[unstable]
pub mod link {
    use std::borrow::ToOwned;
    use std::default::Default;
    use std::fmt;
    use std::iter::{FromIterator, Filter};
    use std::ops::{Deref, DerefMut};

    use regex::Regex;

    use html::Html;

    /// Link element defined in RFC 4287 (section 4.2.7).
    ///
    /// RFC: <https://tools.ietf.org/html/rfc4287#section-4.2.7>.
    #[unstable]
    #[derive(Clone, PartialEq, Eq, Hash, Show)]
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
                let mut first = true;
                for part in pattern.split('*') {
                    if first {
                        first = false
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

        fn permalink(self) -> Option<&'a Link> {
            self.filter_map(|link| {
                let rel_is_alternate = link.relation == "alternate";
                if link.is_html() || rel_is_alternate {
                    Some((link, (link.is_html(), rel_is_alternate)))
                } else {
                    None
                }
            }).max_by(|pair| pair.1).map(|pair| pair.0)
        }

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

    
    #[derive(Default, Show)]
    pub struct LinkList(pub Vec<Link>);

    impl LinkList {
        pub fn new() -> LinkList { LinkList(Vec::new()) }
    }

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

    #[cfg(test)]
    mod test {
        use super::{Link, LinkIteratorExt, LinkList};

        use std::default::Default;

        use html::HtmlExt;

        #[test]
        fn test_link_html_property() {
            let mut link = Link::new("http://dahlia.kr/");
            link.mimetype = Some("text/html".to_string());
            assert!(link.is_html());
            link.mimetype = Some("application/xhtml+xml".to_string());
            assert!(link.is_html());
            link.mimetype = Some("application/xml".to_string());
            assert!(!link.is_html());
        }

        #[test]
        fn test_link_str() {
            let link = Link {
                uri: "http://dahlia.kr/".to_string(),
                relation: "alternate".to_string(),
                mimetype: Some("text/html".to_string()),
                title: Some("Hong Minhee's website".to_string()),
                language: None, byte_size: None,
            };
            assert_eq!(link.to_string(), "http://dahlia.kr/");
        }

        #[test]
        fn test_link_html_method() {
            let link = Link::new("http://dahlia.kr/");
            assert_html!(link,
                         "<link rel=\"alternate\" href=\"http://dahlia.kr/\">");
            let link = Link {
                uri: "http://dahlia.kr/".to_string(),
                relation: "alternate".to_string(),
                mimetype: Some("text/html".to_string()),
                title: Some("Hong Minhee's website".to_string()),
                language: Some("en".to_string()),
                byte_size: None
            };
            assert_html!(link,
                         concat!("<link rel=\"alternate\" type=\"text/html\" ",
                                 "hreflang=\"en\" href=\"http://dahlia.kr/\" ",
                                 "title=\"Hong Minhee\'s website\">"));
        }

        fn fx_feed_links() -> LinkList {
            LinkList(vec![
                Link::new("http://example.org/"),
                Link {
                    relation: "alternate".to_string(),
                    mimetype: Some("text/html".to_string()),
                    uri: "http://example.com/index.html".to_string(),
                    title: None, language: None, byte_size: None,
                },
                Link {
                    relation: "alternate".to_string(),
                    mimetype: Some("text/html".to_string()),
                    uri: "http://example.com/index2.html".to_string(),
                    title: None, language: None, byte_size: None,
                },
                Link {
                    relation: "alternate".to_string(),
                    mimetype: Some("text/xml".to_string()),
                    uri: "http://example.com/index.xml".to_string(),
                    title: None, language: None, byte_size: None,
                },
                Link {
                    relation: "alternate".to_string(),
                    mimetype: Some("application/json".to_string()),
                    uri: "http://example.com/index.json".to_string(),
                    title: None, language: None, byte_size: None,
                },
                Link {
                    relation: "alternate".to_string(),
                    mimetype: Some("text/javascript".to_string()),
                    uri: "http://example.com/index.js".to_string(),
                    title: None, language: None, byte_size: None,
                },
                Link {
                    relation: "alternate".to_string(),
                    mimetype: Some("application/xml+atom".to_string()),
                    uri: "http://example.com/index.atom".to_string(),
                    title: None, language: None, byte_size: None,
                },
                Link {
                    relation: "alternate".to_string(),  // remove it if available
                    mimetype: Some("application/xml+rss".to_string()),
                    uri: "http://example.com/index.atom".to_string(),
                    title: None, language: None, byte_size: None,
                },
                Link {
                    relation: "icon".to_string(),
                    mimetype: Some("image/png".to_string()),
                    uri: "http://example.com/favicon.png".to_string(),
                    title: None, language: None, byte_size: None,
                }
            ])
        }

        #[test]
        fn test_link_list_filter_by_mimetype() {
            let links = fx_feed_links();
            let result: Vec<_> = links.iter()
                .filter_by_mimetype("text/html")
                .collect();
            assert_eq!(result.len(), 2);
            assert_eq!(result.iter()
                             .map(|link| &link.mimetype.as_ref().unwrap()[])
                             .collect::<Vec<_>>(),
                       ["text/html", "text/html"]);
            let result: Vec<_> = links.iter()
                .filter_by_mimetype("application/*")
                .collect();
            assert_eq!(result.len(), 3);
            assert_eq!(result.iter()
                             .map(|link| &link.mimetype.as_ref().unwrap()[])
                             .collect::<Vec<_>>(),
                       ["application/json",
                        "application/xml+atom",
                        "application/xml+rss"]);
        }

        #[test]
        fn test_link_list_permalink() {
            let mut links = fx_feed_links();
            let mut other_link = Link::new("http://example.com/");
            other_link.relation = "other".to_string();
            let mut html_link = Link::new("http://example.com/");
            html_link.relation = "other".to_string();
            html_link.mimetype = Some("text/html".to_string());
            links.extend(vec![other_link, html_link.clone()].into_iter());
            assert_eq!(links.iter().permalink(), Some(&links[1]));
            links.remove(1);
            links.remove(1);
            assert_eq!(links.iter().permalink(), Some(&html_link));
            links.pop();
            assert_eq!(links.iter().permalink(), Some(&links[0]));
            assert_eq!(links[links.len() - 1..].iter().permalink(), None);
        }

        #[test]
        fn test_link_list_favicon() {
            let mut links = fx_feed_links();
            assert_eq!(links.iter().favicon(), links.last());
            links[0] = Link {
                relation: "shortcut icon".to_string(),
                uri: "http://example.com/favicon.ico".to_string(),
                ..Default::default()
            };
            assert_eq!(links.iter().favicon(), links.first());
        }
    }
}

#[unstable]
pub mod category {
    use std::fmt;

    use schema::Mergeable;

    /// Category element defined in :rfc:`4287#section-4.2.2` (section 4.2.2).
    #[derive(Default, Show)]
    pub struct Category {
        /// The required machine-readable identifier string of the cateogry.
        /// It corresponds to ``term`` attribute of :rfc:`4287#section-4.2.2.1` (section 4.2.2.1).
        pub term: String,

        /// The URI that identifies a categorization scheme.  It corresponds to
        /// ``scheme`` attribute of :rfc:`4287#section-4.2.2.2` (section 4.2.2.2).
        ///
        /// ### See also
        ///
        /// * [Tag Scheme?][scheme-1] by Tim Bray
        /// * [Representing tags in Atom][scheme-2] by Edward O'Connor
        ///
        /// [scheme-1]: http://www.tbray.org/ongoing/When/200x/2007/02/01/Tag-Scheme
        /// [scheme-2]: http://edward.oconnor.cx/2007/02/representing-tags-in-atom
        pub scheme_uri: Option<String>,

        /// The optional human-readable label for display in end-user
        /// applications.  It corresponds to ``label`` attribute of :rfc:`4287#section-4.2.2.3` (section 4.2.2.3).
        pub label: Option<String>,
    }

    impl Category {
        #[experimental = "should be exposed as a trait"]
        fn __entity_id__(&self) -> &str { &self.term[] }
    }

    impl Mergeable for Category {
        fn merge_entities(mut self, other: Category) -> Category {
            if self.label.is_none() {
                self.label = other.label
            }
            self
        }
    }

    impl fmt::String for Category {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.label.as_ref().unwrap_or(&self.term))
        }
    }

    #[cfg(test)]
    mod test {
        use super::Category;

        use std::default::Default;

        #[test]
        fn test_category_str() {
            assert_eq!(Category { term: "python".to_string(),
                                  ..Default::default() }.to_string(),
                       "python");
            assert_eq!(Category { term: "python".to_string(),
                                  label: Some("Python".to_string()),
                                  ..Default::default() }.to_string(),
                       "Python");
        }
    }
}

#[unstable]
pub mod content {
    use super::Blob;

    use std::borrow::ToOwned;
    use std::default::Default;
    use std::fmt;
    use std::ops::Deref;
    use std::str::{Utf8Error, from_utf8, from_utf8_unchecked};
    
    use serialize::base64;
    use serialize::base64::ToBase64;

    use mimetype::MimeType;
    use parser::base::{DecodeError, DecodeResult, XmlElement};
    use sanitizer::{escape, sanitize_html};
    use schema::{FromSchemaReader};

    /// Content construct defined in :rfc:`4287#section-4.1.3` (section 4.1.3).
    #[derive(Clone, Show)]
    pub struct Content {
        mimetype: MimeType,
        body: Vec<u8>,
        source_uri: Option<String>,
    }

    impl Content {
        pub fn new<T, S: ?Sized>(mimetype: MimeType, body: Vec<u8>,
                                 source_uri: Option<T>)
                                 -> Result<Content, Utf8Error>
            where T: Deref<Target=S>, S: ToOwned<String>
        {
            if mimetype.is_text() {
                try!(from_utf8(&body[]));
            }
            Ok(Content {
                mimetype: mimetype,
                body: body,
                source_uri: source_uri.map(|e| e.to_owned())
            })
        }

        pub fn from_str<T, S: ?Sized>(mimetype: &str, text: String,
                                      source_uri: Option<T>) -> Option<Content>
            where T: Deref<Target=S>, S: ToOwned<String>
        {
            let mimetype = match mimetype {
                "text" | "" => Some(MimeType::Text),
                "html" => Some(MimeType::Html),
                _ => MimeType::from_str(mimetype),
            };
            if let Some(mimetype) = mimetype {
                Some(Content {
                    mimetype: mimetype,
                    body: text.into_bytes(),
                    source_uri: source_uri.map(|e| e.to_owned()),
                })
            } else {
                None
            }
        }

        pub fn source_uri(&self) -> Option<&str> {
            self.source_uri.as_ref().map(|e| &e[])
        }
    }

    impl Blob for Content {
        fn mimetype(&self) -> MimeType { self.mimetype.clone() }
        fn is_text(&self) -> bool { self.mimetype.is_text() }
        fn as_bytes(&self) -> &[u8] { &self.body[] }
        fn as_str(&self) -> Option<&str> {
            if self.is_text() {
                Some(unsafe { from_utf8_unchecked(self.as_bytes()) })
            } else {
                None
            }
        }

        fn sanitized_html<'a>(&'a self, base_uri: Option<&'a str>) ->
            Box<fmt::String + 'a>
        {
            match self.mimetype {
                MimeType::Text =>
                    Box::new(escape(self.as_str().unwrap(), true))
                    as Box<fmt::String>,
                MimeType::Html | MimeType::Xhtml =>
                    Box::new(sanitize_html(self.as_str().unwrap(), base_uri))
                    as Box<fmt::String>,
                ref mime if mime.is_text() =>
                    Box::new(escape(self.as_str().unwrap(), true))
                    as Box<fmt::String>,
                _ =>
                    Box::new(self.as_bytes().to_base64(base64::MIME))
                    as Box<fmt::String>,
            }
        }
    }

    impl Default for Content {
        fn default() -> Content {
            Content {
                mimetype: MimeType::Text,
                body: vec![],
                source_uri: None
            }
        }
    }

    impl PartialEq for Content {
        fn eq(&self, other: &Content) -> bool {
            if self.source_uri.is_some() {
                (self.mimetype == other.mimetype &&
                 self.source_uri.as_ref() == other.source_uri.as_ref())
            } else {
                self.body == other.body
            }
        }
    }

    impl FromSchemaReader for Content {
        fn read_from<B: Buffer>(&mut self, element: XmlElement<B>)
                                -> DecodeResult<()>
        {
            let source_uri = element.get_attr("src").ok()
                                    .map(|v| v.to_string());
            let mimetype = {
                let m = element.get_attr("type")
                               .map(|v| (MimeType::from_str(v), v));
                match m {
                    Ok((Some(mimetype), _)) => mimetype,
                    Ok((None, "text"))  => MimeType::Text,
                    Ok((None, "html"))  => MimeType::Html,
                    Ok((None, "xhtml")) => MimeType::Xhtml,
                    Ok((None, _)) => MimeType::Text,  // TODO: should be an error
                    Err(DecodeError::AttributeNotFound(_)) => MimeType::Text,
                    Err(e) => { return Err(e); }
                }
            };
            let content = try!(element.read_whole_text());
            // TODO: if mimetype is binary, content should be decoded by base64
            self.source_uri = source_uri;
            self.mimetype = mimetype;
            self.body = content.into_bytes();
            Ok(())
        }
    }
    
    #[cfg(nocompile)]
    mod test {
        use super::{Content, MimeType};

        #[test]
        fn test_content_mimetype() {
            assert_eq!(Content::new("text", "Hello".to_string(),
                                    None).mimetype(),
                       "text/plain");
            assert_eq!(Content::new("html", "Hello".to_string(),
                                    None).mimetype(),
                       "text/html");
            assert_eq!(Content::new("text/xml", "<a>Hello</a>".to_string(),
                                    None).mimetype(),
                       "text/xml");
            assert_eq!(Content::new("text/plain", "Hello".to_string(),
                                    None).text.type_(),
                       "text");
            assert_eq!(Content::new("text/html", "Hello".to_string(),
                                    None).text.type_(),
                       "html");
            assert_eq!(Content::new("text/xml", "<a>Hello</a>".to_string(),
                                    None).text.type_(),
                       "text/xml");
        }

        #[test]
        fn test_invalid_mimetype() {
            assert_eq!(MimeType::from_str("invalid/mime/type"), None);
            assert_eq!(MimeType::from_str("invalidmimetype"), None);
            assert_eq!(MimeType::from_str("invalid/(mimetype)"), None);
        }
    }
}

#[unstable]
pub mod generator {
    use std::fmt;

    use html::Html;
    use sanitizer::escape;

    /// Identify the agent used to generate a feed, for debugging and other
    /// purposes.  It's corresponds to ``atom:generator`` element of
    /// :rfc:`4287#section-4.2.4` (section 4.2.4).
    #[derive(Default, PartialEq, Eq)]
    pub struct Generator {
        /// A URI that represents something relavent to the agent.
        pub uri: Option<String>,
        /// The version of the generating agent.
        pub version: Option<String>,
        /// The human-readable name for the generating agent.
        pub value: String,
    }

    impl fmt::String for Generator {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            try!(write!(f, "{}", self.value));
            if let Some(ref version) = self.version {
                try!(write!(f, " {}", version));
            }
            Ok(())
        }
    }

    impl Html for Generator {
        fn fmt_html(&self, f: &mut fmt::Formatter) -> fmt::Result {
            if let Some(ref uri) = self.uri {
                try!(write!(f, "<a href=\"{}\">", escape(&uri[], false)));
            }
            try!(write!(f, "{}", escape(&self.value[], false)));
            if let Some(ref version) = self.version {
                try!(write!(f, " {}", version));
            }
            if let Some(_) = self.uri {
                try!(write!(f, "</a>"));
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod test {
        use super::Generator;

        use html::HtmlExt;

        #[test]
        fn test_generator_str() {
            assert_eq!(Generator { value: "Earth Reader".to_string(),
                                   uri: None, version: None }.to_string(),
                       "Earth Reader");
            assert_eq!(
                Generator {
                    value: "Earth Reader".to_string(),
                    uri: Some("http://earthreader.github.io/".to_string()),
                    version: None
                }.to_string(),
                "Earth Reader");
            assert_eq!(Generator { value: "Earth Reader".to_string(),
                                   version: Some("1.0".to_string()),
                                   uri: None }.to_string(),
                       "Earth Reader 1.0");
            assert_eq!(
                Generator {
                    value: "Earth Reader".to_string(),
                    version: Some("1.0".to_string()),
                    uri: Some("http://earthreader.github.io/".to_string())
                }.to_string(),
                "Earth Reader 1.0");
        }

        #[ignore]
        #[test]
        fn test_generator_html() {
            assert_html!(Generator { value: "Earth Reader".to_string(),
                                     uri: None, version: None },
                         "Earth Reader");
            assert_html!(Generator { value: "<escape test>".to_string(),
                                     uri: None, version: None },
                         "&lt;escape test&gt;");
            assert_html!(
                Generator {
                    value: "Earth Reader".to_string(),
                    uri: Some("http://earthreader.github.io/".to_string()),
                    version: None,
                },
                "<a href=\"http://earthreader.github.io/\">Earth Reader</a>");
            assert_html!(Generator { value: "Earth Reader".to_string(),
                                     version: Some("1.0".to_string()),
                                     uri: None },
                         "Earth Reader 1.0");
            assert_html!(
                Generator {
                    value: "Earth Reader".to_string(),
                    version: Some("1.0".to_string()),
                    uri: Some("http://earthreader.github.io/".to_string())
                },
                "<a href=\"http://earthreader.github.io/\">Earth Reader 1.0</a>");
        }
    }
}

#[unstable]
pub mod mark {
    use chrono::{DateTime, FixedOffset};
    
    use schema::{Mergeable};

    /// Represent whether the entry is read, starred, or tagged by user.
    ///
    /// It's not a part of [RFC 4287 Atom standard][rfc-atom], but extension
    /// for Earth Reader.
    ///
    /// [rfc-atom]: https://tools.ietf.org/html/rfc4287
    #[derive(Default, PartialEq, Eq, Hash, Show)]
    pub struct Mark {
        /// Whether it's marked or not.
        pub marked: bool,

        /// Updated time.
        pub updated_at: Option<DateTime<FixedOffset>>,
    }

    impl Mark {
        /// If there are two or more marks that have the same tag name, these
        /// are all should be merged into one.
        #[experimental = "should be exposed as a trait"]
        fn __entity_id__(&self) -> &str { "" }
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

    #[cfg(test)]
    mod test {
        use super::Mark;

        use chrono::{Offset, FixedOffset};

        fn fx_mark_true() -> Mark {
            Mark {
                marked: true,
                updated_at: Some(FixedOffset::east(0).ymd(2013, 11, 6)
                                                     .and_hms(14, 36, 0)),
            }
        }

        fn fx_mark_false() -> Mark {
            Mark {
                marked: false,
                updated_at: Some(FixedOffset::east(0).ymd(2013, 11, 6)
                                                     .and_hms(14, 36, 0)),
            }
        }
    }
}
