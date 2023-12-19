
use crate::primitive::*;

pub enum AccessorResult<'a> {
    Accessor(&'a dyn TagDataAccessor),
    Primitive(PrimitiveRef<'a>),
    Size(usize),
    Error(String)
}

pub enum AccessorResultMut<'a> {
    Accessor(&'a mut dyn TagDataAccessor),
    Primitive(PrimitiveRefMut<'a>),
    Size(usize),
    Error(String)
}

pub enum TagDataAccessorType {
    Block,
    Reflexive
}

pub trait TagDataAccessor {
    /// Access a field with a matcher
    fn access(&self, matcher: &str) -> Vec<AccessorResult>;

    /// Mutably access a field with a matcher
    fn access_mut(&mut self, matcher: &str) -> Vec<AccessorResultMut>;

    /// Get the type of accessor
    fn get_type(&self) -> TagDataAccessorType;

    /// Get all available fields
    fn all_fields(&self) -> &'static [&'static str];
}
