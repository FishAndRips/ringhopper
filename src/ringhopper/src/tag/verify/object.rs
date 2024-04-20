use definitions::{Bitmap, Weapon, WeaponHUDInterface};
use primitives::dynamic::DynamicTagDataArray;
use primitives::primitive::TagPath;
use primitives::tag::PrimaryTagStructDyn;
use ringhopper_structs::{GBXModel, HUDGlobals, Model, ModelAnimations, UnicodeStringList, WeaponHUDInterfaceCrosshairType};
use crate::tag::object::downcast_base_object;
use crate::tag::tree::TagTree;
use crate::tag::verify::VerifyResult;
use super::bitmap::{verify_bitmap_sequence_index, SequenceType};
use super::VerifyContext;

const IGNORED_MODEL_NODE_LIST_CHECKSUM: i32 = 0;

pub fn verify_object<T: TagTree + Send + Sync + 'static>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, context: &VerifyContext<T>, result: &mut VerifyResult) {
    let object = downcast_base_object(tag).unwrap();

    let model = context.open_tag_reference_maybe(&object.model, result, None);

    if object.model.is_null() {
        if !object.animation_graph.is_null() {
            result.errors.push("Object has no model but has an animation graph.".to_string());
        }
        if !object.collision_model.is_null() {
            result.errors.push("Object has no model but has a collision model.".to_string());
        }
    }

    if let Some(n) = model {
        let model = n.lock().unwrap();
        let mut verified = false;

        macro_rules! verify_model {
            ($group:ty) => {
                if let Some(model) = model.as_any().downcast_ref::<$group>() {
                    if model.node_list_checksum != IGNORED_MODEL_NODE_LIST_CHECKSUM {
                        if let Some(anim) = context.open_tag_reference_maybe(&object.animation_graph, result, None) {
                            let anim = anim.lock().unwrap();
                            let anim = anim.as_any().downcast_ref::<ModelAnimations>().unwrap();

                            for i in &anim.animations {
                                if i.node_list_checksum != IGNORED_MODEL_NODE_LIST_CHECKSUM && i.node_list_checksum != model.node_list_checksum {
                                    result.errors.push("Object has mismatched model and animations (node list checksum is nonzero and does not match for one or more animations).".to_string());
                                    break
                                }
                            }
                        }
                    }
                    verified = true;
                }
            };
        }

        verify_model!(Model);
        verify_model!(GBXModel);

        debug_assert!(verified);
    }

    let hud_globals_lock = context.hud_globals.clone();
    let hud_globals = hud_globals_lock.lock().unwrap();
    let hud_globals = hud_globals.as_any().downcast_ref::<HUDGlobals>().unwrap();
    if let Some(r) = context.open_tag_reference_maybe(&hud_globals.messaging_parameters.item_message_text, result, None) {
        let item_message_text = r.lock().unwrap();
        let index = object.hud_text_message_index;
        let max = item_message_text.as_any().downcast_ref::<UnicodeStringList>().unwrap().strings.len();
        if max <= index as usize {
            result.errors.push(format!("HUD text message index ({index}) is out-of-bounds (only {max} strings in {})", hud_globals.messaging_parameters.item_message_text.path().unwrap()));
        }
    }
}

pub fn verify_weapon<T: TagTree + Send + Sync + 'static>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, context: &VerifyContext<T>, result: &mut VerifyResult) {
    let weapon: &Weapon = tag.as_any().downcast_ref().unwrap();

    let zoom_levels = weapon.zoom_levels as usize;
    if zoom_levels > 0 {
        if let Some(r) = context.open_tag_reference_maybe(&weapon.hud_interface, result, None) {
            let hud = r.lock().unwrap();
            let weapon_hud_interface: &WeaponHUDInterface = hud.as_any().downcast_ref().unwrap();
            for (crosshair_index, crosshair) in (0..weapon_hud_interface.crosshairs.len()).zip(&weapon_hud_interface.crosshairs) {
                if crosshair.crosshair_type != WeaponHUDInterfaceCrosshairType::ZoomOverlay {
                    continue
                }

                let b = match context.open_tag_reference_maybe(&crosshair.crosshair_bitmap, result, None) {
                    Some(n) => n,
                    None => continue
                };

                let mut lock = b.lock().unwrap();
                let bitmap: &mut Bitmap = lock.as_any_mut().downcast_mut().unwrap();

                for (overlay_index, overlay) in (0..crosshair.crosshair_overlays.len()).zip(&crosshair.crosshair_overlays) {
                    if let Err(e) = verify_bitmap_sequence_index(
                        bitmap,
                        overlay.sequence_index,
                        zoom_levels,
                        match overlay.flags.not_a_sprite { true => SequenceType::Bitmap, false => SequenceType::Sprite }) {
                        result.errors.push(format!("HUD overlay #{overlay_index} of crosshair #{crosshair_index} has an error: {e}"));
                    }
                }
            }
        }
    }
}
