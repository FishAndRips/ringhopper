use primitives::{primitive::TagPath, tag::PrimaryTagStructDyn};
use ringhopper_structs::{Bitmap, ParticleSystem, ParticleSystemComplexSpriteRenderMode};

use crate::tag::tree::TagTree;

use super::{bitmap::{verify_bitmap_sequence_index, SequenceType}, ScenarioContext, TagResult};

pub fn verify_particle_system<T: TagTree + Send + Sync + 'static>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, context: &ScenarioContext<T>, result: &mut TagResult) {
    let particle_system: &ParticleSystem = tag.as_any().downcast_ref().unwrap();

    for (t, ptype) in ziperator!(particle_system.particle_types) {
        let required = match ptype.complex_sprite_render_modes {
            ParticleSystemComplexSpriteRenderMode::Simple => 1,
            ParticleSystemComplexSpriteRenderMode::Rotational => 2,
        };

        for (s, state) in ziperator!(ptype.particle_states) {
            let sequence_index = match state.sequence_index {
                Some(n) => n,
                None => continue // TODO: check if this is OK
            };

            let bitmap = match context.open_tag_reference_maybe(&state.bitmaps, result, None) {
                Some(n) => n,
                None => continue // TODO: check if this is OK
            };
            let lock = bitmap.lock().unwrap();
            let bitmap: &Bitmap = lock.as_any().downcast_ref().unwrap();

            for required in 0..required {
                if let Err(e) = verify_bitmap_sequence_index(bitmap, Some(sequence_index + required), 1, SequenceType::Sprite) {
                    result.errors.push(format!("Particle state #{s} of particle type #{t} has an error with its sequence index: {e}"));

                    if required == 1 {
                        result.errors.push(format!("The referenced bitmap must have an additional, valid sequence after its first sequence for render mode `{}`", ptype.complex_sprite_render_modes))
                    }
                }
            }
        }
    }
}
