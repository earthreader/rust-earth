#![unstable]
//! Data structures for feeds.
//!
//! **rust-earth** internally stores archive data as Atom format, like
//! [libearth][] does.  It's exactly not a complete set of [RFC 4287][], but a
//! subset of the most of that.
//! Since it's not intended for crawling but internal representation, it does
//! not follow robustness principle or such thing.  It simply treats stored
//! data are all valid and well-formed.
//!
//! [libearth]: https://github.com/earthreader/libearth
//! [RFC 4287]: https://tools.ietf.org/html/rfc4287
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

mod category;
mod content;
mod entry;
mod feed;
mod generator;
mod link;
mod mark;
mod metadata;
mod person;
mod source;
mod text;


/// The XML namespace name used for Atom (RFC 4287).
const ATOM_XMLNS: &'static str = "http://www.w3.org/2005/Atom";

/// The XML namespace name used for Earth Reader `Mark` metadata.
const MARK_XMLNS: &'static str = "http://earthreader.org/mark/";


#[experimental]
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

fn parse_datetime<B: Buffer>(element: XmlElement<B>)
                                 -> DecodeResult<DateTime<FixedOffset>>
{
    match codecs::RFC3339.decode(&*try!(element.read_whole_text())) {
        Ok(v) => Ok(v),
        Err(e) => Err(DecodeError::SchemaError(e)),
    }
}
