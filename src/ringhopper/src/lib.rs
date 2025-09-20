#![allow(mismatched_lifetime_syntaxes)]

pub extern crate ringhopper_primitives as primitives;
pub extern crate ringhopper_structs as definitions;
pub use primitives::error;

pub mod tag;
pub mod map;
pub mod constants;
pub mod data;
