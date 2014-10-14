#![feature(phase, struct_variant, macro_rules, unsafe_destructor, slicing_syntax)]

#[phase(plugin)]
extern crate regex_macros;

extern crate regex;
extern crate time;

extern crate chrono;
extern crate xml;

pub mod macros;

pub mod codecs;
pub mod feed;
pub mod parser;
pub mod schema;
