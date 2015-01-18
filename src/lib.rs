#![feature(plugin, unsafe_destructor)]
#![allow(unstable)]

#[plugin]
extern crate regex_macros;

extern crate regex;
extern crate time;

extern crate chrono;
extern crate url;
extern crate xml;

pub mod macros;

pub mod codecs;
pub mod feed;
pub mod parser;
pub mod repository;
pub mod schema;
pub mod stage;
