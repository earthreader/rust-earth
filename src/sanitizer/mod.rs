//! Sanitize HTML tags.
use std::borrow::Cow;
use std::fmt;

#[cfg(html_sanitizer)] mod html;
#[cfg(html_sanitizer)] pub use html::{clean_html, sanitize_html};

/// Convert given string to HTML-safe sequences by replacing the characters
/// `&`, `<` and `>`.  If the optional `flag` quote is true, the characters `"`
/// and `'` are also translated.
///
/// ### Example
///
/// ```
/// # use earth::sanitizer::escape;
/// let s = r#"<a href="http://example.org/?a=1&b=2">Example</a>"#;
/// assert_eq!(format!("{}", escape(s, false)),
///            r#"&lt;a href="http://example.org/?a=1&amp;b=2"&gt;Example&lt;/a&gt;"#);
/// assert_eq!(format!("{}", escape(s, true)),
///            r#"&lt;a href=&quot;http://example.org/?a=1&amp;b=2&quot;&gt;Example&lt;/a&gt;"#);
/// ```
pub fn escape<'a>(text: &'a str, quote: bool) -> Escape<'a> {
    Escape(text, if quote { QUOTE } else { ESCAPE })
}

#[doc(hidden)]
pub type EscapeTable<'a> = Cow<'a, [(char, &'static str)]>;

const ESCAPE: EscapeTable<'static> = Cow::Borrowed(&[
    ('&', "&amp;"),
    ('<', "&lt;"),
    ('>', "&gt;"),
]);

const QUOTE: EscapeTable<'static> = Cow::Borrowed(&[
    ('&', "&amp;"),
    ('<', "&lt;"),
    ('>', "&gt;"),
    ('\"', "&quot;"),
    ('\'', "&#x27;"),
]);

#[doc(hidden)]
pub const QUOTE_BR: EscapeTable<'static> = Cow::Borrowed(&[
    ('&', "&amp;"),
    ('<', "&lt;"),
    ('>', "&gt;"),
    ('\"', "&quot;"),
    ('\'', "&#x27;"),
    ('\n', "<br>\n"),
]);

pub struct Escape<'a>(#[doc(hidden)] pub &'a str,
                      #[doc(hidden)] pub EscapeTable<'static>);

impl<'a> fmt::Display for Escape<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let table = &self.1;
        let mut last_written = 0usize;
        for (i, ch) in self.0.char_indices() {
            let q = table.iter().filter_map(|&(m, alter)| {
                if ch == m { Some(alter) } else { None }
            }).next();
            if let Some(quoted) = q {
                try!(f.write_str(&self.0[last_written..i]));
                try!(f.write_str(quoted));
                last_written = i;
            }
        }
        if last_written < self.0.len() {
            try!(f.write_str(&self.0[last_written..]));
        }
        Ok(())
    }
}
