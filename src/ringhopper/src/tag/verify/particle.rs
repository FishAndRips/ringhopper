use primitives::{primitive::TagPath, tag::PrimaryTagStructDyn};
use ringhopper_structs::Particle;

use crate::tag::tree::TagTree;

use super::{ScenarioContext, TagResult};

pub fn verify_particle<T: TagTree + Send + Sync + 'static>(tag: &dyn PrimaryTagStructDyn, _path: &TagPath, _context: &ScenarioContext<T>, result: &mut TagResult) {
    let particle: &Particle = tag.as_any().downcast_ref().unwrap();

    if ((particle.fade_start_size == 0.0) ^ (particle.fade_end_size == 0.0)) && particle.fade_start_size != 5.0 && particle.fade_end_size != 4.0 {
        let (zero, defaulted, value) = if particle.fade_start_size == 0.0 {
            ("start", "end", particle.fade_end_size)
        }
        else {
            ("end", "start", particle.fade_start_size)
        };

        result.warnings.push(format!("fade {zero} size is zero, thus fade {defaulted} size's value ({value}) will be overwritten"))
    }
}
