pub use self::dirtybuffer::DirtyBuffer;


mod dirtybuffer {
    use repository::{Names, Repository, RepositoryResult, invalid_key};

    use std::collections::{HashMap};
    use std::io::{BufReader, IoResult, MemWriter, Writer};
    use std::path::BytesContainer;

    enum NestedItem<K, V> {
        Item(V), Map(HashMap<K, NestedItem<K, V>>)
    }
    type NestedMap<K, V> = HashMap<K, NestedItem<K, V>>;

    type PathKey = Vec<u8>;
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
    }

    impl<R: Repository> Repository for DirtyBuffer<R> {
        fn read<'a, T: BytesContainer>(&'a self, key: &[T]) ->
            RepositoryResult<Box<Buffer + 'a>>
        {
            let b = match find_item(&self.dictionary, key) {
                Ok(Some(v)) => v,
                Ok(None) => { return self.inner.read(key); }
                Err(_) => { return Err(invalid_key(key, None)); }
            };
            let reader = BufReader::new(&b[]);
            Ok(Box::new(reader) as Box<Buffer>)
        }

        fn write<'a, T: BytesContainer>(&'a mut self, key: &[T]) ->
            RepositoryResult<Box<Writer + 'a>>
        {
            let mut slot = match dig(&mut self.dictionary, key) {
                Some(v) => v,
                None => { return Err(invalid_key(key, None)); }
            };
            let writer = DirtyWriter {
                slot: slot,
                writer: MemWriter::new(),
            };
            Ok(Box::new(writer) as Box<Writer>)
        }

        fn exists<T: BytesContainer>(&self, _key: &[T]) -> bool {
            true
        }

        fn list<T: BytesContainer>(&self, _key: &[T]) ->
            RepositoryResult<Names>
        {
            let names = self.dictionary.keys();
            Ok(Names::new(names.map(|e| &e[])))
        }
    }

    pub struct DirtyWriter<'a> {
        slot: &'a mut Option<Vec<u8>>,
        writer: MemWriter,
    }

    impl<'a> Writer for DirtyWriter<'a> {
        fn write(&mut self, buf: &[u8]) -> IoResult<()> {
            self.writer.write(buf)
        }
        fn flush(&mut self) -> IoResult<()> { self.writer.flush() }
    }

    #[unsafe_destructor]
    impl<'a> Drop for DirtyWriter<'a> {
        fn drop(&mut self) { }
    }

    fn find_item<'a, T: BytesContainer>(dict: &'a Dictionary, key: &[T]) ->
        Result<Option<&'a Vec<u8>>, ()>
    {
        let head = match key.first() {
            Some(k) => k.container_as_bytes(),
            None => { return Err(()); }
        };
        let tail = key.tail();
        match dict.get(head) {
            Some(&NestedItem::Item(Some(ref v))) if tail.is_empty() =>
                Ok(Some(v)),
            Some(&NestedItem::Map(ref m)) => find_item(m, key.tail()),
            None => Ok(None),
            _ => Err(()),
        }
    }

    fn dig<'a, T: BytesContainer>(map: &'a mut Dictionary, key: &[T]) ->
        Option<&'a mut Option<Vec<u8>>>
    {
        let head = match key.first() {
            Some(k) => k.container_as_bytes().to_vec(),
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
                        _ => { panic!("something is wrong"); }
                    }
                } else {
                    match slot.insert(NestedItem::Map(HashMap::new())) {
                        &mut NestedItem::Map(ref mut m) => m,
                        _ => { panic!("something's wrong"); }
                    }
                }
            }
            _ => { return None; }
        };
        dig(next, tail)
    }
}
