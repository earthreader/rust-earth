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
use std::borrow::ToOwned;
use std::error::Error as ErrorTrait;
use std::fmt;
use std::io;
use std::iter::IntoIterator;
use std::path::PathBuf;

pub use self::utils::{Bytes, Names};
pub use self::fs::FileSystemRepository;

pub mod fs;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    InvalidKey(Vec<String>, Option<io::Error>),
    InvalidUrl(&'static str),
    NotADirectory(PathBuf),
    CannotBorrow,
    Io(io::Error),
}

impl Error {
    #[inline]
    pub fn invalid_key<T: AsRef<str>>(key: &[T], cause: Option<io::Error>) ->
        Error
    {
        let copied_key = key.iter()
            .map(|e| e.as_ref().to_owned())
            .collect();
        Error::InvalidKey(copied_key, cause)
    }

    #[inline]
    pub fn invalid_url(detail: &'static str) -> Error {
        Error::InvalidUrl(detail)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "{}", self.description()));
        match *self {
            Error::InvalidKey(ref key, _) => {
                try!(write!(f, ": ["));
                let mut first = true;
                for i in key.iter() {
                    if first { first = false; } else { try!(write!(f, ", ")); }
                    try!(write!(f, "{:?}", i));
                }
                try!(write!(f, "]"));
            }
            Error::InvalidUrl(ref msg) => {
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

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::InvalidKey(_, _) => "invalid key",
            Error::InvalidUrl(_) => "invalid URL",
            Error::NotADirectory(_) => "not a directory",
            Error::CannotBorrow => "can't borrow",
            Error::Io(_) => "IO error"
        }
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        match *self {
            Error::InvalidKey(_, Some(ref err)) => Some(err as &::std::error::Error),
            Error::Io(ref err) => Some(err as &::std::error::Error),
            _ => None
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
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
    fn get_reader<'a, T>(&'a self, key: &[T]) -> Result<Box<io::BufRead + 'a>>
        where T: AsRef<str>;

    /// Get a writer to write data into the ``key``.
    fn get_writer<'a, T>(&'a mut self, key: &[T]) -> Result<Box<io::Write + 'a>>
        where T: AsRef<str>;

    fn read<T>(&self, key: &[T]) -> Result<Vec<u8>>
        where T: AsRef<str>
    {
        let mut buf = vec![];
        try!(try!(self.get_reader(key)).read_to_end(&mut buf));
        Ok(buf)
    }

    fn write<T, U, I>(&mut self, key: &[T], buf: I) -> Result<()>
        where T: AsRef<str>, U: Bytes, I: IntoIterator<Item=U>
    {
        let mut w = try!(self.get_writer(key));
        for b in buf {
            try!(w.write_all(b.as_bytes()));
        }
        Ok(())
    }

    /// Return whether the `key` exists or not.
    fn exists<T: AsRef<str>>(&self, key: &[T]) -> bool;

    /// List all subkeys in the `key`.
    fn list<'a, T: AsRef<str>>(&'a self, key: &[T]) -> Result<Names<'a>>;
}

pub trait ToRepository<R: Repository> {
    /// Create a new instance of the repository from itself.
    /// It may be used for configuring the repository in plain text
    /// e.g. `*.ini`.
    fn to_repo(&self) -> Result<R>;

    /// Generate a value that `to_repo()` can accept.
    /// It's used for configuring the repository in plain text
    /// e.g. `*.ini`.
    fn from_repo(repo: &R) -> Self;
}

mod utils {
    pub type Names<'a> = Box<Iterator<Item=super::Result<String>> + 'a>;

    pub trait Bytes {
        fn as_bytes<'a>(&'a self) -> &'a [u8];
    }

    impl Bytes for [u8] {
        fn as_bytes(&self) -> &[u8] { self }
    }

    impl Bytes for Vec<u8> {
        fn as_bytes(&self) -> &[u8] { &self }
    }

    impl Bytes for str {
        fn as_bytes(&self) -> &[u8] { str::as_bytes(self) }
    }

    impl Bytes for String {
        fn as_bytes(&self) -> &[u8] { str::as_bytes(&self) }
    }

    impl<'a, T: ?Sized + Bytes> Bytes for &'a T {
        fn as_bytes(&self) -> &[u8] { (*self).as_bytes() }
    }
}

#[cfg(test)]
#[macro_use]
pub mod test {
    use super::{Names, Repository};

    use std::borrow::ToOwned;
    use std::collections::BTreeSet;
    use std::io;
    use std::marker::PhantomData;

    struct RepositoryImplemented;

    impl Repository for RepositoryImplemented {
        fn get_reader<T: AsRef<str>>(&self, _key: &[T]) ->
            super::Result<Box<io::BufRead>>
        {
            Ok(Box::new(io::empty()) as Box<io::BufRead>)
        }

        fn get_writer<T: AsRef<str>>(&mut self, _key: &[T]) ->
            super::Result<Box<io::Write>>
        {
            Ok(Box::new(io::sink()) as Box<io::Write>)
        }

        fn exists<T: AsRef<str>>(&self, _key: &[T]) -> bool {
            true
        }

        fn list<T: AsRef<str>>(&self, _key: &[T]) -> super::Result<Names> {
            struct Empty<'a> { _a: PhantomData<&'a ()> }
            impl<'a> Iterator for Empty<'a> {
                type Item = super::Result<String>;
                fn next(&mut self) -> Option<super::Result<String>> { None }
                fn size_hint(&self) -> (usize, Option<usize>) { (0, Some(0)) }
            }
            Ok(Box::new(Empty { _a: PhantomData }) as Names)
        }
    }

    #[test]
    fn test_dummy_implementation() {
        let mut repository = RepositoryImplemented;
        {
            let mut reader = repository.get_reader(&["key"]).unwrap();
            let mut buf = vec![];
            assert_eq!(reader.read_to_end(&mut buf).unwrap(), 0);
            assert!(buf.len() == 0);
        }
        {
            let mut writer = repository.get_writer(&["key"]).unwrap();
            writer.write_all("Hello".as_bytes()).unwrap();
        }
        assert!(repository.exists(&["key"]));
        let mut path_list = repository.list(&["key"]).unwrap();
        assert!(path_list.next().is_none());
    }

    pub fn test_repository<R: Repository>(mut repository: R) {
        let empty: &[&str] = &[];
        expect_invalid_key!(repository.get_reader, &[]);
        expect_invalid_key!(repository.get_writer, &[]);
        assert!(unwrap!(repository.list(empty)).next().is_none());
        assert!(!repository.exists(&["key"]));
        expect_invalid_key!(repository.read, &["key"]);
        unwrap!(repository.write(&["key"], &["cont", "ents"]));
        assert_eq!(
            repository.list(empty).unwrap().map(|e| e.unwrap())
                .collect::<Vec<_>>(),
            ["key"]);
        assert!(repository.exists(&["key"]));
        assert_eq!(unwrap!(repository.read(&["key"])), b"contents");
        assert!(!repository.exists(&["dir", "key"]));
        expect_invalid_key!(repository.read, &["dir", "key"]);
        unwrap!(repository.write(&["dir", "key"], &["cont", "ents"]));
        assert_eq!(
            repository.list(empty).unwrap()
                .map(|e| e.unwrap())
                .collect::<BTreeSet<_>>(),
            ["dir", "key"].iter().map(|&e| e.to_owned())
                .collect::<BTreeSet<_>>());
        assert!(repository.exists(&["dir", "key"]));
        assert!(!repository.exists(&["dir", "key2"]));
        assert_eq!(unwrap!(repository.read(&["dir", "key"])), b"contents");
        // directory test
        expect_invalid_key!(repository.get_writer, &["key", "key"]);
        expect_invalid_key!(repository.list, &["key"]);
    }
}
