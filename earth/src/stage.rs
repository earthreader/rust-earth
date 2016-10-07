pub use self::dirtybuffer::DirtyBuffer;


mod dirtybuffer {
    use repository as repo;
    use repository::{Names, Repository};

    use std::borrow::ToOwned;
    use std::collections::{HashMap, HashSet};
    use std::collections::hash_map::Entry;
    use std::io;

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

        pub fn flush(&mut self) -> repo::Result<()> {
            _flush(&mut self.inner, &mut self.dictionary, vec![])
        }
    }

    fn _flush<R: Repository>(repo: &mut R,
                             _dictionary: &mut Dictionary,
                             _key: Vec<String>) -> repo::Result<()> {
        for (k, value) in _dictionary.iter_mut() {
            let mut key = _key.clone();
            key.push(k.clone());
            match *value {
                NestedItem::Map(ref mut m) => { return _flush(repo, m, key); }
                NestedItem::Item(Some(ref v)) => {
                    // TODO: merge with inner repo
                    let mut w = try!(repo.get_writer(&key));
                    try!(w.write_all(&v));
                }
                _ => { /* unsure */ }
            }
        }
        _dictionary.clear();
        Ok(())
    }

    impl<R: Repository> Repository for DirtyBuffer<R> {
        fn get_reader<'a, T: AsRef<str>>(&'a self, key: &[T]) ->
            repo::Result<Box<io::BufRead + 'a>>
        {
            let b = match find_item(&self.dictionary, key) {
                FindResult::Found(&NestedItem::Item(Some(ref v))) => v,
                FindResult::NotFound => { return self.inner.get_reader(key); }
                _ => { return Err(repo::Error::invalid_key(key, None)); }
            };
            let reader = io::BufReader::new(&b[..]);
            Ok(Box::new(reader) as Box<io::BufRead>)
        }

        fn get_writer<'a, T: AsRef<str>>(&'a mut self, key: &[T]) ->
            repo::Result<Box<io::Write + 'a>>
        {
            let mut slot = match dig(&mut self.dictionary, key) {
                Some(v) => v,
                None => { return Err(repo::Error::invalid_key(key, None)); }
            };
            let writer = DirtyWriter {
                slot: slot,
                writer: Some(Vec::new()),
            };
            Ok(Box::new(writer) as Box<io::Write>)
        }

        fn exists<T: AsRef<str>>(&self, key: &[T]) -> bool {
            match find_item(&self.dictionary, key) {
                FindResult::Found(_) => true,
                FindResult::NotFound => self.inner.exists(key),
                FindResult::InvalidKey => false,
            }
        }

        fn list<T: AsRef<str>>(&self, key: &[T]) -> repo::Result<Names> {
            let d = if key.is_empty() {
                &self.dictionary
            } else {
                match find_item(&self.dictionary, key) {
                    FindResult::Found(&NestedItem::Map(ref v)) => v,
                    FindResult::NotFound => { return self.inner.list(key); }
                    _ => {
                        return Err(repo::Error::invalid_key(key, None));
                    }
                }
            };
            let names = d.iter().filter_map(|(k, v)| match *v {
                NestedItem::Item(None) => None,
                _ => Some(k.clone()),
            });
            let src = match self.inner.list(key) {
                Ok(src) => src,
                Err(_) => {
                    let names = names.map(|k| {
                        let v: repo::Result<_> = Ok(k);
                        v
                    });
                    return Ok(Box::new(names) as Names);
                }
            };
            let names = NameList {
                cached: names,
                knowns: HashSet::new(),
                inner: src,
            };
            Ok(Box::new(names) as Names)
        }
    }

    struct NameList<'a, I> where I: Iterator<Item=String> {
        cached: I,
        knowns: HashSet<String>,
        inner: Names<'a>,
    }

    impl<'a, I> Iterator for NameList<'a, I> where I: Iterator<Item=String> {
        type Item = repo::Result<String>;

        fn next(&mut self) -> Option<repo::Result<String>> {
            if let Some(v) = self.cached.next() {
                self.knowns.insert(v.clone());
                Some(Ok(v))
            } else {
                loop {
                    match self.inner.next() {
                        Some(Ok(v)) => {
                            if self.knowns.contains(&v) {
                                continue
                            } else {
                                return Some(Ok(v));
                            }
                        }
                        x => { return x; }
                    }
                }
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let (l1, u1) = self.cached.size_hint();
            let (l2, u2) = self.inner.size_hint();
            let upper = match (u1, u2) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None,
            };
            (l1 + l2, upper)
        }
    }

    pub struct DirtyWriter<'a> {
        slot: &'a mut Option<Vec<u8>>,
        writer: Option<Vec<u8>>,
    }

    impl<'a> io::Write for DirtyWriter<'a> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writer.as_mut().unwrap().write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.writer.as_mut().unwrap().flush()
        }
    }

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

    fn find_item<'a, T: AsRef<str>>(dict: &'a Dictionary, key: &[T]) ->
        FindResult<&'a NestedItem<PathKey, Option<Vec<u8>>>>
    {
        let head = match key.first() {
            Some(k) => k,
            None => { return FindResult::InvalidKey; }
        };
        let tail = &key[1..];
        match dict.get(head.as_ref()) {
            Some(v) if tail.is_empty() => FindResult::Found(v),
            Some(&NestedItem::Map(ref m)) => find_item(m, &key[1..]),
            None => FindResult::NotFound,
            _ => FindResult::InvalidKey,
        }
    }

    fn dig<'a, T: AsRef<str>>(map: &'a mut Dictionary, key: &[T]) ->
        Option<&'a mut Option<Vec<u8>>>
    {
        let head = match key.first() {
            Some(k) => k.as_ref().to_owned(),
            None => { return None; }
        };
        let tail = &key[1..];
        let mut next = match map.entry(head) {
            Entry::Occupied(slot) => match slot.into_mut() {
                &mut NestedItem::Map(ref mut m) => m,
                _ => { return None; }
            },
            Entry::Vacant(slot) => {
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
