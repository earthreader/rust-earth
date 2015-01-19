use xml;
use xml::reader::events::XmlEvent as x;

use schema;

pub use self::events::NestedEvent;

pub enum DecodeError {
    XmlError(xml::common::Error),
    UnexpectedEvent { event: xml::reader::events::XmlEvent, depth: usize },
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
    pub fn next(&mut self) -> Option<events::NestedEvent<B>> {
        if self.finished {
            None
        } else {
            use self::NestedEvent as n;
            let ev = match self.reader.next() {
                x::StartDocument { version, encoding, standalone } =>
                n::StartDocument { version: version,
                                   encoding: encoding,
                                   standalone: standalone },

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
impl<'a, B: Buffer + 'a> Drop for NestedEventReader<'a, B> {
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

    use xml::attribute::OwnedAttribute;
    use xml::common;
    use xml::common::{HasPosition, XmlVersion};
    use xml::name::OwnedName;
    use xml::namespace::Namespace;
    use xml::reader::events::XmlEvent as x;
    use self::NestedEvent as n;

    use super::NestedEventReader;

    pub enum NestedEvent<'a, B: 'a> {
        StartDocument {
            version: XmlVersion,
            encoding: String,
            standalone: Option<bool>
        },
        EndDocument,
        ProcessingInstruction { 
            name: String, 
            data: Option<String> 
        },
        Element { 
            name: OwnedName,
            attributes: Vec<OwnedAttribute>,
            namespace: Namespace,
            children: NestedEventReader<'a, B>,
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
                (&n::StartDocument { version: ref v1,
                                     encoding: ref e1,
                                     standalone: ref s1 },
                 &n::StartDocument { version: ref v2,
                                     encoding: ref e2,
                                     standalone: ref s2 }) => {
                    v1 == v2 && e1 == e2 && s1 == s2
                }
                (&n::EndDocument, &n::EndDocument) => { true }
                (&n::ProcessingInstruction { name: ref n1, data: ref d1 },
                 &n::ProcessingInstruction { name: ref n2, data: ref d2 }) => {
                    n1 == n2 && d1 == d2
                }
                (&n::Element { name: ref n1,
                               attributes: ref a1,
                               namespace: ref ns1, .. },
                 &n::Element { name: ref n2,
                               attributes: ref a2,
                               namespace: ref ns2, .. }) => {
                    n1 == n2 && a1 == a2 && ns1 == ns2
                }
                (&n::CData(ref c1),      &n::CData(ref c2)     ) => { c1 == c2 }
                (&n::Comment(ref c1),    &n::Comment(ref c2)   ) => { c1 == c2 }
                (&n::Characters(ref c1), &n::Characters(ref c2)) => { c1 == c2 }
                (&n::Whitespace(ref c1), &n::Whitespace(ref c2)) => { c1 == c2 }
                (&n::Error(ref e1),      &n::Error(ref e2)     ) => { e1 == e2 }
                (_, _) => { false }
            }
        }
    }

    impl<'a, B> PartialEq<x> for NestedEvent<'a, B> {
        fn eq(&self, other: &x) -> bool {
            match (self, other) {
                (&n::StartDocument { version: ref v1,
                                     encoding: ref e1,
                                     standalone: ref s1 },
                 &x::StartDocument { version: ref v2,
                                     encoding: ref e2,
                                     standalone: ref s2 }) => {
                    v1 == v2 && e1 == e2 && s1 == s2
                }
                (&n::EndDocument, &x::EndDocument) => { true }
                (&n::ProcessingInstruction { name: ref n1, data: ref d1 },
                 &x::ProcessingInstruction { name: ref n2, data: ref d2 }) => {
                    n1 == n2 && d1 == d2
                }
                (&n::     Element { name: ref n1,
                                    attributes: ref a1,
                                    namespace: ref ns1, .. },
                 &x::StartElement { name: ref n2,
                                    attributes: ref a2,
                                    namespace: ref ns2 }) => {
                    n1 == n2 && a1 == a2 && ns1 == ns2
                }
                (&n::CData(ref c1),      &x::CData(ref c2)     ) => { c1 == c2 }
                (&n::Comment(ref c1),    &x::Comment(ref c2)   ) => { c1 == c2 }
                (&n::Characters(ref c1), &x::Characters(ref c2)) => { c1 == c2 }
                (&n::Whitespace(ref c1), &x::Whitespace(ref c2)) => { c1 == c2 }
                (&n::Error(ref e1),      &x::Error(ref e2)     ) => { e1 == e2 }
                (_, _) => { false }
            }
        }
    }

    impl<'a, B> PartialEq<NestedEvent<'a, B>> for x {
        fn eq(&self, other: &NestedEvent<'a, B>) -> bool { other == self }
    }

    impl<'a, B> fmt::Show for NestedEvent<'a, B> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                n::StartDocument { ref version, ref encoding, ref standalone } =>
                    write!(f, "StartDocument({}, {:?}, {:?})",
                           version, *encoding, *standalone),
                n::EndDocument =>
                    write!(f, "EndDocument"),
                n::ProcessingInstruction { ref name, ref data } => {
                    try!(write!(f, "ProcessingInstruction({:?}", *name));
                    if let Some(ref data) = *data {
                        try!(write!(f, ", {:?}", data));
                    }
                    write!(f, ")")
                }
                n::Element { ref name, ref attributes,
                             namespace: Namespace(ref namespace), .. } => {
                    try!(write!(f, "Element({:?}, {:?}", name, namespace));
                    if !attributes.is_empty() {
                        try!(write!(f, ", ["));
                        let mut first = true;
                        for attr in attributes.iter() {
                            if first {
                                first = false;
                            } else {
                                try!(write!(f, ", "));
                            }
                            try!(write!(f, "{:?} -> {:?}",
                                        attr.name, attr.value));
                        }
                    }
                    write!(f, "])")
                }
                n::Comment(ref data) =>
                    write!(f, "Comment({:?})", data),
                n::CData(ref data) =>
                    write!(f, "CData({:?})", data),
                n::Characters(ref data) =>
                    write!(f, "Characters({:?})", data),
                n::Whitespace(ref data) =>
                    write!(f, "Whitespace({:?})", data),
                n::Error(ref e) =>
                    write!(f, "Error(row: {}, col: {}, message: {:?})",
                           e.row() + 1, e.col() + 1, e.msg())
            }
        }
    }
}
