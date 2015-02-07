#![unstable]

pub use self::dirtybuffer::DirtyBuffer;


mod dirtybuffer {
    use repository::{Names, Repository, RepositoryError, RepositoryResult};

    use std::collections::{HashMap, HashSet};
    use std::old_io::{BufReader, IoResult, Writer};

    enum NestedItem<K, V> {
        Item(V), Map(HashMap<K, NestedItem<K, V>>)
    }
    type NestedMap<K, V> = HashMap<K, NestedItem<K, V>>;

    type PathKey = String;
    type Dictionary = NestedMap<PathKey, Option<Vec<u8>>>;

    pub struct DirtyBuffer<R> {
        inner: R,
        dictionary: Dictionary,
    }

    impl<R: Repository> DirtyBuffer<R> {
        pub fn new(repo: R) -> DirtyBuffer<R> {
            DirtyBuffer {
                inner: repo,
                dictionary: HashMap::new(),
            }
        }

        pub fn flush(&mut self) -> RepositoryResult<()> {
            _flush(&mut self.inner, &mut self.dictionary, vec![])
        }
    }

    fn _flush<R: Repository>(repo: &mut R,
                             _dictionary: &mut Dictionary,
                             _key: Vec<String>) -> RepositoryResult<()> {
        for (k, value) in _dictionary.iter_mut() {
            let mut key = _key.clone();
            key.push(k.clone());
            match *value {
                NestedItem::Map(ref mut m) => { return _flush(repo, m, key); }
                NestedItem::Item(Some(ref v)) => {
                    // TODO: merge with inner repo
                    let mut w = try!(repo.get_writer(&key[]));
                    try!(w.write_all(&v[]));
                }
                _ => { /* unsure */ }
            }
        }
        _dictionary.clear();
        Ok(())
    }

    impl<R: Repository> Repository for DirtyBuffer<R> {
        fn get_reader<'a, T: Str>(&'a self, key: &[T]) ->
            RepositoryResult<Box<Buffer + 'a>>
        {
            let b = match find_item(&self.dictionary, key) {
                FindResult::Found(&NestedItem::Item(Some(ref v))) => v,
                FindResult::NotFound => { return self.inner.get_reader(key); }
                _ => { return Err(RepositoryError::invalid_key(key, None)); }
            };
            let reader = BufReader::new(&b[]);
            Ok(Box::new(reader) as Box<Buffer>)
        }

        fn get_writer<'a, T: Str>(&'a mut self, key: &[T]) ->
            RepositoryResult<Box<Writer + 'a>>
        {
            let mut slot = match dig(&mut self.dictionary, key) {
                Some(v) => v,
                None => { return Err(RepositoryError::invalid_key(key, None)); }
            };
            let writer = DirtyWriter {
                slot: slot,
                writer: Some(Vec::new()),
            };
            Ok(Box::new(writer) as Box<Writer>)
        }

        fn exists<T: Str>(&self, key: &[T]) -> bool {
            match find_item(&self.dictionary, key) {
                FindResult::Found(_) => true,
                FindResult::NotFound => self.inner.exists(key),
                FindResult::InvalidKey => false,
            }
        }

        fn list<T: Str>(&self, key: &[T]) -> RepositoryResult<Names> {
            let d = if key.is_empty() {
                &self.dictionary
            } else {
                match find_item(&self.dictionary, key) {
                    FindResult::Found(&NestedItem::Map(ref v)) => v,
                    FindResult::NotFound => { return self.inner.list(key); }
                    _ => {
                        return Err(RepositoryError::invalid_key(key, None));
                    }
                }
            };
            let names = d.iter().filter_map(|(k, v)| match *v {
                NestedItem::Item(None) => None,
                _ => Some(k.clone()),
            });
            let src = match self.inner.list(key) {
                Ok(src) => src,
                Err(_) => { return Ok(Box::new(names) as Names); }
            };
            let mut names: HashSet<_> = names.collect();
            names.extend(src);
            Ok(Box::new(names.into_iter()) as Names)
        }
    }

    pub struct DirtyWriter<'a> {
        slot: &'a mut Option<Vec<u8>>,
        writer: Option<Vec<u8>>,
    }

    impl<'a> Writer for DirtyWriter<'a> {
        fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
            self.writer.as_mut().unwrap().write_all(buf)
        }
        fn flush(&mut self) -> IoResult<()> {
            self.writer.as_mut().unwrap().flush()
        }
    }

    #[unsafe_destructor]
    impl<'a> Drop for DirtyWriter<'a> {
        fn drop(&mut self) {
            *self.slot = self.writer.take();
        }
    }

    enum FindResult<T> {
        Found(T),
        NotFound,
        InvalidKey,
    }

    fn find_item<'a, T: Str>(dict: &'a Dictionary, key: &[T]) ->
        FindResult<&'a NestedItem<PathKey, Option<Vec<u8>>>>
    {
        let head = match key.first() {
            Some(k) => k,
            None => { return FindResult::InvalidKey; }
        };
        let tail = key.tail();
        match dict.get(head.as_slice()) {
            Some(v) if tail.is_empty() => FindResult::Found(v),
            Some(&NestedItem::Map(ref m)) => find_item(m, key.tail()),
            None => FindResult::NotFound,
            _ => FindResult::InvalidKey,
        }
    }

    fn dig<'a, T: Str>(map: &'a mut Dictionary, key: &[T]) ->
        Option<&'a mut Option<Vec<u8>>>
    {
        let head = match key.first() {
            Some(k) => k.as_slice().to_string(),
            None => { return None; }
        };
        let tail = key.tail();
        let mut next = match map.entry(head).get() {
            Ok(&mut NestedItem::Map(ref mut m)) => m,
            Err(slot) => {
                if tail.is_empty() {
                    match slot.insert(NestedItem::Item(None)) {
                        &mut NestedItem::Item(ref mut v) => {
                            return Some(v);
                        }
                        _ => unreachable!()
                    }
                } else {
                    match slot.insert(NestedItem::Map(HashMap::new())) {
                        &mut NestedItem::Map(ref mut m) => m,
                        _ => unreachable!()
                    }
                }
            }
            _ => { return None; }
        };
        dig(next, tail)
    }

    #[cfg(test)]
    mod test {
        use super::DirtyBuffer;

        use test_utils::temp_dir;
        use repository::FileSystemRepository;
        use repository::test::test_repository;
        
        #[test]
        fn test_dirty_buffer() {
            let tmpdir = temp_dir();
            let f = FileSystemRepository::from_path(tmpdir.path(), true).unwrap();
            let dirty_buffer = DirtyBuffer::new(f);
            test_repository(dirty_buffer);
        }
    }
}