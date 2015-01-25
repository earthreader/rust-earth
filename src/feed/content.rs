#![unstable]

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
        Box<fmt::Display + 'a>
    {
        match self.mimetype {
            MimeType::Text =>
                Box::new(escape(self.as_str().unwrap(), true))
                as Box<fmt::Display>,
            MimeType::Html | MimeType::Xhtml =>
                Box::new(sanitize_html(self.as_str().unwrap(), base_uri))
                as Box<fmt::Display>,
            ref mime if mime.is_text() =>
                Box::new(escape(self.as_str().unwrap(), true))
                as Box<fmt::Display>,
            _ =>
                Box::new(self.as_bytes().to_base64(base64::MIME))
                as Box<fmt::Display>,
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
        let source_uri = element.get_attr("src").ok().map(|v| v.to_string());
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

