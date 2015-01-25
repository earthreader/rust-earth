#![unstable]

use std::borrow::ToOwned;
use std::default::Default;
use std::fmt;
use std::iter::{FromIterator, Filter};
use std::mem::swap;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use regex::Regex;

use html::ForHtml;
use parser::base::{DecodeResult, XmlElement};
use schema::{FromSchemaReader, Mergeable};
use util::merge_vec;

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

impl<'a> fmt::String for ForHtml<'a, Link> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

impl FromSchemaReader for Link {
    fn read_from<B: Buffer>(&mut self, element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        self.uri = try!(element.get_attr("href")).to_owned();
        self.relation = element.get_attr("rel")
                               .unwrap_or("alternate").to_owned();
        self.mimetype = element.get_attr("type").ok()
                               .map(ToOwned::to_owned);
        self.language = element.get_attr("hreflang").ok()
                               .map(ToOwned::to_owned);
        self.title = element.get_attr("title").ok()
                            .map(ToOwned::to_owned);
        self.byte_size = element.get_attr("length").ok()
                                .and_then(FromStr::from_str);
        Ok(())
    }
}


#[experimental]
pub enum Predicate<'a> {
    #[doc(hidden)] Simple(&'a str),
    #[doc(hidden)] Regex(Regex)
}

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

#[experimental]
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


#[deprecated = "wondering where this struct is needed"]
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

impl Mergeable for Vec<Link> {
    fn merge_with(&mut self, mut other: Vec<Link>) {
        swap(self, &mut other);
        merge_vec(self, other.into_iter());
    }
}


#[cfg(test)]
mod test {
    use super::{Link, LinkIteratorExt};

    use std::default::Default;

    use html::ToHtml;

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

    fn fx_feed_links() -> Vec<Link> {
        vec![
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
            },
        ]
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
