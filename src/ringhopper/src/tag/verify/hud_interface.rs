use std::collections::HashSet;

use definitions::WeaponHUDInterface;
use primitives::dynamic::DynamicTagDataArray;
use primitives::primitive::{TagPath, TagReference};
use primitives::tag::PrimaryTagStructDyn;
use ringhopper_structs::{Bitmap, GrenadeHUDInterface, HUDGlobals, HUDInterfaceMeterElement, HUDInterfaceStaticElement, UnicodeStringList, UnitHUDInterface};
use crate::tag::tree::TagTree;
use crate::tag::verify::VerifyResult;

use super::bitmap::{verify_bitmap_sequence_index, SequenceType};
use super::VerifyContext;

pub fn verify_weapon_hud_interface<T: TagTree>(tag: &dyn PrimaryTagStructDyn, path: &TagPath, context: &mut VerifyContext<T>, result: &mut VerifyResult) {
    let weapon_hud_interface: &WeaponHUDInterface = tag.as_any().downcast_ref().unwrap();

    macro_rules! check_sequence_index_for_2d_thing {
        ($outer_reflexive:tt, $outer_reflexive_bitmap:tt, $inner_reflexive:tt, $name:expr) => {
            for (oi, outer) in ziperator!(weapon_hud_interface.$outer_reflexive) {
                let bitmap = match context.open_tag_reference_maybe(&outer.$outer_reflexive_bitmap, result, None) {
                    Some(n) => n,
                    None => continue
                };

                let mut bitmap = bitmap.lock().unwrap();
                let bitmap = bitmap.as_any_mut().downcast_mut::<Bitmap>().unwrap();

                for (ii, inner) in ziperator!(outer.$inner_reflexive) {
                    match verify_bitmap_sequence_index(bitmap, path, inner.sequence_index, 1, SequenceType::Any) {
                        Ok(_) => (),
                        Err(e) => result.errors.push(format!("Overlay #{ii} of {} {oi} has an error: {e}", $name))
                    }
                }
            }
        };
    }

    check_sequence_index_for_2d_thing!(crosshairs, crosshair_bitmap, crosshair_overlays, "crosshair");
    check_sequence_index_for_2d_thing!(overlay_elements, overlay_bitmap, overlays, "overlay element");

    for (i, element) in ziperator!(weapon_hud_interface.meter_elements) {
        check_meter_element(&element.properties, path, context, result, || format!("Meter element #{i}"))
    }

    for (i, element) in ziperator!(weapon_hud_interface.static_elements) {
        check_static_element(&element.properties, path, context, result, || format!("Static element #{i}"))
    }

    verify_no_infinite_loop(weapon_hud_interface, path, context, result);
}

pub fn verify_unit_hud_interface<T: TagTree>(tag: &dyn PrimaryTagStructDyn, path: &TagPath, context: &mut VerifyContext<T>, result: &mut VerifyResult) {
    let unit_hud_interface: &UnitHUDInterface = tag.as_any().downcast_ref().unwrap();

    check_static_element(&unit_hud_interface.hud_background, path, context, result, || "HUD background".to_string());
    check_static_element(&unit_hud_interface.shield_panel_background, path, context, result, || "Shield panel background".to_string());
    check_static_element(&unit_hud_interface.health_panel_background, path, context, result, || "Health panel background".to_string());
    check_meter_element(&unit_hud_interface.shield_panel_meter.hudinterface_meter_element, path, context, result, || "Shield panel meter".to_string());
    check_meter_element(&unit_hud_interface.health_panel_meter.hudinterface_meter_element, path, context, result, || "Health panel meter".to_string());
    check_static_element(&unit_hud_interface.motion_sensor_background, path, context, result, || "Motion sensor background".to_string());
    check_static_element(&unit_hud_interface.motion_sensor_foreground, path, context, result, || "Motion sensor foreground".to_string());

    for (i, element) in ziperator!(unit_hud_interface.auxiliary_elements.overlays) {
        check_static_element(&element.properties, path, context, result, || format!("Auxiliary overlay #{i}"));
    }

    for (i, element) in ziperator!(unit_hud_interface.auxiliary_elements.meters) {
        check_static_element(&element.background, path, context, result, || format!("Auxiliary meter #{i}'s background"));
        check_meter_element(&element.meter, path, context, result, || format!("Auxiliary meter #{i}'s meter"));
    }
}

pub fn verify_grenade_hud_interface<T: TagTree>(tag: &dyn PrimaryTagStructDyn, path: &TagPath, context: &mut VerifyContext<T>, result: &mut VerifyResult) {
    let grenade_hud_interface: &GrenadeHUDInterface = tag.as_any().downcast_ref().unwrap();

    check_static_element(&grenade_hud_interface.background, path, context, result, || "Background".to_string());
    check_static_element(&grenade_hud_interface.total_grenades_background, path, context, result, || "Total grenades background".to_string());

    match context.open_tag_reference_maybe(&grenade_hud_interface.total_grenades_overlays.bitmap, result, None) {
        Some(bitmap) => {
            let mut bitmap = bitmap.lock().unwrap();
            let bitmap = bitmap.as_any_mut().downcast_mut::<Bitmap>().unwrap();

            for (i, overlay) in ziperator!(grenade_hud_interface.total_grenades_overlays.overlays) {
                match verify_bitmap_sequence_index(bitmap, path, overlay.sequence_index, 1, SequenceType::Any) {
                    Ok(_) => (),
                    Err(e) => result.errors.push(format!("Total grenade overlay #{i} has an error: {e}"))
                }
            }
        },
        None => ()
    };
}

pub fn verify_hud_globals<T: TagTree>(tag: &dyn PrimaryTagStructDyn, path: &TagPath, context: &mut VerifyContext<T>, result: &mut VerifyResult) {
    let hud_globals: &HUDGlobals = tag.as_any().downcast_ref().unwrap();

    match context.open_tag_reference_maybe(&hud_globals.messaging_parameters.item_message_text, result, None) {
        Some(item_message_text) => {
            let mut item_message_text = item_message_text.lock().unwrap();
            item_message_text.metadata_mut().verification_dependants.insert(path.to_owned());

            let item_message_text = item_message_text.as_any_mut().downcast_mut::<UnicodeStringList>().unwrap();

            let out_of_bounds = |what: Option<u16>| what.is_some_and(|index| {
                index as usize >= item_message_text.strings.len()
            });

            if out_of_bounds(hud_globals.more_hud_crap.checkpoint_begin_text) {
                result.errors.push("Checkpoint begin text index is out-of-bounds".to_owned());
            }
            if out_of_bounds(hud_globals.more_hud_crap.checkpoint_end_text) {
                result.errors.push("Checkpoint end text index is out-of-bounds".to_owned());
            }
            if out_of_bounds(hud_globals.more_hud_crap.loading_begin_text) {
                result.errors.push("Loading begin text index is out-of-bounds".to_owned());
            }
            if out_of_bounds(hud_globals.more_hud_crap.loading_end_text) {
                result.errors.push("Loading end text index is out-of-bounds".to_owned());
            }
        },
        None => ()
    };
}

fn check_static_element<T: TagTree, N: FnOnce() -> String>(element: &HUDInterfaceStaticElement, path: &TagPath, context: &mut VerifyContext<T>, result: &mut VerifyResult, name: N) {
    let bitmap = match context.open_tag_reference_maybe(&element.interface_bitmap, result, None) {
        Some(n) => n,
        None => return
    };
    let mut bitmap = bitmap.lock().unwrap();
    let bitmap = bitmap.as_any_mut().downcast_mut::<Bitmap>().unwrap();
    match verify_bitmap_sequence_index(bitmap, path, element.sequence_index, 1, SequenceType::Any) {
        Ok(_) => (),
        Err(e) => result.errors.push(format!("{} has an error: {e}", name()))
    }
}

fn check_meter_element<T: TagTree, N: FnOnce() -> String>(element: &HUDInterfaceMeterElement, path: &TagPath, context: &mut VerifyContext<T>, result: &mut VerifyResult, name: N) {
    let bitmap = match context.open_tag_reference_maybe(&element.meter_bitmap, result, None) {
        Some(n) => n,
        None => return
    };
    let mut bitmap = bitmap.lock().unwrap();
    let bitmap = bitmap.as_any_mut().downcast_mut::<Bitmap>().unwrap();
    match verify_bitmap_sequence_index(bitmap, path, element.sequence_index, 1, SequenceType::Any) {
        Ok(_) => (),
        Err(e) => result.errors.push(format!("{} has an error: {e}", name()))
    }
}

fn verify_no_infinite_loop<T: TagTree>(base: &WeaponHUDInterface, path: &TagPath, context: &mut VerifyContext<T>, result: &mut VerifyResult) {
    let mut all_references = HashSet::new();
    all_references.insert(path.to_owned());
    let mut child_ref = base.child_hud.to_owned();
    while let TagReference::Set(n) = &child_ref {
        if all_references.contains(n) {
            result.errors.push(format!("Infinite loop detected (loop start found at {n})"));
            break
        }

        child_ref = match context.open_tag_reference_maybe(&child_ref, result, None) {
            Some(n) => {
                let mut lock = n.lock().unwrap();
                lock.metadata_mut().verification_dependants.insert(path.to_owned());

                let child = lock.as_any().downcast_ref::<WeaponHUDInterface>().unwrap();
                child.child_hud.clone()
            },
            None => break // failed to load
        }
    }
}
