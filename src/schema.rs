#![unstable]

use std::default::Default;
use std::error::Error;

use parser::base::{DecodeResult, XmlElement, XmlName};
use parser::base::NestedEvent::Nested;

pub type SchemaResult<T> = Result<T, SchemaError>;

#[deprecated]
#[derive(Show)]
pub enum SchemaError {
//    DescriptorConflict,
//    IntegrityError,
    EncodeError,
    DecodeError(&'static str, Option<String>),
}

impl Error for SchemaError {
    fn description(&self) -> &str {
        match *self {
            SchemaError::EncodeError => "Error occured while encoding",
            SchemaError::DecodeError(ref msg, _) => &msg[],
        }
    }

    fn detail(&self) -> Option<String> {
        match *self {
            SchemaError::EncodeError => None,
            SchemaError::DecodeError(_, ref detail) => detail.clone(),
        }
    }
}

pub trait Codec<T> {
    fn encode(&self, value: &T, w: &mut Writer) -> SchemaResult<()>;
    fn decode(&self, r: &str) -> SchemaResult<T>;
}

#[experimental]
pub trait Mergeable {
    fn merge_entities(self, other: Self) -> Self;
}

/// The root element of the document.
#[experimental]
pub trait DocumentElement {
    fn tag() -> &'static str;
    fn xmlns() -> Option<&'static str>;
}

#[experimental]
pub trait FromSchemaReader: Default + Sized {
    fn build_from<B: Buffer>(element: XmlElement<B>) -> DecodeResult<Self> {
        let mut result: Self = Default::default();
        try!(result.read_from(element));
        Ok(result)
    }

    fn read_from<B: Buffer>(&mut self, mut element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        loop {
            match element.children.next() {
                Some(Nested { name, element }) => {
                    try!(self.match_child(&name, element));
                }
                None => { break }
                _ => { }
            }
        }
        Ok(())
    }
    
    fn match_child<B: Buffer>(&mut self, _name: &XmlName,
                              _child: XmlElement<B>) -> DecodeResult<()>
    { Ok(()) }
}
