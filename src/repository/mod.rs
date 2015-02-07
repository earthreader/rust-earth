#![unstable]
//! Abstracts storage backend e.g. filesystem.
//!
//! There might be platforms that have no chance to directly access file system
//! e.g. iOS, and in that case the concept of repository makes you to store
//! data directly to [Dropbox][] or [Google Drive][] instead of filesystem.
//! However in the most cases we will simply use `FileSystemRepository` even if
//! data are synchronized using Dropbox or `rsync`.
//!
//! [Dropbox]: http://dropbox.com/
//! [Google Drive]: https://drive.google.com/
use std::error::{Error, FromError};
use std::fmt;
use std::old_io::IoError;

pub use self::utils::{Bytes, Names};
pub use self::fs::FileSystemRepository;

pub mod fs;

pub type RepositoryResult<T> = Result<T, RepositoryError>;

#[derive(Debug)]
pub enum RepositoryError {
    InvalidKey(Vec<String>, Option<IoError>),
    InvalidUrl(&'static str),
    NotADirectory(Path),
    CannotBorrow,
    Io(IoError),
}

impl RepositoryError {
    #[inline]
    pub fn invalid_key<T: Str>(key: &[T], cause: Option<IoError>) ->
        RepositoryError
    {
        let copied_key = key.iter()
            .map(|e| e.as_slice().to_string())
            .collect();
        RepositoryError::InvalidKey(copied_key, cause)
    }

    #[inline]
    pub fn invalid_url(detail: &'static str) -> RepositoryError {
        RepositoryError::InvalidUrl(detail)
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "{}", self.description()));
        match *self {
            RepositoryError::InvalidKey(ref key, _) => {
                try!(write!(f, ": ["));
                let mut first = true;
                for i in key.iter() {
                    if first { first = false; } else { try!(write!(f, ", ")); }
                    try!(write!(f, "{:?}", i));
                }
                try!(write!(f, "]"));
            }
            RepositoryError::InvalidUrl(ref msg) => {
                try!(write!(f, ": {}", msg));
            }
            _ => { }
        }
        if let Some(cause) = self.cause() {
            try!(write!(f, " caused by `{}`", cause));
        }
        Ok(())
    }
}

impl Error for RepositoryError {
    fn description(&self) -> &str {
        match *self {
            RepositoryError::InvalidKey(_, _) => "invalid key",
            RepositoryError::InvalidUrl(_) => "invalid URL",
            RepositoryError::NotADirectory(_) => "not a directory",
            RepositoryError::CannotBorrow => "can't borrow",
            RepositoryError::Io(_) => "IO error"
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            RepositoryError::InvalidKey(_, Some(ref err)) => Some(err as &Error),
            RepositoryError::Io(ref err) => Some(err as &Error),
            _ => None
        }
    }
}

impl FromError<IoError> for RepositoryError {
    fn from_error(err: IoError) -> RepositoryError {
        RepositoryError::Io(err)
    }
}

/// Repository interface agnostic to its underlying storage implementation.
/// Stage objects can deal with documents to be stored using the interface.
///
/// Every content in repositories is accessible using *keys*.  It actually
/// abstracts out "filenames" in "file systems", hence keys share the common
/// concepts with filenames.  Keys are hierarchical, like file paths, so
/// consists of multiple sequential strings e.g. `['dir', 'subdir', 'key']`.
/// You can `list()` all subkeys in the upper key as well e.g.:
///
/// ```
/// # use earth::test_utils::temp_dir;
/// # use earth::repository::{FileSystemRepository, Repository};
/// # let tmpdir = temp_dir();
/// # let repository = FileSystemRepository::from_path(tmpdir.path(), true).unwrap();
/// repository.list(&["dir", "subdir"])
/// # ;
/// ```
pub trait Repository {
    /// Read the content from the `key`.
    fn get_reader<'a, T: Str>(&'a self, key: &[T]) ->
        RepositoryResult<Box<Buffer + 'a>>;

    /// Get a writer to write data into the ``key``.
    fn get_writer<'a, T: Str>(&'a mut self, key: &[T]) ->
        RepositoryResult<Box<Writer + 'a>>;

    fn read<T: Str>(&self, key: &[T]) -> RepositoryResult<Vec<u8>> {
        Ok(try!(try!(self.get_reader(key)).read_to_end()))
    }

    fn write<T: Str, U: Bytes>(&mut self, key: &[T], buf: &[U]) ->
        RepositoryResult<()>
    {
        let mut w = try!(self.get_writer(key));
        for b in buf.iter() {
            try!(w.write_all(b.as_bytes()));
        }
        Ok(())
    }

    /// Return whether the `key` exists or not.
    fn exists<T: Str>(&self, key: &[T]) -> bool;

    /// List all subkeys in the `key`.
    fn list<'a, T: Str>(&'a self, key: &[T]) -> RepositoryResult<Names<'a>>;
}

pub trait ToRepository<R: Repository> {
    /// Create a new instance of the repository from itself.
    /// It may be used for configuring the repository in plain text
    /// e.g. `*.ini`.
    fn to_repo(&self) -> RepositoryResult<R>;

    /// Generate a value that `to_repo()` can accept.
    /// It's used for configuring the repository in plain text
    /// e.g. `*.ini`.  URL `scheme` is determined by caller,
    /// and given through argument.
    fn from_repo(repo: &R, scheme: &str) -> Self;
}

mod utils {
    pub type Names<'a> = Box<Iterator<Item=String> + 'a>;

    pub trait Bytes {
        fn as_bytes<'a>(&'a self) -> &'a [u8];
    }

    impl Bytes for [u8] {
        fn as_bytes(&self) -> &[u8] { self }
    }

    impl Bytes for Vec<u8> {
        fn as_bytes(&self) -> &[u8] { &self[] }
    }

    impl Bytes for str {
        fn as_bytes(&self) -> &[u8] { StrExt::as_bytes(self) }
    }

    impl Bytes for String {
        fn as_bytes(&self) -> &[u8] { StrExt::as_bytes(&self[]) }
    }

    impl<'a, T: ?Sized + Bytes> Bytes for &'a T {
        fn as_bytes(&self) -> &[u8] { (*self).as_bytes() }
    }
}

#[cfg(test)]
#[macro_use]
pub mod test {
    use super::{Names, Repository, RepositoryError, RepositoryResult};

    use std::borrow::ToOwned;
    use std::collections::BTreeSet;
    use std::old_io::fs::PathExtensions;
    use std::old_io::util::{NullReader, NullWriter};

    struct RepositoryImplemented;
    
    impl Repository for RepositoryImplemented {
        fn get_reader<T: Str>(&self, _key: &[T]) ->
            RepositoryResult<Box<Buffer>>
        {
            Ok(Box::new(NullReader) as Box<Buffer>)
        }

        fn get_writer<T: Str>(&mut self, _key: &[T]) ->
            RepositoryResult<Box<Writer>>
        {
            Ok(Box::new(NullWriter) as Box<Writer>)
        }

        fn exists<T: Str>(&self, _key: &[T]) -> bool {
            true
        }

        fn list<T: Str>(&self, _key: &[T]) -> RepositoryResult<Names> {
            struct Empty<'a>;
            impl<'a> Iterator for Empty<'a> {
                type Item = String;
                fn next(&mut self) -> Option<String> { None }
                fn size_hint(&self) -> (usize, Option<usize>) { (0, Some(0)) }
            }
            Ok(Box::new(Empty) as Names)
        }
    }

    #[test]
    fn test_dummy_implementation() {
        let mut repository = RepositoryImplemented;
        {
            let mut reader = repository.get_reader(&["key"]).unwrap();
            assert_eq!(reader.read_to_end().unwrap(), vec![]);
        }
        {
            let mut writer = repository.get_writer(&["key"]).unwrap();
            writer.write_str("Hello").unwrap();
        }
        assert!(repository.exists(&["key"]));
        let mut path_list = repository.list(&["key"]).unwrap();
        assert_eq!(path_list.next(), None);
    }

    pub fn test_repository<R: Repository>(mut repository: R) {
        let empty: &[&str] = &[];
        expect_invalid_key!(repository.get_reader, &[]);
        expect_invalid_key!(repository.get_writer, &[]);
        assert_eq!(unwrap!(repository.list(empty)).next(), None);
        assert!(!repository.exists(&["key"]));
        expect_invalid_key!(repository.read, &["key"]);
        unwrap!(repository.write(&["key"], &["cont", "ents"]));
        assert_eq!(unwrap!(repository.list(empty)).collect::<Vec<_>>(),
                   ["key"]);
        assert!(repository.exists(&["key"]));
        assert_eq!(unwrap!(repository.read(&["key"])), b"contents");
        assert!(!repository.exists(&["dir", "key"]));
        expect_invalid_key!(repository.read, &["dir", "key"]);
        unwrap!(repository.write(&["dir", "key"], &["cont", "ents"]));
        assert_eq!(unwrap!(repository.list(empty)).collect::<BTreeSet<_>>(),
                   ["dir".to_string(), "key".to_string()]
                   .iter().map(ToOwned::to_owned).collect::<BTreeSet<_>>());
        assert!(repository.exists(&["dir", "key"]));
        assert!(!repository.exists(&["dir", "key2"]));
        assert_eq!(unwrap!(repository.read(&["dir", "key"])), b"contents");
        // directory test
        expect_invalid_key!(repository.get_writer, &["key", "key"]);
        expect_invalid_key!(repository.list, &["key"]);
    }
}