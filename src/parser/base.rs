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


pub struct NestedEventReader<'a, B: 'a> {
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
    use xml::reader::events as x;

    use super::NestedEventReader;

    pub enum NestedEvent<'a, B: 'a> {
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

    impl<'a, B> PartialEq for NestedEvent<'a, B> {
        fn eq(&self, other: &NestedEvent<'a, B>) -> bool {
            match (self, other) {
                (&StartDocument { version: ref v1, encoding: ref e1, standalone: ref s1 },
                 &StartDocument { version: ref v2, encoding: ref e2, standalone: ref s2 }) => {
                    v1 == v2 && e1 == e2 && s1 == s2
                }
                (&EndDocument, &EndDocument) => { true }
                (&ProcessingInstruction { name: ref n1, data: ref d1 },
                 &ProcessingInstruction { name: ref n2, data: ref d2 }) => {
                    n1 == n2 && d1 == d2
                }
                (&Element { name: ref n1, attributes: ref a1, namespace: ref ns1, .. },
                 &Element { name: ref n2, attributes: ref a2, namespace: ref ns2, .. }) => {
                    n1 == n2 && a1 == a2 && ns1 == ns2
                }
                (&CData(ref c1), &CData(ref c2)) => { c1 == c2 }
                (&Comment(ref c1), &Comment(ref c2)) => { c1 == c2 }
                (&Characters(ref c1), &Characters(ref c2)) => { c1 == c2 }
                (&Whitespace(ref c1), &Whitespace(ref c2)) => { c1 == c2 }
                (&Error(ref e1), &Error(ref e2)) => { e1 == e2 }
                (_, _) => { false }
            }
        }
    }

    impl<'a, B> Equiv<x::XmlEvent> for NestedEvent<'a, B> {
        fn equiv(&self, other: &x::XmlEvent) -> bool {
            match (self, other) {
                (&   StartDocument { version: ref v1, encoding: ref e1, standalone: ref s1 },
                 &x::StartDocument { version: ref v2, encoding: ref e2, standalone: ref s2 }) => {
                    v1 == v2 && e1 == e2 && s1 == s2
                }
                (&EndDocument, &x::EndDocument) => { true }
                (&   ProcessingInstruction { name: ref n1, data: ref d1 },
                 &x::ProcessingInstruction { name: ref n2, data: ref d2 }) => {
                    n1 == n2 && d1 == d2
                }
                (&        Element { name: ref n1, attributes: ref a1, namespace: ref ns1, .. },
                 &x::StartElement { name: ref n2, attributes: ref a2, namespace: ref ns2 }) => {
                    n1 == n2 && a1 == a2 && ns1 == ns2
                }
                (&CData(ref c1), &x::CData(ref c2)) => { c1 == c2 }
                (&Comment(ref c1), &x::Comment(ref c2)) => { c1 == c2 }
                (&Characters(ref c1), &x::Characters(ref c2)) => { c1 == c2 }
                (&Whitespace(ref c1), &x::Whitespace(ref c2)) => { c1 == c2 }
                (&Error(ref e1), &x::Error(ref e2)) => { e1 == e2 }
                (_, _) => { false }
            }
        }
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
