use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use definitions::ScenarioType;
use primitives::engine::Engine;
use primitives::primitive::TagPath;
use primitives::tag::PrimaryTagStructDyn;
use crate::tag::tree::TagTree;

#[derive(Clone, Default)]
pub struct TagResult {
    pub pedantic_warnings: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>
}

impl TagResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn combine(&mut self, mut other: TagResult) {
        self.pedantic_warnings.append(&mut other.pedantic_warnings);
        self.warnings.append(&mut other.warnings);
        self.errors.append(&mut other.errors);
    }
}

pub(crate) struct ScenarioContext<T: TagTree + Send + Sync> {
    pub scenario: Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>,
    pub hud_globals: Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>,
    pub scenario_type: ScenarioType,
    pub engine: &'static Engine,
    pub tag_tree: Arc<T>,
    pub results: Mutex<HashMap<TagPath, TagResult>>
}
