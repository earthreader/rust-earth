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
