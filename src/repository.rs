#![allow(unstable)]

use std::error::{Error, FromError};
use std::io;
use std::io::IoError;
use std::io::fs::{File, PathExtensions, readdir, mkdir_recursive};
use std::path::BytesContainer;

use url::{Url};

const ENOENT: usize = 2;  // from `libc` crate

pub type RepositoryResult<T> = Result<T, RepositoryError>;

#[derive(Show)]
pub enum RepositoryError {
    InvalidKey(Vec<Vec<u8>>, Option<IoError>),
    InvalidUrl(&'static str),
    NotADirectory(Path),
    CannotBorrow,
    Io(IoError),
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

    fn detail(&self) -> Option<String> {
        match *self {
            RepositoryError::InvalidUrl(ref msg) => Some(msg.to_string()),
            RepositoryError::Io(ref err) => err.detail(),
            _ => None
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

#[inline]
pub fn invalid_key<T: BytesContainer>(key: &[T], cause: Option<IoError>) ->
    RepositoryError
{
    let copied_key = key.iter()
        .map(|e| e.container_as_bytes().to_vec())
        .collect();
    RepositoryError::InvalidKey(copied_key, cause)
}

#[inline]
fn invalid_url(detail: &'static str) -> RepositoryError {
    RepositoryError::InvalidUrl(detail)
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
/// ```ignore
/// repository.list(&["dir", "subdir"])
/// ```
pub trait Repository {
    /// Read the content from the `key`.
    fn read<'a, T: BytesContainer>(&'a self, key: &[T]) ->
        RepositoryResult<Box<Buffer + 'a>>;

    /// Get a writer to write data into the ``key``.
    fn write<'a, T: BytesContainer>(&'a mut self, key: &[T]) ->
        RepositoryResult<Box<Writer + 'a>>;

    /// Return whether the `key` exists or not.
    fn exists<T: BytesContainer>(&self, key: &[T]) -> bool;

    /// List all subkeys in the `key`.
    fn list<'a, T: BytesContainer>(&'a self, key: &[T]) ->
        RepositoryResult<Names<'a>>;
}

pub struct Names<'a>(Box<Iterator<Item=Vec<u8>> + 'a>);

impl<'a> Names<'a> {
    pub fn new<'b, T>(iter: T) -> Names<'b>
        where T: Iterator<Item=Vec<u8>> + 'b
    {
        Names(Box::new(iter) as Box<Iterator<Item=Vec<u8>>>)
    }
}

impl<'a> Iterator for Names<'a> {
    type Item = Vec<u8>;
    fn next(&mut self) -> Option<Vec<u8>> { self.0.next() }
    fn size_hint(&self) -> (usize, Option<usize>) { self.0.size_hint() }
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


/// Builtin implementation of `Repository` trait which uses the ordinary
/// file system.
pub struct FileSystemRepository {
    path: Path,
}

impl FileSystemRepository {
    pub fn from_path(path: &Path, mkdir: bool) -> RepositoryResult<FileSystemRepository> {
        if !path.exists() {
            if mkdir {
                match mkdir_recursive(path, io::USER_DIR) {
                    Ok(_) => { }
                    Err(err) => match err.kind {
                        io::IoErrorKind::PathAlreadyExists => { }
                        _ => { return Err(FromError::from_error(err)) }
                    }
                }
            } else {
                return Err(FromError::from_error(
                    IoError::from_errno(ENOENT, false)));
            }
        }
        if !path.is_dir() {
            return Err(RepositoryError::NotADirectory(path.clone()));
        }
        Ok(FileSystemRepository {
            path: path.clone()
        })
    }
}

impl Repository for FileSystemRepository {
    fn read<'a, T: BytesContainer>(&'a self, key: &[T]) ->
        RepositoryResult<Box<Buffer + 'a>>
    {
        let path = self.path.join_many(key);
        if !path.is_file() {
            return Err(invalid_key(key, None));
        }
        let file = try!(File::open(&path));
        Ok(Box::new(io::BufferedReader::new(file)) as Box<Buffer>)
    }

    fn write<'a, T: BytesContainer>(&'a mut self, key: &[T]) ->
        RepositoryResult<Box<Writer + 'a>>
    {
        let path = self.path.join_many(key);
        let dir_path = path.dir_path();
        try!(mkdir_recursive(&dir_path, io::USER_DIR));
        let file_res = File::open_mode(&path,
                                       io::FileMode::Open,
                                       io::FileAccess::Write);
        let file = match file_res {
            Ok(f) => f,
            Err(e) => { return Err(invalid_key(key, Some(e))); }
        };
        Ok(Box::new(file) as Box<Writer>)
    }

    fn exists<T: BytesContainer>(&self, key: &[T]) -> bool {
        PathExtensions::exists(&self.path.join_many(key))
    }

    fn list<'a, T: BytesContainer>(&'a self, key: &[T]) ->
        RepositoryResult<Names<'a>>
    {
        let names = match readdir(&self.path.join_many(key)) {
            Ok(v) => v,
            Err(e) => { return Err(invalid_key(key, Some(e))); }
        };
        let iter = names.into_iter().filter_map(|path| path.filename()
                                                           .map(|p| p.to_vec()));
        Ok(Names::new(iter))
    }
}

impl ToRepository<FileSystemRepository> for Url {
    fn to_repo(&self) -> RepositoryResult<FileSystemRepository> {
        if self.scheme != "file" {
            return Err(invalid_url("FileSystemRepository only accepts file:// scheme"));
        } else if self.query != None || self.fragment != None {
            return Err(invalid_url("file:// must not contain any host/port/user/password/parameters/query/fragment"));
        }
        let path = match self.to_file_path() {
            Ok(p) => p,
            Err(_) => { return Err(invalid_url("invalid file path")); }
        };
        FileSystemRepository::from_path(&path, true)
    }

    fn from_repo(repo: &FileSystemRepository, scheme: &str) -> Url {
        match Url::from_file_path(&repo.path) {
            Ok(mut v) => {
                v.scheme = scheme.to_string();
                v
            },
            Err(_) => unimplemented!()
        }
    }
}


#[cfg(test)]
mod test {
    use super::{Names, Repository, RepositoryError, RepositoryResult,
                ToRepository};
    use super::{FileSystemRepository};
    
    use std::io::{File, IoErrorKind, TempDir, USER_DIR};
    use std::io::fs::{PathExtensions, mkdir_recursive};
    use std::io::util::{NullReader, NullWriter};
    use std::path::BytesContainer;
    use std::str;

    use url::Url;

    struct RepositoryImplemented;
    
    impl Repository for RepositoryImplemented {
        fn read<T: BytesContainer>(&self, _key: &[T]) ->
            RepositoryResult<Box<Buffer>>
        {
            Ok(Box::new(NullReader) as Box<Buffer>)
        }

        fn write<T: BytesContainer>(&mut self, _key: &[T]) ->
            RepositoryResult<Box<Writer>>
        {
            Ok(Box::new(NullWriter) as Box<Writer>)
        }

        fn exists<T: BytesContainer>(&self, _key: &[T]) -> bool {
            true
        }

        fn list<T: BytesContainer>(&self, _key: &[T]) ->
            RepositoryResult<Names>
        {
            struct Empty<'a>;
            impl<'a> Iterator for Empty<'a> {
                type Item = Vec<u8>;
                fn next(&mut self) -> Option<Vec<u8>> { None }
                fn size_hint(&self) -> (usize, Option<usize>) { (0, Some(0)) }
            }
            Ok(Names::new(Empty))
        }
    }

    #[test]
    fn test_dummy_implementation() {
        let mut repository = RepositoryImplemented;
        {
            let mut reader = repository.read(&["key"]).unwrap();
            assert_eq!(reader.read_to_end().unwrap(), vec![]);
        }
        {
            let mut writer = repository.write(&["key"]).unwrap();
            writer.write_str("Hello").unwrap();
        }
        assert!(repository.exists(&["key"]));
        let mut path_list = repository.list(&["key"]).unwrap();
        assert_eq!(path_list.next(), None);
    }

    fn temp_dir() -> TempDir {
        TempDir::new("rust-earth-test").unwrap()
    }

    macro_rules! assert_err {
        ($expr:expr, $err_pat:pat => $blk:block) => (
            match $expr {
                Ok(_) => { panic!("unexpected success"); }
                Err($err_pat) => $blk,
                Err(e) => { panic!("unexpected error: {:?}", e); }
            }
        )
    }

    fn expect_invalid_key<T>(v: RepositoryResult<T>, key: &[&[u8]]) {
        assert_err!(v, RepositoryError::InvalidKey(k, _) => {
            assert_eq!(k, key);
        });
    }

    #[test]
    fn test_file_from_to_url() {
        let tmpdir = temp_dir();
        let path_str = tmpdir.path().as_str().unwrap();
        let raw_url = format!("file://{}", path_str);
        let url = Url::parse(&*raw_url).unwrap();
        let fs: FileSystemRepository = url.to_repo().unwrap();
        assert_eq!(&fs.path, tmpdir.path());
        let u1: Url = ToRepository::from_repo(&fs, "file");
        let u2: Url = ToRepository::from_repo(&fs, "fs");
        assert_eq!(u1, url);
        assert_eq!(u2.serialize(), format!("fs://{}", path_str));
    }

    #[test]
    fn test_file_read() {
        let tmpdir = temp_dir();
        let f = FileSystemRepository::from_path(tmpdir.path(), true).unwrap();

        expect_invalid_key(f.read(&["key"]), &[b"key"]);
        {
            let mut file = File::create(&tmpdir.path().join("key")).unwrap();
            write!(&mut file, "file content").unwrap();
        }
        let content = f.read(&["key"]).unwrap().read_to_end().unwrap();
        assert_eq!(&content[], b"file content");
    }

    #[test]
    fn test_file_read_nested() {
        let tmpdir = temp_dir();
        let f = FileSystemRepository::from_path(tmpdir.path(), true).unwrap();

        expect_invalid_key(f.read(&["dir", "dir2", "key"]),
                           &[b"dir", b"dir2", b"key"]);
        {
            let mut path = tmpdir.path().clone();
            path.push("dir");
            path.push("dir2");
            mkdir_recursive(&path, USER_DIR).unwrap();
            path.push("key");
            let mut file = File::create(&path).unwrap();
            write!(&mut file, "file content").unwrap();
        }
        let content = f.read(&["dir", "dir2", "key"]).unwrap().read_to_end().unwrap();
        assert_eq!(&content[], b"file content");
    }

    #[test]
    fn test_file_write() {
        let tmpdir = temp_dir();
        let mut f = FileSystemRepository::from_path(tmpdir.path(), true).unwrap();
        {
            let mut w = f.write(&["key"]).unwrap();
            write!(&mut w, "file ").unwrap();
            write!(&mut w, "content").unwrap();
        }
        let content = File::open(&tmpdir.path().join("key")).unwrap().read_to_end().unwrap();
        assert_eq!(content, b"file content");
    }
    
    #[test]
    fn test_file_write_nested() {
        let tmpdir = temp_dir();
        let mut f = FileSystemRepository::from_path(tmpdir.path(), true).unwrap();
        {
            let mut w = f.write(&["dir", "dir2", "key"]).unwrap();
            write!(&mut w, "deep ").unwrap();
            write!(&mut w, "dark ").unwrap();
            write!(&mut w, "content").unwrap();
        }
        let path = tmpdir.path().join_many(&["dir", "dir2", "key"]);
        let content = File::open(&path).unwrap().read_to_end().unwrap();
        assert_eq!(content, b"deep dark content");
    }

    #[test]
    fn test_file_write_on_wrong_key() {
        let tmpdir = temp_dir();
        let mut f = FileSystemRepository::from_path(tmpdir.path(), true).unwrap();
        let empty: &[&[u8]] = &[];
        expect_invalid_key(f.write(empty), empty);
    }

    #[test]
    fn test_file_exists() {
        let tmpdir = temp_dir();
        let f = FileSystemRepository::from_path(tmpdir.path(), true).unwrap();
        {
            let path = tmpdir.path().join("dir");
            mkdir_recursive(&path, USER_DIR).unwrap();
            let mut file = File::create(&path.join("file")).unwrap();
            write!(&mut file, "content").unwrap();
            let mut file = File::create(&tmpdir.path().join("file")).unwrap();
            write!(&mut file, "content").unwrap();
        }
        assert!(f.exists(&["dir"]));
        assert!(f.exists(&["dir", "file"]));
        assert!(f.exists(&["file"]));
        assert!(!f.exists(&["dir", "file-not-exist"]));
        assert!(!f.exists(&["dir-not-exist"]));
    }

    #[test]
    fn test_file_list() {
        let tmpdir = temp_dir();
        let f = FileSystemRepository::from_path(tmpdir.path(), true).unwrap();
        let d = tmpdir.path().join("dir");
        mkdir_recursive(&d, USER_DIR).unwrap();
        for i in 0..100 {
            mkdir_recursive(&d.join(format!("d{}", i)), USER_DIR).unwrap();
        }
        let mut expected: Vec<_> = (0..100)
            .map(|i| format!("d{}", i))
            .collect();
        let mut paths = f.list(&["dir"]).unwrap()
            .map(|i| str::from_utf8(&i[]).unwrap().to_string())
            .collect::<Vec<_>>();
        paths.sort();
        expected.sort();
        assert_eq!(paths, expected);
    }

    #[test]
    fn test_file_list_on_wrong_key() {
        let tmpdir = temp_dir();
        let f = FileSystemRepository::from_path(tmpdir.path(), true).unwrap();
        expect_invalid_key(f.list(&["not-exist"]), &[b"not-exist"]);
    }

    #[test]
    fn test_file_not_found() {
        let tmpdir = temp_dir();
        let path = tmpdir.path().join("not-exist");
        assert_err!(FileSystemRepository::from_path(&path, false),
            RepositoryError::Io(e) => {
                assert_eq!(e.kind, IoErrorKind::FileNotFound);
            });
        let _f = FileSystemRepository::from_path(&path, true);
        assert!(path.is_dir());
    }

    #[test]
    fn test_not_dir() {
        let tmpdir = temp_dir();
        let path = tmpdir.path().join("not-dir.txt");
        File::create(&path).write(&[]).unwrap();
        assert_err!(FileSystemRepository::from_path(&path, false),
            RepositoryError::NotADirectory(p) => {
                assert_eq!(path, p);
            });
        }
    }
}
