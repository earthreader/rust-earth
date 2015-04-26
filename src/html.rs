//! The adatper to display a given value as an HTML element.
//!
//! ## Example
//!
//! ```
//! use earth::feed::Link;
//! use earth::html::{ForHtml, ToHtml};
//! let link = Link::new("http://earthreader.org/");
//!
//! assert_eq!(format!("{}", link), "http://earthreader.org/");
//! assert_eq!(format!("{}", link.to_html()),
//!            r#"<link rel="alternate" href="http://earthreader.org/">"#);
//! ```
//!
//! If you want to convert the type of values to an HTML element, you can
//! implement `std::fmt::Display` trait on the type wrapped by this adapter.
//!
//! ```ignore
//! use earth::html::{ForHtml, ToHtml};
//! use std::fmt;
//!
//! struct Answer(i32);
//!
//! impl<'a> fmt::Display for ForHtml<'a, Answer> {
//!     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//!         write!(f, "<em>{}</em>", self.0)
//!     }
//! }
//! 
//! assert_eq!(format!("{}", Answer(42).to_html()), "<em>42</em>");
//! ```
//!
//! This pattern was suggested from [rust-lang/rfcs#565][].
//!
//! [rust-lang/rfcs#565]: https://github.com/rust-lang/rfcs/blob/master/text/0565-show-string-guidelines.md#user-facing-fmtdisplay

use std::ops::Deref;
use std::fmt;


pub struct ForHtml<'a, T: ?Sized + 'a> { _inner: &'a T }

impl<'a, T: ?Sized> Deref for ForHtml<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { self._inner }
}


pub trait ToHtml {
    fn to_html(&self) -> ForHtml<Self> { ForHtml { _inner: self } }
}

impl<'a, T> ToHtml for T where ForHtml<'a, T>: fmt::Display { }
