#![experimental]

use std::fmt;

#[experimental]
/// When a value can be semantically expressed as an HTML element, this trait
/// may be used.
///
/// ## Example
///
/// ```
/// # use earth::feed::Link;
/// # use earth::html::HtmlExt;
/// let link = Link::new("http://earthreader.org/");
/// assert_eq!(format!("{}", link), "http://earthreader.org/");
/// assert_eq!(format!("{}", link.to_html()),
///            r#"<link rel="alternate" href="http://earthreader.org/">"#);
/// ```
pub trait Html {
    fn fmt_html(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

#[experimental]
pub trait HtmlExt: Html {
    fn to_html(&self) -> HtmlWrapper<Self> { HtmlWrapper(self) }
}

impl<T: Html> HtmlExt for T { }

#[experimental]
pub struct HtmlWrapper<'a, T: ?Sized + 'a + Html>(&'a T);

impl<'a, T: Html> fmt::String for HtmlWrapper<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.0.fmt_html(f) }
}
