pub extern crate ringhopper_primitives as primitives;
pub extern crate ringhopper_structs as definitions;
pub use primitives::error;

use fixed::types::{I16F48, I32F32};
type FixedPrecision = I32F32;
type FixedPrecisionSmall = I16F48;

macro_rules! fixed_small {
    ($val:expr) => {
        crate::FixedPrecisionSmall::saturating_from_num($val)
    };
}

macro_rules! fixed_med {
    ($val:expr) => {
        crate::FixedPrecision::saturating_from_num($val)
    };
}


pub mod tag;
pub mod map;
pub mod constants;
pub mod data;
