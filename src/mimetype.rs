use std::borrow::ToOwned;
use std::fmt;

use regex;

#[derive(Clone, PartialEq, Eq, Show)]
pub enum MimeType {
    Text,
    Html,
    Xhtml,
    Other(String),
}

static MIMETYPE_PATTERN: regex::Regex = regex!(concat!(
    r#"^"#,
    r#"(?P<type>[A-Za-z0-9!#$&.+^_-]{1,127})"#,
    r#"/"#,
    r#"(?P<subtype>[A-Za-z0-9!#$&.+^_-]{1,127})"#,
    r#"$"#));

impl MimeType {
    pub fn from_str(mimetype: &str) -> Option<MimeType> {
        let captures = MIMETYPE_PATTERN.captures(mimetype);
        if let Some(captures) = captures {
            Some(match (captures.name("type"), captures.name("subtype")) {
                (Some("text"), Some("plain")) => MimeType::Text,
                (Some("text"), Some("html")) => MimeType::Html,
                (Some("application"), Some("xhtml+xml")) => MimeType::Xhtml,
                _ => MimeType::Other(mimetype.to_owned()),
            })
        } else {
            None
        }
    }

    pub fn mimetype(&self) -> &str {
        match *self {
            MimeType::Text => "text/plain",
            MimeType::Html => "text/html",
            MimeType::Xhtml => "application/xhtml+xml",
            MimeType::Other(ref mimetype) => &mimetype[],
        }
    }

    #[unstable = "incomplete"]
    pub fn is_text(&self) -> bool {
        match *self {
            MimeType::Other(ref _mimetype) => false,
            _ => true
        }
    }
}

impl fmt::String for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.mimetype())
    }
}
