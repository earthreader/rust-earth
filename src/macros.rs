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
