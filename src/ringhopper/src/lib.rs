pub extern crate ringhopper_primitives as primitives;
pub extern crate ringhopper_structs as definitions;
pub use primitives::error;

type FixedPrecision = fixed::types::I64F64;

macro_rules! fixed_med {
    ($val:expr) => {
        crate::FixedPrecision::saturating_from_num($val)
    };
}


pub mod tag;
pub mod map;
pub mod constants;
pub mod data;
