#![unstable]

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
