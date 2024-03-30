use std::borrow::ToOwned;
use definitions::{BasicObject, Equipment, Garbage, GBXModel, Item, Model, Scenery, Weapon};
use primitives::error::RinghopperResult;
use primitives::primitive::TagGroup;
use primitives::tag::PrimaryTagStructDyn;
use crate::tag::object::{downcast_base_item, downcast_base_object};

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
    },
    ConvertibleFunctions {
        from: TagGroup::Object,
        to: &[(TagGroup::Scenery, object_to_scenery)]
    },
    ConvertibleFunctions {
        from: TagGroup::Item,
        to: &[
            (TagGroup::Garbage, |obj| Ok(Box::new(item_to_garbage(item_to_item_explicit(obj))))),
            (TagGroup::Weapon, |obj| Ok(Box::new(item_to_weapon(item_to_item_explicit(obj))))),
            (TagGroup::Equipment, |obj| Ok(Box::new(item_to_equipment(item_to_item_explicit(obj)))))
        ]
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

fn object_to_scenery(tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
    let object = downcast_base_object(tag).unwrap();
    Ok(Box::new(Scenery {
        basic_object: BasicObject {
            object: object.to_owned(),
            ..Default::default()
        },
        ..Default::default()
    }))
}

fn item_to_item_explicit(tag: &dyn PrimaryTagStructDyn) -> Item {
    downcast_base_item(tag).unwrap().to_owned()
}

fn item_to_garbage(item: Item) -> Garbage {
    Garbage { item, ..Default::default() }
}

fn item_to_equipment(item: Item) -> Equipment {
    Equipment { item, ..Default::default() }
}

fn item_to_weapon(item: Item) -> Weapon {
    Weapon { item, ..Default::default() }
}

/// Get the tag conversion function.
pub fn get_tag_conversion_fn(from: TagGroup, to: TagGroup) -> Option<ConversionFn> {
    if from == to {
        return None
    }

    for i in CONVERTIBLE_FUNCTIONS {
        if from.full_subgroup_tree().contains(&i.from) {
            for j in i.to {
                if j.0 == to {
                    return Some(j.1)
                }
            }
        }
    }
    None
}
