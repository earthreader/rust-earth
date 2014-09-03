
pub type SchemaResult<T> = Result<T, SchemaError>;

#[deriving(Show)]
pub enum SchemaError {
    DescriptorConflict,
    IntegrityError,
    EncodeError,
    DecodeError(String),
}


}

pub trait Mergeable {
    fn merge_entities(self, other: Self) -> Self;
}


}
