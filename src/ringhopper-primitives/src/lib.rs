//! Contains all of the primitives, tag definitions, and parsing code for Ringhopper.

pub extern crate byteorder;

pub mod primitive;
pub mod error;
pub mod parse;
pub mod tag;
pub mod accessor;
pub mod crc32;
