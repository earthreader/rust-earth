#![unstable]

use std::borrow::{Cow, ToOwned};
use std::fmt;

use parser::base::{DecodeResult, XmlElement};
use schema::{Entity, FromSchemaReader, Mergeable};

/// Category element defined in :rfc:`4287#section-4.2.2` (section 4.2.2).
#[derive(Default, Show)]
pub struct Category {
    /// The required machine-readable identifier string of the cateogry.
    /// It corresponds to ``term`` attribute of :rfc:`4287#section-4.2.2.1` (section 4.2.2.1).
    pub term: String,

    /// The URI that identifies a categorization scheme.  It corresponds to
    /// ``scheme`` attribute of :rfc:`4287#section-4.2.2.2` (section 4.2.2.2).
    ///
    /// ### See also
    ///
    /// * [Tag Scheme?][scheme-1] by Tim Bray
    /// * [Representing tags in Atom][scheme-2] by Edward O'Connor
    ///
    /// [scheme-1]: http://www.tbray.org/ongoing/When/200x/2007/02/01/Tag-Scheme
    /// [scheme-2]: http://edward.oconnor.cx/2007/02/representing-tags-in-atom
    pub scheme_uri: Option<String>,

    /// The optional human-readable label for display in end-user
    /// applications.  It corresponds to ``label`` attribute of :rfc:`4287#section-4.2.2.3` (section 4.2.2.3).
    pub label: Option<String>,
}

impl Entity for Category {
    type OwnedId = String;
    type BorrowedId = str;
    fn entity_id(&self) -> Cow<String, str> {
        Cow::Borrowed(&self.term[])
    }
}

impl Mergeable for Category {
    fn merge_with(&mut self, other: Category) {
        if self.label.is_none() {
            self.label = other.label
        }
    }
}

impl fmt::String for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.label.as_ref().unwrap_or(&self.term))
    }
}

impl FromSchemaReader for Category {
    fn read_from<B: Buffer>(&mut self, element: XmlElement<B>)
                            -> DecodeResult<()>
    {
        self.term = try!(element.get_attr("term")).to_owned();
        self.scheme_uri = element.get_attr("scheme").ok()
                                 .map(|v| v.to_string());
        self.label = element.get_attr("label").ok().map(|v| v.to_string());
        Ok(())
    }
}


#[cfg(test)]
mod test {
    use super::Category;

    use std::default::Default;

    #[test]
    fn test_category_str() {
        assert_eq!(Category { term: "rust".to_string(),
                              ..Default::default() }.to_string(),
                   "rust");
        assert_eq!(Category { term: "rust".to_string(),
                              label: Some("Rust".to_string()),
                              ..Default::default() }.to_string(),
                   "Rust");
    }
}
