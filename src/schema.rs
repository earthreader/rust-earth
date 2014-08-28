pub type SchemaResult<T> = Result<T, SchemaError>;

pub enum SchemaError {
    DescriptorConflict,
    IntegrityError,
    EncodeError,
    DecodeError,
}
