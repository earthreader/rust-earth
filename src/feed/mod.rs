use std::fmt;
use std::str::from_utf8;

use mimetype::MimeType;

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
pub mod schema;
pub mod source;
pub mod text;


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
    /// ```ignore
    /// # use earth::feed::Text;
    /// let text = Text::text("<Hello>");
    /// let html = Text::html("<script>alert(1);</script><p>Hello</p>");
    /// assert_eq!(format!("{}", text.sanitized_html(None)), "&lt;Hello&gt;");
    /// assert_eq!(format!("{}", html.sanitized_html(None)), "<p>Hello</p>");
    /// ```
    fn sanitized_html<'a>(&'a self, base_uri: Option<&'a str>) ->
        Box<fmt::String + 'a>;
}


#[cfg(test)]
mod test {
    use super::{Feed, Link, Person, Text};
    use super::feed::read_feed;

    use chrono::{Offset, UTC};
    
    fn fx_feed() -> Feed {
        read_feed(r##"
        <feed xmlns="http://www.w3.org/2005/Atom"
              xmlns:mark="http://earthreader.org/mark/">
            <title>Example Feed</title>
            <link href="http://example.org/"/>
            <updated>2003-12-13T18:30:02Z</updated>
            <author><name>John Doe</name></author>
            <author><name>Jane Doe</name></author>
            <id>urn:uuid:60a76c80-d399-11d9-b93C-0003939e0af6</id>
            <category term="technology"/>
            <category term="business"/>
            <rights>Public Domain</rights>
            <entry>
                <title>Atom-Powered Robots Run Amok</title>
                <link href="http://example.org/2003/12/13/atom03"/>
                <id>urn:uuid:1225c695-cfb8-4ebb-aaaa-80da344efa6a</id>
                <updated>2003-12-13T18:30:02Z</updated>
                <summary>Some text.</summary>
                <author><name>Jane Doe</name></author>
                <mark:read updated="2013-11-06T14:36:00Z">true</mark:read>
            </entry>
            <entry>
                <title>Danger, Will Robinson!</title>
                <link href="http://example.org/2003/12/13/lost"/>
                <id>urn:uuid:b12f2c10-ffc1-11d9-8cd6-0800200c9a66</id>
                <updated>2003-12-13T18:30:02Z</updated>
                <summary>Don't Panic!</summary>
            </entry>
        </feed>
        "## // "
        .as_bytes())
    }

    #[test]
    fn test_feed_read() {
        let feed = fx_feed();
        assert_eq!(feed.title, Text::text("Example Feed"));
        assert_eq!(feed.links.len(), 1);
        let ref link = feed.links[0];
        assert_eq!(link.relation, "alternate");
        assert_eq!(link.uri, "http://example.org/");
        assert_eq!(feed.updated_at, UTC.ymd(2003, 12, 13).and_hms(18, 30, 2));
        let ref authors = feed.authors;
        assert_eq!(feed.authors.len(), 2);
        assert_eq!(authors[0].name, "John Doe");
        assert_eq!(authors[1].name, "Jane Doe");
        assert_eq!(feed.id, "urn:uuid:60a76c80-d399-11d9-b93C-0003939e0af6");
        let ref categories = feed.categories;
        assert_eq!(categories.len(), 2);
        assert_eq!(categories[0].term, "technology");
        assert_eq!(categories[1].term, "business");
        assert_eq!(feed.rights, Some(Text::text("Public Domain")));
        let ref entries = feed.entries;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title,
                   Text::text("Atom-Powered Robots Run Amok"));
        assert_eq!(&entries[0].links[],
                   [Link::new("http://example.org/2003/12/13/atom03")]);
        assert_eq!(entries[0].id,
                   "urn:uuid:1225c695-cfb8-4ebb-aaaa-80da344efa6a");
        assert_eq!(entries[0].updated_at,
                   UTC.ymd(2003, 12, 13).and_hms(18, 30, 2));
        assert_eq!(entries[0].summary, Some(Text::text("Some text.")));
        assert_eq!(&entries[0].authors[], [Person::new("Jane Doe")]);
        assert_eq!(entries[1].title, Text::text("Danger, Will Robinson!"));
        assert_eq!(&entries[1].links[],
                   [Link::new("http://example.org/2003/12/13/lost")]);
        assert_eq!(entries[1].id,
                   "urn:uuid:b12f2c10-ffc1-11d9-8cd6-0800200c9a66");
        assert_eq!(entries[1].updated_at,
                   UTC.ymd(2003, 12, 13).and_hms(18, 30, 2));
        assert_eq!(entries[1].summary, Some(Text::text("Don't Panic!")));
    }
}
