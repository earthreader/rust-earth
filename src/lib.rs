#![feature(plugin, unboxed_closures, unsafe_destructor)]
#![allow(unstable)]

#[plugin]
#[no_link]
extern crate regex_macros;

extern crate serialize;
extern crate regex;
extern crate time;

extern crate chrono;
extern crate url;
extern crate xml;

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
