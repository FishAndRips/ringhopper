use std::borrow::ToOwned;
use definitions::{GBXModel, Model};
use primitives::error::RinghopperResult;
use primitives::primitive::TagGroup;
use primitives::tag::PrimaryTagStructDyn;

use super::model::ModelFunctions;

pub type ConversionFn = fn(&dyn PrimaryTagStructDyn) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>>;

struct ConvertibleFunctions {
    from: TagGroup,
    to: &'static [(TagGroup, ConversionFn)]
}

const CONVERTIBLE_FUNCTIONS: &'static [ConvertibleFunctions] = &[
    ConvertibleFunctions {
        from: TagGroup::GBXModel,
        to: &[(TagGroup::Model, gbxmodel_to_model)]
    },
    ConvertibleFunctions {
        from: TagGroup::Model,
        to: &[(TagGroup::GBXModel, model_to_gbxmodel)]
    }
];

fn gbxmodel_to_model(tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
    let model: GBXModel = tag.as_any().downcast_ref::<GBXModel>().unwrap().to_owned();
    model.check_indices()?;
    Ok(Box::new(model.convert_to_model()))
}

fn model_to_gbxmodel(tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
    let model: Model = tag.as_any().downcast_ref::<Model>().unwrap().to_owned();
    model.check_indices()?;
    Ok(Box::new(model.convert_to_gbxmodel()))
}

/// Get the tag conversion function.
pub fn get_tag_conversion_fn(from: TagGroup, to: TagGroup) -> Option<ConversionFn> {
    for i in CONVERTIBLE_FUNCTIONS {
        if i.from == from {
            for j in i.to {
                if j.0 == to {
                    return Some(j.1)
                }
            }
        }
    }
    None
}
