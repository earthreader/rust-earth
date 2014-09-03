use xml;
use xml::reader::events::{XmlEvent, EndDocument, StartElement, EndElement, Error};

use schema;

pub struct XmlDecoder<B> {
    reader: xml::EventReader<B>,
    peeked: Option<(XmlEvent, uint)>,
    depth: uint,
}

pub enum DecodeError {
    XmlError(xml::common::Error),
    UnexpectedEvent { event: XmlEvent, depth: uint },
    NoResult,
    AttributeNotFound(String),
    SchemaError(schema::SchemaError),
}

pub type DecodeResult<T> = Result<T, DecodeError>;

impl<B: Buffer> XmlDecoder<B> {
    pub fn new(reader: xml::EventReader<B>) -> XmlDecoder<B> {
        XmlDecoder { reader: reader, peeked: None, depth: 0 }
    }

    fn _next(&mut self) -> (XmlEvent, uint) {
        let v = self.reader.next();
        let depth = match &v {
            &StartElement { .. } => { self.depth += 1; self.depth }
            &EndElement { .. } => { let d = self.depth; self.depth -= 1; d }
            _ => { self.depth }
        };
        (v, depth)
    }

    #[inline]
    fn next(&mut self) -> (XmlEvent, uint) {
        if self.peeked.is_some() { self.peeked.take().unwrap() }
        else { self._next() }
    }

    #[inline]
    fn peek<'a>(&'a mut self) -> &'a (XmlEvent, uint) {
        if self.peeked.is_none() {
            self.peeked = Some(self._next());
        }
        self.peeked.get_ref()
    }

    fn drain_children(&mut self, depth: uint) -> DecodeResult<(XmlEvent, uint)> {
        loop {
            match self.next() {
                (Error(e), _) => { return Err(XmlError(e)); }
                (_, d) if d > depth => { continue; }
                (evt @ EndElement { .. }, d) => { return Ok((evt, d)); }
                (evt, d) => { return Err(UnexpectedEvent { event: evt, depth: d }); }
            }
        }
    }

    pub fn read_event<T>(&mut self, f: |&mut XmlDecoder<B>, XmlEvent| -> DecodeResult<T>) -> DecodeResult<T> {
        match self.next() {
            (evt @ StartElement { .. }, depth) => {
                let result = f(self, evt);
                try!(self.drain_children(depth));
                result
            }
            (evt @ EndElement { .. }, depth) => {
                Err(UnexpectedEvent { event: evt, depth: depth })
            }
            (evt, _) => { f(self, evt) }
        }
    }

    pub fn each_child(&mut self, f: |&mut XmlDecoder<B>| -> DecodeResult<()>) -> DecodeResult<()> {
        loop {
            match *self.peek().ref0() {
                Error(ref e) => { return Err(XmlError(e.clone())); }
                EndDocument => { return Ok(()); }
                EndElement { .. } => { return Ok(()); }
                _ => { }
            }
            match f(self) {
                Ok(()) => { }
                Err(e) => { return Err(e); }
            }
        }
    }
}

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
            use xml::reader::events as x;
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

                x::EndElement { name } => {
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
                EndDocument | EndElement { .. } | Error(_) => break,
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
