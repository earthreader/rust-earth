use std::borrow::{Cow, ToOwned};
use std::collections::hash_map::{Entry, HashMap};
use std::default::Default;
use std::error::Error;
use std::fmt;
use std::hash::Hash;
use std::io;
use std::mem;

use chrono::{DateTime, TimeZone};

use parser::base::{DecodeResult, XmlElement, XmlName};
use parser::base::NestedEvent::Nested;

pub type SchemaResult<T> = Result<T, SchemaError>;

#[derive(Debug)]
pub enum SchemaError {
//    DescriptorConflict,
//    IntegrityError,
    EncodeError,
    DecodeError(&'static str, Option<String>),
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for SchemaError {
    fn description(&self) -> &str {
        match *self {
            SchemaError::EncodeError => "Error occured while encoding",
            SchemaError::DecodeError(ref msg, _) => &msg,
        }
    }
}

pub trait Codec<T> {
    fn encode(&self, value: &T, w: &mut io::Write) -> SchemaResult<()>;
    fn decode(&self, r: &str) -> SchemaResult<T>;
}

pub trait Entity {
    type Id: ?Sized + ToOwned;
    fn entity_id(&self) -> Cow<<Self as Entity>::Id>;
}

/// This trait is used to merge conflicts between concurrent updates.
///
/// ## Note
///
/// The default implementation does nothing.  That means the entity of the
/// newer session will always win unless the method is redefined.
pub trait Mergeable: Sized {
    /// Merge data with the given value to renew itself as a latest state.
    ///
    /// The given argument `other` is guaranteed that it's from the older
    /// session. (note that it doesn't mean this entity is older than `self`,
    /// but the last update of the session is)
    #[inline(always)]
    fn merge_with(&mut self, other: Self) { let _ = other; /* noop */ }
}

impl Mergeable for String { }
impl<Off: TimeZone> Mergeable for DateTime<Off> { }

impl<T: Mergeable> Mergeable for Option<T> {
    fn merge_with(&mut self, other: Option<T>) {
        let this = self.take();
        *self = match (this, other) {
            (Some(mut a), Some(b)) => {
                a.merge_with(b);
                Some(a)
            }
            (Some(a), None   ) => Some(a),
            (None   , Some(b)) => Some(b),
            (None   , None   ) => None
        }
    }
}

impl<T: Entity + Mergeable> Mergeable for Vec<T>
    where <<T as Entity>::Id as ToOwned>::Owned: Hash + Eq
{
    fn merge_with(&mut self, other: Vec<T>) {
        let mut place = vec![];
        mem::swap(self, &mut place);
        let mut identifiers: HashMap<_, T> =
            place.into_iter().map(|e| (e.entity_id().into_owned(), e)).collect();
        for element in other {
            let eid = element.entity_id().into_owned();
            match identifiers.entry(eid) {
                Entry::Occupied(e) => {
                    <T as Mergeable>::merge_with(e.into_mut(), element);
                }
                Entry::Vacant(e) => {
                    e.insert(element);
                }
            }
        }
        self.extend(identifiers.into_iter().map(|(_, v)| v));
    }
}

/// The root element of the document.
pub trait DocumentElement {
    fn tag() -> &'static str;
    fn xmlns() -> Option<&'static str>;
}

pub trait FromSchemaReader: Default + Sized {
    fn build_from<B: io::BufRead>(element: XmlElement<B>) -> DecodeResult<Self> {
        let mut result: Self = Default::default();
        try!(result.read_from(element));
        Ok(result)
    }

    fn read_from<B: io::BufRead>(&mut self, mut element: XmlElement<B>)
                                 -> DecodeResult<()>
    {
        loop {
            match element.children.next() {
                Some(Ok(Nested { name, element })) => {
                    try!(self.match_child(&name, element));
                }
                Some(Err(e)) => {
                    return Err(e);
                }
                None => { break }
                _ => { }
            }
        }
        Ok(())
    }
    
    fn match_child<B: io::BufRead>(&mut self, _name: &XmlName,
                                   _child: XmlElement<B>) -> DecodeResult<()>
    { Ok(()) }
}
