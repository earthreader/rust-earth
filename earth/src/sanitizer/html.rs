use std::borrow::{Cow, IntoCow, ToOwned};
use std::default::Default;
use std::fmt;
use std::str::CharRange;

use html5ever::tokenizer::{Attribute, Tag, TokenSink, Token};
use html5ever::tokenizer::{CharacterTokens, CommentToken, NullCharacterToken,
                           ParseError, TagToken};
use html5ever::tokenizer::TagKind::{StartTag, EndTag};
use html5ever::driver::{tokenize_to, one_input};
use regex;
use url::{Url, UrlParser};

/// Strip *all* markup tags from HTML string.
/// That means, it simply makes the given HTML document a plain text.
///
/// ### Example
///
/// ```
/// # use earth::sanitizer::clean_html;
/// let s = "<em>Simple</em> example";
/// assert_eq!(format!("{}", clean_html(s)), "Simple example");
/// ```
pub fn clean_html<'a>(html: &'a str) -> CleanHtml<'a> {
    CleanHtml(html)
}

/// Sanitize the given HTML string.  It removes the following tags and
/// attributes that are not secure nor useful for RSS reader layout:
///
/// - `<script>` tags
/// - `display: none;` styles
/// - JavaScript event attributes e.g. `onclick`, `onload`
/// - `href` attributes that start with `javascript:`, `jscript:`,
///   `livescript:`, `vbscript:`, `data:`, `about:`, or `mocha:`.
///
/// Also, it rebases all links on the ``base_uri`` if it's given.
///
/// ### Example
///
/// ```
/// # use earth::sanitizer::sanitize_html;
/// let s = r#"<a href="a/b/c">Example</a>"#;
/// assert_eq!(format!("{}", sanitize_html(s, Some("http://example.org/"))),
///            r#"<a href="http://example.org/a/b/c">Example</a>"#);
/// ```
pub fn sanitize_html<'a>(html: &'a str, base_uri: Option<&str>) ->
    SanitizeHtml<'a>
{
    SanitizeHtml(html, base_uri.and_then(|e| Url::parse(e).ok()))
}

#[unstable]
pub struct CleanHtml<'a>(pub &'a str);

impl<'a> fmt::Display for CleanHtml<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sink = MarkupTagCleaner { w: f };
        tokenize_to(sink, one_input(self.0.to_owned()),
                    Default::default());
        Ok(())
    }
}

struct MarkupTagCleaner<'a, 'b: 'a> {
    w: &'a mut fmt::Formatter<'b>,
}

impl<'a, 'b> TokenSink for MarkupTagCleaner<'a, 'b> {
    fn process_token(&mut self, token: Token) {
        match token {
            CharacterTokens(b) => {
                self.w.write_str(&b).unwrap();
            }
            NullCharacterToken => self.w.write_str("\0").unwrap(),
            ParseError(_) => { }  // TODO
            _ => { }
        }
    }
}

#[unstable]
pub struct SanitizeHtml<'a>(pub &'a str, pub Option<Url>);

impl<'a> fmt::Display for SanitizeHtml<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sink = HtmlSanitizer {
            base_uri: &self.1,
            w: f,
            ignore: false,
        };
        tokenize_to(sink, one_input(self.0.to_owned()),
                    Default::default());
        Ok(())
    }
}

/// The regular expression pattern that matches to disallowed CSS properties.
#[inline]
fn disallowed_style_pattern() -> regex::Regex {
    Regex::new(r#"(^|;)\s*display\s*:\s*[a-z-]+\s*(?:;\s*|$)"#).unwrap()
}

/// The set of disallowed URI schemes e.g. `javascript:`.
static DISALLOWED_SCHEMES: &'static [&'static str] = &[
    "javascript:", "jscript:", "livescript:", "vbscript:", "data:",
    "about:", "mocha:",
];


struct HtmlSanitizer<'a, 'b: 'a> {
    base_uri: &'a Option<Url>,
    w: &'a mut fmt::Formatter<'b>,
    ignore: bool,
}

impl<'a, 'b> HtmlSanitizer<'a, 'b> {
    #[inline]
    fn write_fmt(&mut self, fmt: fmt::Arguments) {
        self.w.write_fmt(fmt).unwrap()
    }

    #[inline]
    fn write_str(&mut self, data: &str) {
        self.w.write_str(data).unwrap()
    }
}

fn remove_css(value: &str) -> String {
    disallowed_style_pattern().replace(value, "$1")
}

fn disallowed_scheme(value: &str) -> bool {
    DISALLOWED_SCHEMES.iter().any(|s| value.starts_with(*s))
}

impl<'a, 'b> TokenSink for HtmlSanitizer<'a, 'b> {
    fn process_token(&mut self, token: Token) {
        match (self.ignore, token) {
            (_, TagToken(Tag { kind: EndTag, name: atom!(script), .. })) => {
                self.ignore = false;
            }
            (_, TagToken(Tag { kind: EndTag, name, .. })) => {
                write!(self, "</{}>", name.as_slice());
            }
            (true, _) => { }
            (false, TagToken(Tag { kind: StartTag, name: atom!(script), .. })) => {
                self.ignore = true;
            }
            (false, TagToken(Tag { kind: StartTag, name, mut attrs, .. })) => {
                write!(self, "<{}", name.as_slice());
                if let Some(base_uri) = self.base_uri.as_ref() {
                    if name == atom!(a) || name == atom!(link) {
                        let mut url_parser = UrlParser::new();
                        let base_uri = url_parser.base_url(base_uri);
                        for &mut Attribute { ref name,
                                             ref mut value } in attrs.iter_mut() {
                            if name.local == atom!(href) {
                                match base_uri.parse(&value) {
                                    Ok(u) => { *value = u.serialize(); }
                                    Err(_) => { }  // ignore malformed url
                                }
                            }
                        }
                    }
                }
                for Attribute { name, value } in attrs.into_iter() {
                    write!(self, " {}", name.local.as_slice());
                    if !value.is_empty() {
                        let value = match name.local {
                            atom!(href) if disallowed_scheme(&value) => {
                                "".into_cow()
                            }
                            atom!(style) => {
                                remove_css(&value).into_cow()
                            }
                            _ => value.into_cow()
                        };
                        write!(self, "=\"{}\"", value);
                    }
                }
                write!(self, ">");
            }
            (false, CommentToken(c)) => write!(self, "<!--{}-->", c),
            (false, CharacterTokens(b)) => self.write_str(&b),
            (false, NullCharacterToken) => self.write_str("\0"),
            (_, ParseError(_)) => { }  // TODO
            _ => { }
        }
    }
}
