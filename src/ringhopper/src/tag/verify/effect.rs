use primitives::{primitive::{TagGroup, TagPath, TagReference}, tag::PrimaryTagStructDyn};
use ringhopper_structs::Effect;
use crate::tag::tree::TagTree;
use super::{VerifyContext, VerifyResult};

pub fn verify_effect<T: TagTree>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &mut VerifyContext<T>, result: &mut VerifyResult) {
    let effect: &Effect = tag.as_any().downcast_ref().unwrap();

    for (e, event) in (0..effect.events.items.len()).zip(effect.events.items.iter()) {
        for (p, part) in (0..event.parts.items.len()).zip(event.parts.items.iter()) {
            match part._type {
                TagReference::Set(_) => (),

                // Null damage_effect references crash the game
                TagReference::Null(TagGroup::DamageEffect) => result.errors.push(format!("Part #{p} of event #{e} contains a null damage_effect reference. This is invalid.")),

                // This will just do nothing
                TagReference::Null(g) => result.pedantic_warnings.push(format!("Part #{p} of event #{e} contains a null {g} reference and is thus unused."))
            }
        }
        for (p, particle) in (0..event.particles.items.len()).zip(event.particles.items.iter()) {
            if particle.particle_type.is_null() {
                result.pedantic_warnings.push(format!("Particle #{p} of event #{e} contains a null particle reference and is thus unused."));
            }
        }
    }
}
