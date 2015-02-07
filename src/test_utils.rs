#![macro_use]
#![doc(hidden)]

use std::old_io::TempDir;

pub fn temp_dir() -> TempDir {
    TempDir::new("rust-earth-test").unwrap()
}


#[cfg(test)]
#[macro_use]
pub mod macros {
    #[macro_export]
    macro_rules! unwrap {
        ($expr:expr) => (match $expr {
            Ok(t) => t,
            Err(e) =>
                panic!("called `unwrap!()` on an `Err` value: {:?}", e)
        })
    }

    #[macro_export]
    macro_rules! assert_err {
        ($expr:expr, $err_pat:pat => $blk:block) => (match $expr {
            Ok(_) => { panic!("unexpected success"); }
            Err($err_pat) => $blk,
            Err(e) => { panic!("unexpected error: {:?}", e); }
        })
    }

    #[macro_export]
    macro_rules! expect_invalid_key {
        ($($f:ident).+, $key:expr) => ({
            let key: &[&[u8]] = $key;
            assert_err!($($f).+(key), RepositoryError::InvalidKey(k, _) => {
                assert_eq!(k, key);
            })
        })
    }

    #[macro_export]
    macro_rules! assert_html {
        ($value:expr, $expected:expr) => (
            assert_eq!($value.to_html().to_string(), $expected)
        )
    }
}
