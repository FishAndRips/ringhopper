extern crate ringhopper_structs_codegen;
extern crate ringhopper_primitives;

use ringhopper_primitives::primitive::*;
use ringhopper_primitives::parse::*;
use ringhopper_primitives::error::*;
use ringhopper_primitives::byteorder::ByteOrder;

ringhopper_structs_codegen::generate_ringhopper_structs!();
