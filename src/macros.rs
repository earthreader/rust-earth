#![macro_escape]

macro_rules! for_each(
    ($e:ident in $it:expr $body:expr) => (
        loop {
            match $it {
                Some($e) => $body,
                None => break,
            }
        }
    )
)
