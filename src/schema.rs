
pub type SchemaResult<T> = Result<T, SchemaError>;

#[deriving(Show)]
pub enum SchemaError {
    DescriptorConflict,
    IntegrityError,
    EncodeError,
    DecodeError(String),
}

pub trait Codec<T> {
    fn encode(&self, value: &T, w: &mut Writer) -> SchemaResult<()>;
    fn decode(&self, r: &str) -> SchemaResult<T>;
}

pub trait Mergeable {
    fn merge_entities(self, other: Self) -> Self;
}
