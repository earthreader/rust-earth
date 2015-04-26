//! **rust-earth** is an alternative library of [libearth][], the shared common
//! library for various [Earth Reader][] apps.
//!
//! Earth Reader try to support many platforms as possible (e.g. [web][],
//! mobile apps, desktop apps), so there must be a large part of common concepts
//! and implementations they share like subscription lists, synchronization
//! through cloud storages between several devices, and crawler, that libearth
//! actually implements.  Rust-earth is developing to cover the corner cases
//! which libearth cannot be easily included like mobile devices.
//!
//! [libearth]: https://github.com/earthreader/libearth
//! [Earth Reader]: http://earthreader.org/
//! [web]: https://github.com/earthreader/web

#![doc(html_logo_url = "http://libearth.earthreader.org/en/0.2.0/_static/libearth.svg",
       html_favicon_url = "http://earthreader.org/favicon.ico",
       html_root_url = "http://earthreader.org/rust-earth/")]
#![cfg_attr(html_sanitizer, plugin(string_cache_plugin))]

extern crate chrono;
extern crate rustc_serialize as serialize;
extern crate regex;
extern crate tempdir;
extern crate time;
extern crate url;
extern crate xml;

#[cfg(html_sanitizer)] extern crate html5ever;
#[cfg(html_sanitizer)] extern crate string_cache;

pub mod macros;
pub mod test_utils;

pub mod codecs;
pub mod feed;
pub mod html;
pub mod mimetype;
pub mod parser;
pub mod repository;
pub mod sanitizer;
pub mod schema;
pub mod stage;
pub mod util;
