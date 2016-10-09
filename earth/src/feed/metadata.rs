use std::io;

use chrono::{DateTime, FixedOffset};

use parser::base::{DecodeResult, XmlElement, XmlName};
use schema::{FromSchemaReader};
use util::set_default;

use super::{ATOM_XMLNS, Category, Link, Person, Text, parse_datetime};


/// Common metadata shared by `Source`, `Entry`, and `Feed`.
pub trait Metadata {
    /// The URI that conveys a permanent, universally unique identifier for an
    /// entry or feed.  It corresponds to `atom:id` element of :rfc:`4287#section-4.2.6` (section 4.2.6).
    fn id(&self) -> &str;
    fn id_mut(&mut self) -> &mut String;

    /// The human-readable title for an entry or feed.
    /// It corresponds to `atom:title` element of :rfc:`4287#section-4.2.14` (section 4.2.14).
    fn title(&self) -> &Text;
    fn title_mut(&mut self) -> &mut Text;

    /// The list of :class:`Link` objects that define a reference from an entry
    /// or feed to a web resource.  It corresponds to `atom:link` element of
    /// :rfc:`4287#section-4.2.7` (section 4.2.7).
    fn links(&self) -> &[Link];
    fn links_mut(&mut self) -> &mut Vec<Link>;

    /// The datetime value with a fixed timezone offset, indicating the most
    /// recent instant in time when the entry was modified in a way the
    /// publisher considers significant.  Therefore, not all modifications
    /// necessarily result in a changed `updated_at` value.
    /// It corresponds to `atom:updated` element of :rfc:`4287#section-4.2.15` (section 4.2.15).
    fn updated_at(&self) -> &DateTime<FixedOffset>;
    fn updated_at_mut(&mut self) -> &mut DateTime<FixedOffset>;

    /// The list of `Person` values which indicates the author of the entry or
    /// feed.  It corresponds to `atom:author` element of :rfc:`4287#section-4.2.1` (section 4.2.1).
    fn authors(&self) -> &[Person];
    fn authors_mut(&mut self) -> &mut Vec<Person>;

    /// The list of `Person` values which indicates a person or other entity
    /// who contributed to the entry or feed.  It corresponds to
    /// `atom:contributor` element of :rfc:`4287#section-4.2.3` (section 4.2.3).
    fn contributors(&self) -> &[Person];
    fn contributors_mut(&mut self) -> &mut Vec<Person>;

    /// The list of `Category` values that conveys information about categories
    /// associated with an entry or feed.  It corresponds to `atom:category`
    /// element of :rfc:`4287#section-4.2.2` (section 4.2.2).
    fn categories(&self) -> &[Category];
    fn categories_mut(&mut self) -> &mut Vec<Category>;

    /// The text field that conveys information about rights held in and of an
    /// entry or feed.  It corresponds to `atom:rights` element of
    /// :rfc:`4287#section-4.2.10` (section 4.2.10).
    fn rights(&self) -> Option<&Text>;
    fn rights_mut(&mut self) -> &mut Option<Text>;
}

pub fn match_metadata_child<M, B>(m: &mut M, name: &XmlName, child: XmlElement<B>) -> DecodeResult<()>
    where M: Metadata, B: io::BufRead
{
    match (name.namespace_ref(), &name.local_name[..]) {
        (Some(ATOM_XMLNS), "id") => {
            *m.id_mut() = try!(child.read_whole_text());
        }
        (Some(ATOM_XMLNS), "title") => {
            try!(m.title_mut().read_from(child));
        }
        (Some(ATOM_XMLNS), "link") => {
            m.links_mut().push(try!(FromSchemaReader::build_from(child)));
        }
        (Some(ATOM_XMLNS), "updated") => {
            *m.updated_at_mut() = try!(parse_datetime(child));
        }
        (Some(ATOM_XMLNS), "modified") => {
            *m.updated_at_mut() = try!(parse_datetime(child));
        }
        (Some(ATOM_XMLNS), "author") => {
            if let Some(p) = try!(FromSchemaReader::build_from(child)) {
                m.authors_mut().push(p);
            }
        }
        (Some(ATOM_XMLNS), "contributor") => {
            if let Some(p) = try!(FromSchemaReader::build_from(child)) {
                m.contributors_mut().push(p);
            }
        }
        (Some(ATOM_XMLNS), "category") => {
            m.categories_mut().push(try!(FromSchemaReader::build_from(child)));
        }
        (Some(ATOM_XMLNS), "rights") => {
            *set_default(m.rights_mut()) = try!(FromSchemaReader::build_from(child));
        }
        _ => { }
    }
    Ok(())
}
