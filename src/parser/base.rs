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
