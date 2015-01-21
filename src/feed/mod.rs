use std::fmt;
use std::str::from_utf8;

use chrono::{DateTime, FixedOffset};

use codecs;
use mimetype::MimeType;
use parser::base::{DecodeResult, DecodeError, XmlElement};
use schema::Codec;

pub use self::category::Category;
pub use self::content::Content;
pub use self::entry::Entry;
pub use self::feed::Feed;
pub use self::generator::Generator;
pub use self::link::{Link, LinkIteratorExt, LinkList};
pub use self::mark::Mark;
pub use self::metadata::Metadata;
pub use self::person::Person;
pub use self::source::Source;
pub use self::text::Text;

pub mod category;
pub mod content;
pub mod entry;
pub mod feed;
pub mod generator;
pub mod link;
pub mod mark;
pub mod metadata;
pub mod person;
pub mod source;
pub mod text;


/// The XML namespace name used for Atom (RFC 4287).
const ATOM_XMLNS: &'static str = "http://www.w3.org/2005/Atom";

/// The XML namespace name used for Earth Reader `Mark` metadata.
const MARK_XMLNS: &'static str = "http://earthreader.org/mark/";


pub trait Blob {
    fn mimetype(&self) -> MimeType;

    fn is_text(&self) -> bool { self.mimetype().is_text() }

    fn as_bytes(&self) -> &[u8];

    fn as_str(&self) -> Option<&str> { from_utf8(self.as_bytes()).ok() }

    /// Get the secure HTML string of the text.  If it's a plain text, this
    /// returns entity-escaped HTML string, if it's a HTML text, `value` is
    /// sanitized, and if it's a binary data, this returns base64-encoded
    /// string.
    ///
    /// ```
    /// # use earth::feed::{Blob, Text};
    /// let text = Text::text("<Hello>");
    /// let html = Text::html("<script>alert(1);</script><p>Hello</p>");
    /// assert_eq!(format!("{}", text.sanitized_html(None)), "&lt;Hello&gt;");
    /// assert_eq!(format!("{}", html.sanitized_html(None)), "<p>Hello</p>");
    /// ```
    fn sanitized_html<'a>(&'a self, base_uri: Option<&'a str>) ->
        Box<fmt::String + 'a>;
}

pub fn parse_datetime<B: Buffer>(element: XmlElement<B>)
                                 -> DecodeResult<DateTime<FixedOffset>>
{
    match codecs::RFC3339.decode(&*try!(element.read_whole_text())) {
        Ok(v) => Ok(v),
        Err(e) => Err(DecodeError::SchemaError(e)),
    }
}
