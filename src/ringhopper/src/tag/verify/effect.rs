use primitives::{primitive::{TagGroup, TagPath, TagReference}, tag::PrimaryTagStructDyn};
use ringhopper_structs::Effect;
use crate::tag::tree::TagTree;
use super::{ScenarioContext, TagResult};

pub fn verify_effect<T: TagTree + Send + Sync>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &ScenarioContext<T>, result: &mut TagResult) {
    let effect: &Effect = tag.as_any().downcast_ref().unwrap();

    for (e, event) in (0..effect.events.items.len()).zip(effect.events.items.iter()) {
        for (p, part) in (0..event.parts.items.len()).zip(event.parts.items.iter()) {
            match part._type {
                TagReference::Set(_) => (),

                // Null damage_effect references crash the game
                TagReference::Null(TagGroup::DamageEffect) => result.errors.push(format!("Part #{p} of event #{e} contains a null damage_effect reference. This is invalid, and thus you should remove this or set something.")),

                // This will just do nothing
                TagReference::Null(g) => result.pedantic_warnings.push(format!("Part #{p} of event #{e} contains a null {g} reference, which is a no-op. You can safely remove this."))
            }
        }
        for (p, particle) in (0..event.particles.items.len()).zip(event.particles.items.iter()) {
            if particle.particle_type.is_null() {
                result.pedantic_warnings.push(format!("Particle #{p} of event #{e} contains a null particle reference, which is a no-op. You can safely remove this."));
            }
        }
    }
}
