use std::collections::HashMap;
use definitions::*;
use primitives::primitive::{TagPath, TagReference};
use primitives::tag::PrimaryTagStructDyn;
use crate::tag::dependency::{get_tag_dependencies_for_block, recursively_get_dependencies_for_tag};
use crate::tag::tree::TagTree;
use crate::tag::tree::MockTagTree;

fn generate_test_tag_tree() -> MockTagTree {
    let mut weapon_tag = Weapon::default();
    weapon_tag.item.object.model = TagReference::Set(TagPath::from_path("weapons\\myweapon\\myweapon.model").unwrap());
    weapon_tag.item.object.collision_model = TagReference::Set(TagPath::from_path("weapons\\myweapon\\myweapon.model_collision_geometry").unwrap());
    weapon_tag.first_person_model = TagReference::Set(TagPath::from_path("weapons\\myweapon\\fp\\fp.model").unwrap());
    weapon_tag.first_person_animations = TagReference::Set(TagPath::from_path("weapons\\myweapon\\fp\\fp.model_animations").unwrap());
    let mut weapon_trigger = WeaponTrigger::default();
    weapon_trigger.projectile = TagReference::Set(TagPath::from_path("weapons\\myweapon\\myweapon.projectile").unwrap());
    weapon_tag.triggers.items = vec![weapon_trigger];

    let mut model_tag = Model::default();
    model_tag.shaders.items = vec![
        ModelShaderReference {
            shader: TagReference::Set(TagPath::from_path("weapons\\myweapon\\shaders\\shader.shader_model").unwrap()),
            ..Default::default()
        }
    ];

    let mut shader_model = ShaderModel::default();
    shader_model.maps.base_map = TagReference::Set(TagPath::from_path("weapons\\myweapon\\bitmaps\\shader.bitmap").unwrap());

    let mut items: HashMap<String, Option<Box<dyn PrimaryTagStructDyn>>> = HashMap::new();
    items.insert("weapons\\myweapon\\myweapons.weapon".to_owned(), Some(Box::new(weapon_tag)));
    items.insert("weapons\\myweapon\\myweapon.model_collision_geometry".to_owned(), Some(Box::new(ModelCollisionGeometry::default())));
    items.insert("weapons\\myweapon\\fp\\fp.model".to_owned(), Some(Box::new(model_tag.clone())));
    items.insert("weapons\\myweapon\\fp\\fp.model_animations".to_owned(), Some(Box::new(ModelAnimations::default())));
    items.insert("weapons\\myweapon\\myweapon.model".to_owned(), Some(Box::new(model_tag)));
    items.insert("weapons\\myweapon\\shaders\\shader.shader_model".to_owned(), Some(Box::new(shader_model)));
    items.insert("weapons\\myweapon\\bitmaps\\shader.bitmap".to_owned(), Some(Box::new(Bitmap::default())));
    items.insert("weapons\\myweapon\\myweapon.projectile".to_owned(), Some(Box::new(Projectile::default())));

    MockTagTree {
        items,
        ..Default::default()
    }
}

#[test]
fn dependencies_single_tag() {
    let test_tree = generate_test_tag_tree();
    let dependencies = get_tag_dependencies_for_block(
        test_tree.open_tag_shared(&TagPath::from_path("weapons\\myweapon\\myweapons.weapon").unwrap()).unwrap().lock().unwrap().as_ref()
    );

    assert_eq!(5, dependencies.len());
}

#[test]
fn dependencies_recursive() {
    let test_tree = generate_test_tag_tree();
    let dependencies = recursively_get_dependencies_for_tag(&TagPath::from_path("weapons\\myweapon\\myweapons.weapon").unwrap(), &test_tree).unwrap();

    assert_eq!(7, dependencies.len());
}
