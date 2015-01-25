#![macro_use]
#![unstable]

macro_rules! for_each {
    ($e:ident in $it:expr { $($body:stmt)* }) => (
        loop {
            match $it {
                Some($e) => { $($body)* },
                None => break,
            }
        }
    )
}

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
