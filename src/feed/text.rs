#![unstable]

use super::Blob;

use std::borrow::ToOwned;
use std::default::Default;
use std::ops::Deref;
use std::fmt;

use html::{Html};
use mimetype::MimeType;
use sanitizer;
use sanitizer::{clean_html, sanitize_html};

use parser::base::{DecodeResult, DecodeError, XmlElement};
use schema::FromSchemaReader;


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
            Text::Plain(ref value) => {
                let s = sanitizer::Escape(&value[], sanitizer::QUOTE_BR);
                Box::new(s) as Box<fmt::String>
            }
            Text::Html(ref value) =>
                Box::new(sanitize_html(&value[], base_uri)) as Box<fmt::String>,
        }
    }
}

impl FromSchemaReader for Text {
    fn read_from<B: Buffer>(&mut self, element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        let type_ = match element.get_attr("type") {
            Ok("text") => "text",
            Ok("html") => "html",
            Ok(_type) => {
                // TODO: should be warned
                "text"
            }
            Err(DecodeError::AttributeNotFound(_)) => "text",
            Err(e) => { return Err(e); }
        };
        *self = Text::new(type_, try!(element.read_whole_text()));
        Ok(())
    }
}


#[cfg(test)]
mod test {
    use super::Text;

    use feed::Blob;

    #[test]
    fn test_text_str() {
        assert_eq!(Text::plain("Hello world").to_string(), "Hello world");
        assert_eq!(Text::plain("<p>Hello <em>world</em></p>").to_string(),
                   "<p>Hello <em>world</em></p>");
        assert_eq!(Text::html("Hello world").to_string(), "Hello world");
        assert_eq!(Text::html("<p>Hello <em>world</em></p>").to_string(),
                   "Hello world");
        assert_eq!(Text::html("<p>안녕 <em>세상</em>아</p>").to_string(),
                   "안녕 세상아");
    }

    macro_rules! assert_sanitized {
        ($text:expr, $expected:expr) => (
            assert_eq!($text.sanitized_html(None).to_string(), $expected);
        );
        ($text:expr, $base_uri:expr, $expected:expr) => (
            assert_eq!($text.sanitized_html(Some($base_uri)).to_string(), $expected);
        )
    }

    #[test]
    fn test_get_sanitized_html() {
        let text = Text::plain("Hello world");
        assert_sanitized!(text, "Hello world");
        let text = Text::plain("Hello\nworld");
        assert_sanitized!(text, "Hello<br>\nworld");
        let text = Text::plain("<p>Hello <em>world</em></p>");
        assert_sanitized!(text, concat!("&lt;p&gt;Hello &lt;em&gt;",
                                        "world&lt;/em&gt;&lt;/p&gt;"));
        let text = Text::plain("<p>안녕 <em>세상</em>아</p>");
        assert_sanitized!(text, concat!("&lt;p&gt;안녕 &lt;em&gt;",
                                        "세상&lt;/em&gt;아&lt;/p&gt;"));
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
