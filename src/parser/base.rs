use xml;
use xml::reader::events as x;

use schema;

pub enum DecodeError {
    XmlError(xml::common::Error),
    UnexpectedEvent { event: x::XmlEvent, depth: uint },
    NoResult,
    AttributeNotFound(String),
    SchemaError(schema::SchemaError),
}

pub type DecodeResult<T> = Result<T, DecodeError>;


pub struct NestedEventReader<'a, B> {
    reader: &'a mut xml::EventReader<B>,
    finished: bool,
}

impl<'a, B: Buffer> NestedEventReader<'a, B> {
    pub fn new(reader: &'a mut xml::EventReader<B>) -> NestedEventReader<'a, B> {
        NestedEventReader { reader: reader, finished: false }
    }

    #[inline]
    pub fn next<'b>(&'b mut self) -> Option<events::NestedEvent<'b, B>> {
        if self.finished {
            None
        } else {
            use self::events as n;
            let ev = match self.reader.next() {
                x::StartDocument { version, encoding, standalone } =>
                n::StartDocument { version: version, encoding: encoding, standalone: standalone },

                x::EndDocument => {
                    self.finished = true;
                    n::EndDocument
                }

                x::ProcessingInstruction { name, data } =>
                n::ProcessingInstruction { name: name, data: data },

                x::StartElement { name, attributes, namespace } => {
                    n::Element {
                        name: name,
                        attributes: attributes,
                        namespace: namespace,
                        children: NestedEventReader::new(self.reader)
                    }
                }

                x::EndElement { .. } => {
                    self.finished = true;
                    return None;
                }

                x::CData(c) => n::CData(c),
                x::Comment(c) => n::Comment(c),
                x::Characters(c) => n::Characters(c),
                x::Whitespace(c) => n::Whitespace(c),

                x::Error(e) => {
                    self.finished = true;
                    n::Error(e)
                }
            };
            Some(ev)
        }
    }
}

#[unsafe_destructor]
impl<'a, B: Buffer + Send> Drop for NestedEventReader<'a, B> {
    #[inline]
    fn drop(&mut self) {
        // drain all remained events
        loop {
            match self.reader.next() {
                x::EndDocument | x::EndElement { .. } | x::Error(_) => break,
                _ => { }
            }
        }
    }
}

pub mod events {
    use std::fmt;

    use xml::common;
    use xml::common::{Name, Error, HasPosition, Attribute, XmlVersion};
    use xml::namespace::Namespace;

    use super::NestedEventReader;

    pub enum NestedEvent<'a, B> {
        StartDocument {
            pub version: XmlVersion,
            pub encoding: String,
            pub standalone: Option<bool>
        },
        EndDocument,
        ProcessingInstruction { 
            pub name: String, 
            pub data: Option<String> 
        },
        Element { 
            pub name: Name,
            pub attributes: Vec<Attribute>,
            pub namespace: Namespace,
            pub children: NestedEventReader<'a, B>,
        },
        CData(String),
        Comment(String),
        Characters(String),
        Whitespace(String),
        Error(common::Error)
    }

    impl<'a, B> fmt::Show for NestedEvent<'a, B> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                StartDocument { ref version, ref encoding, ref standalone } =>
                    write!(f, "StartDocument({}, {}, {})", version, *encoding, *standalone),
                EndDocument =>
                    write!(f, "EndDocument"),
                ProcessingInstruction { ref name, ref data } =>
                    write!(f, "ProcessingInstruction({}{})", *name, match *data {
                        Some(ref data) => format!(", {}", data),
                        None       => String::new()
                    }),
                Element { ref name, ref attributes, namespace: Namespace(ref namespace), .. } =>
                    write!(f, "Element({}, {}{})", name, namespace, if attributes.is_empty() {
                        String::new()
                    } else {
                        let attributes: Vec<String> = attributes.iter().map(
                            |a| format!("{} -> {}", a.name, a.value)
                                ).collect();
                        format!(", [{}]", attributes.connect(", "))
                    }),
                Comment(ref data) =>
                    write!(f, "Comment({})", data),
                CData(ref data) =>
                    write!(f, "CData({})", data),
                Characters(ref data) =>
                    write!(f, "Characters({})", data),
                Whitespace(ref data) =>
                    write!(f, "Whitespace({})", data),
                Error(ref e) =>
                    write!(f, "Error(row: {}, col: {}, message: {})", e.row()+1, e.col()+1, e.msg())
            }
        }
    }
}
