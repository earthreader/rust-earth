#![experimental]

use std::fmt;

pub fn escape<'a>(text: &'a str, quote: bool) -> Escape<'a> {
    Escape(text, quote)
}

pub fn clean_html<'a>(html: &'a str) -> MarkupTagCleaner<'a> {
    MarkupTagCleaner(html)
}

pub fn sanitize_html<'a, 'b: 'a>(html: &'a str, base_uri: Option<&'b str>) ->
    HtmlSanitizer<'a, 'b>
{
    HtmlSanitizer(html, base_uri)
}

#[experimental = "incomplete"]
pub struct Escape<'a>(pub &'a str, bool);

impl<'a> fmt::String for Escape<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for ch in self.0.chars() {
            // TODO
            try!(write!(f, "{}", ch));
        }
        Ok(())
    }
}

#[experimental = "incomplete"]
pub struct MarkupTagCleaner<'a>(pub &'a str);

impl<'a> fmt::String for MarkupTagCleaner<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO
        try!(write!(f, "{}", self.0));
        Ok(())
    }
}

#[experimental = "incomplete"]
pub struct HtmlSanitizer<'a, 'b: 'a>(pub &'a str, pub Option<&'b str>);

impl<'a, 'b> fmt::String for HtmlSanitizer<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO
        try!(write!(f, "{}", self.0));
        Ok(())
    }
}
