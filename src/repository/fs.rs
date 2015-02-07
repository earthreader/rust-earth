use super::{Names, Repository, RepositoryError, RepositoryResult,
            ToRepository};

use std::error::FromError;
use std::old_io;
use std::old_io::{BufferedReader, FileAccess, FileMode, IoError, IoErrorKind};
use std::old_io::fs::{File, PathExtensions, readdir, mkdir_recursive};
use std::old_path::BytesContainer;

use url::{Url};

const ENOENT: usize = 2;  // from `libc` crate

/// Builtin implementation of `Repository` trait which uses the ordinary
/// file system.
pub struct FileSystemRepository {
    path: Path,
}

impl FileSystemRepository {
    pub fn from_path(path: &Path, mkdir: bool) ->
        RepositoryResult<FileSystemRepository>
    {
        if !path.exists() {
            if mkdir {
                match mkdir_recursive(path, old_io::USER_DIR) {
                    Ok(_) => { }
                    Err(err) => match err.kind {
                        IoErrorKind::PathAlreadyExists => { }
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
    fn get_reader<'a, T: BytesContainer>(&'a self, key: &[T]) ->
        RepositoryResult<Box<Buffer + 'a>>
    {
        let path = self.path.join_many(key);
        if !path.is_file() {
            return Err(RepositoryError::invalid_key(key, None));
        }
        let file = try!(File::open(&path));
        Ok(Box::new(BufferedReader::new(file)) as Box<Buffer>)
    }

    fn get_writer<'a, T: BytesContainer>(&'a mut self, key: &[T]) ->
        RepositoryResult<Box<Writer + 'a>>
    {
        let path = self.path.join_many(key);
        let dir_path = path.dir_path();
        if !dir_path.exists() {
            match mkdir_recursive(&dir_path, old_io::USER_DIR) {
                Ok(_) => { }
                Err(e) => match e.kind {
                    IoErrorKind::PathAlreadyExists => {
                        return Err(RepositoryError::invalid_key(key, Some(e)));
                    }
                    _ => {
                        return Err(FromError::from_error(e));
                    }
                }
            }
        }
        if path.is_dir() {  // additional check for windows
            return Err(RepositoryError::invalid_key(key, None));
        }
        let file_res = File::open_mode(&path,
                                       FileMode::Open,
                                       FileAccess::Write);
        let file = match file_res {
            Ok(f) => f,
            Err(e) => return Err(RepositoryError::invalid_key(key, Some(e))),
        };
        Ok(Box::new(file) as Box<Writer>)
    }

    fn exists<T: BytesContainer>(&self, key: &[T]) -> bool {
        PathExtensions::exists(&self.path.join_many(key))
    }

    fn list<'a, T: BytesContainer>(&'a self, key: &[T]) ->
        RepositoryResult<Names>
    {
        let names = match readdir(&self.path.join_many(key)) {
            Ok(v) => v,
            Err(e) => return Err(RepositoryError::invalid_key(key, Some(e))),
        };
        let iter = names.into_iter().filter_map(|path| path.filename()
                                                .map(|p| p.to_vec()));
        Ok(Box::new(iter) as Names)
    }
}

impl ToRepository<FileSystemRepository> for Url {
    fn to_repo(&self) -> RepositoryResult<FileSystemRepository> {
        if self.scheme != "file" {
            return Err(RepositoryError::invalid_url(
                "FileSystemRepository only accepts file:// scheme"));
        } else if self.query != None || self.fragment != None {
            return Err(RepositoryError::invalid_url(
                concat!("file:// must not contain any host/port/user/",
                        "password/parameters/query/fragment")));
        }
        let path = match self.to_file_path() {
            Ok(p) => p,
            Err(_) => {
                return Err(RepositoryError::invalid_url("invalid file path"));
            }
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
    use test_utils::temp_dir;
    use super::super::test::test_repository;

    use super::super::{Repository, RepositoryError, ToRepository};
    use super::FileSystemRepository as FsRepo;

    use std::collections::BTreeSet;
    use std::old_io::{File, IoErrorKind, USER_DIR};
    use std::old_io::fs::{PathExtensions, mkdir_recursive};
    use std::str;

    use url::Url;

    #[cfg(not(windows))]
    #[test]
    fn test_file_from_to_url_on_posix() {
        let tmpdir = temp_dir();
        let path_str = tmpdir.path().as_str().unwrap();
        let raw_url = format!("file://{}", path_str);
        let url = Url::parse(&*raw_url).unwrap();
        let fs: FsRepo = url.to_repo().unwrap();
        assert_eq!(&fs.path, tmpdir.path());
        let u1: Url = ToRepository::from_repo(&fs, "file");
        let u2: Url = ToRepository::from_repo(&fs, "fs");
        assert_eq!(u1, url);
        assert_eq!(u2.serialize(), format!("fs://{}", path_str));
    }

    #[cfg(windows)]
    #[test]
    fn test_file_from_to_url_on_windows() {
        use std::old_path::windows::prefix;
        use std::old_path::windows::PathPrefix;
        let tmpdir = temp_dir();
        let path_str = tmpdir.path()
            .str_components()
            .map(|e| e.unwrap())

            .collect::<Vec<_>>()
            .connect("/");
        let path_prefix_len = match prefix(tmpdir.path()) {
            None => 0,
            Some(PathPrefix::VerbatimPrefix(x)) => 4 + x,
            Some(PathPrefix::VerbatimUNCPrefix(x,y)) => 8 + x + 1 + y,
            Some(PathPrefix::VerbatimDiskPrefix) => 6,
            Some(PathPrefix::UNCPrefix(x,y)) => 2 + x + 1 + y,
            Some(PathPrefix::DeviceNSPrefix(x)) => 4 + x,
            Some(PathPrefix::DiskPrefix) => 2
        };
        let path_prefix = &tmpdir.path().as_str()
            .unwrap()[0..path_prefix_len];
        println!("{}", path_str);
        let raw_url = format!("file:///{}/{}", path_prefix, path_str);
        let url = Url::parse(&*raw_url).unwrap();
        let fs: FsRepo = url.to_repo().unwrap();
        assert_eq!(&fs.path, tmpdir.path());
        let u1: Url = ToRepository::from_repo(&fs, "file");
        let u2: Url = ToRepository::from_repo(&fs, "fs");
        assert_eq!(u1, url);
        assert_eq!(u2.serialize(),
                   format!("fs:///{}/{}", path_prefix, path_str));
    }

    #[test]
    fn test_file_read() {
        let tmpdir = temp_dir();
        let f = FsRepo::from_path(tmpdir.path(), true).unwrap();

        expect_invalid_key!(f.get_reader, &[b"key"]);
        {
            let mut file = File::create(&tmpdir.path().join("key")).unwrap();
            write!(&mut file, "file content").unwrap();
        }
        let content = f.get_reader(&["key"]).unwrap().read_to_end().unwrap();
        assert_eq!(&content[], b"file content");
    }

    #[test]
    fn test_file_read_nested() {
        let tmpdir = temp_dir();
        let f = FsRepo::from_path(tmpdir.path(), true).unwrap();

        expect_invalid_key!(f.get_reader, &[b"dir", b"dir2", b"key"]);
        {
            let mut path = tmpdir.path().clone();
            path.push("dir");
            path.push("dir2");
            mkdir_recursive(&path, USER_DIR).unwrap();
            path.push("key");
            let mut file = File::create(&path).unwrap();
            write!(&mut file, "file content").unwrap();
        }
        let content = f.get_reader(&["dir", "dir2", "key"]).unwrap()
            .read_to_end().unwrap();
        assert_eq!(&content[], b"file content");
    }

    #[test]
    fn test_file_write() {
        let tmpdir = temp_dir();
        let mut f = FsRepo::from_path(tmpdir.path(), true).unwrap();
        {
            let mut w = f.get_writer(&["key"]).unwrap();
            write!(&mut w, "file ").unwrap();
            write!(&mut w, "content").unwrap();
        }
        let content = File::open(&tmpdir.path().join("key")).unwrap()
            .read_to_end().unwrap();
        assert_eq!(content, b"file content");
    }
    
    #[test]
    fn test_file_write_nested() {
        let tmpdir = temp_dir();
        let mut f = FsRepo::from_path(tmpdir.path(), true).unwrap();
        {
            let mut w = f.get_writer(&["dir", "dir2", "key"]).unwrap();
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
        let mut f = FsRepo::from_path(tmpdir.path(), true).unwrap();
        expect_invalid_key!(f.get_writer, &[]);
    }

    #[test]
    fn test_file_exists() {
        let tmpdir = temp_dir();
        let f = FsRepo::from_path(tmpdir.path(), true).unwrap();
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
        let f = FsRepo::from_path(tmpdir.path(), true).unwrap();
        let d = tmpdir.path().join("dir");
        mkdir_recursive(&d, USER_DIR).unwrap();
        for i in 0..100 {
            mkdir_recursive(&d.join(format!("d{}", i)), USER_DIR).unwrap();
        }
        let expected: BTreeSet<_> = (0..100)
            .map(|i| format!("d{}", i))
            .collect();
        let paths = f.list(&["dir"]).unwrap()
            .map(|i| str::from_utf8(&i[]).unwrap().to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(paths, expected);
    }

    #[test]
    fn test_file_list_on_wrong_key() {
        let tmpdir = temp_dir();
        let f = FsRepo::from_path(tmpdir.path(), true).unwrap();
        expect_invalid_key!(f.list, &[b"not-exist"]);
    }

    #[test]
    fn test_file_not_found() {
        let tmpdir = temp_dir();
        let path = tmpdir.path().join("not-exist");
        assert_err!(FsRepo::from_path(&path, false),
                    RepositoryError::Io(e) => {
                        assert_eq!(e.kind, IoErrorKind::FileNotFound);
                    });
        let _f = FsRepo::from_path(&path, true);
        assert!(path.is_dir());
    }

    #[test]
    fn test_not_dir() {
        let tmpdir = temp_dir();
        let path = tmpdir.path().join("not-dir.txt");
        File::create(&path).write_all(&[]).unwrap();
        assert_err!(FsRepo::from_path(&path, false),
                    RepositoryError::NotADirectory(p) => {
                        assert_eq!(path, p);
                    });
    }
    
    
    #[test]
    fn test_filesystem_repository() {
        let tmpdir = temp_dir();
        let f = FsRepo::from_path(tmpdir.path(), true).unwrap();
        test_repository(f);
    }
}
