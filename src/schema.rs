#![unstable]

use std::borrow::{BorrowFrom, Cow, ToOwned};
use std::collections::hash_map::{Entry, Hasher, HashMap};
use std::default::Default;
use std::error::Error;
use std::fmt;
use std::hash::Hash;

use chrono::DateTime;

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

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for SchemaError {
    fn description(&self) -> &str {
        match *self {
            SchemaError::EncodeError => "Error occured while encoding",
            SchemaError::DecodeError(ref msg, _) => &msg[],
        }
    }
}

pub trait Codec<T> {
    fn encode(&self, value: &T, w: &mut Writer) -> SchemaResult<()>;
    fn decode(&self, r: &str) -> SchemaResult<T>;
}

#[experimental]
pub trait Entity {
    type OwnedId;
    type BorrowedId: ?Sized;
    fn entity_id(&self) -> Cow<<Self as Entity>::OwnedId,
                               <Self as Entity>::BorrowedId>;
}

/// This trait is used to merge conflicts between concurrent updates.
///
/// ## Note
///
/// The default implementation does nothing.  That means the entity of the
/// newer session will always win unless the method is redefined.
#[experimental]
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
impl<Off> Mergeable for DateTime<Off> { }

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
    where <T as Entity>::OwnedId: Hash<Hasher> + Eq,
          <T as Entity>::BorrowedId: Hash<Hasher> + Eq +
                                     BorrowFrom<<T as Entity>::OwnedId> +
                                     ToOwned<<T as Entity>::OwnedId>
{
    fn merge_with(&mut self, other: Vec<T>) {
        let mut identifiers: HashMap<<T as Entity>::OwnedId, T> =
            self.drain().map(|e| (e.entity_id().into_owned(), e)).collect();
        for element in other.into_iter() {
            let eid = element.entity_id().into_owned();
            match identifiers.entry(eid) {
                Entry::Occupied(mut e) => {
                    e.get_mut().merge_with(element);
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
