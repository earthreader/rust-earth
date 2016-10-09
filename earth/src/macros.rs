#![macro_use]

macro_rules! impl_mergeable {
    ($target:ident, $($field:ident),+) => {
        impl Mergeable for $target {
            fn merge_with(&mut self, other: $target) {
                let $target { $($field,)+ .. } = other;
                $( self.$field.merge_with($field); )+
            }
        }
    }
}

macro_rules! impl_metadata {
    ($target:ident) => {
        impl $crate::feed::Metadata for $target {
            fn id(&self) -> &str { &self.id[..] }
            fn title(&self) -> &Text { &self.title }
            fn links(&self) -> &[Link] { &self.links[..] }
            fn updated_at(&self) -> &DateTime<FixedOffset> { &self.updated_at }
            fn authors(&self) -> &[Person] { &self.authors[..] }
            fn contributors(&self) -> &[Person] { &self.contributors[..] }
            fn categories(&self) -> &[Category] { &self.categories[..] }
            fn rights(&self) -> Option<&Text> { self.rights.as_ref() }

            fn id_mut(&mut self) -> &mut String { &mut self.id }
            fn title_mut(&mut self) -> &mut Text { &mut self.title }
            fn links_mut(&mut self) -> &mut Vec<Link> { &mut self.links }
            fn updated_at_mut(&mut self) -> &mut DateTime<FixedOffset> { &mut self.updated_at }
            fn authors_mut(&mut self) -> &mut Vec<Person> { &mut self.authors }
            fn contributors_mut(&mut self) -> &mut Vec<Person> { &mut self.contributors }
            fn categories_mut(&mut self) -> &mut Vec<Category> { &mut self.categories }
            fn rights_mut(&mut self) -> &mut Option<Text> { &mut self.rights }
        }
    }
}
