use std::path::Path;
use primitives::primitive::{TagGroup, TagPath};
use tag::tree::{TagTree, VirtualTagDirectory};

#[test]
fn test_tag_tree_traversal() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tag")
        .join("tree")
        .join("tags");

    let tag_directory = VirtualTagDirectory::new(&[path]).unwrap();

    let root = tag_directory.root();
    assert!(root.is_directory());
    let root_contents = root.files().unwrap();
    assert_eq!(1, root_contents.len());

    let weapons = &root_contents[0];
    assert!(weapons.is_directory());
    let weapons_contents = weapons.files().unwrap();
    assert_eq!(1, weapons_contents.len());

    let dummy = &weapons_contents[0];
    assert!(dummy.is_directory());
    let dummy_contents = dummy.files().unwrap();
    assert_eq!(3, dummy_contents.len()); // excludes ".txt"
    let model = dummy_contents.iter().find(|&p| p.tag_path() == Some(TagPath::from_path("weapons\\dummy\\dummy.model").unwrap())).expect("should exist");
    assert!(model.is_tag() && model.tag_group().unwrap() == TagGroup::Model);
    let weapon = dummy_contents.iter().find(|&p| p.tag_path() == Some(TagPath::from_path("weapons\\dummy\\dummy.weapon").unwrap())).expect("should exist");
    assert!(weapon.is_tag() && weapon.tag_group().unwrap() == TagGroup::Weapon);

    let fp = dummy_contents.iter().find(|f| f.is_directory()).expect("should also exist");
    let fp_contents = fp.files().unwrap();
    assert_eq!(2, fp_contents.len());

    let fp_model = fp_contents.iter().find(|&p| p.tag_path() == Some(TagPath::from_path("weapons\\dummy\\fp\\fp.model").unwrap())).expect("should exist");
    assert!(fp_model.is_tag() && fp_model.tag_group().unwrap() == TagGroup::Model);

    let fp_animations = fp_contents.iter().find(|&p| p.tag_path() == Some(TagPath::from_path("weapons\\dummy\\fp\\fp.model_animations").unwrap())).expect("should exist");
    assert!(fp_animations.is_tag() && fp_animations.tag_group().unwrap() == TagGroup::ModelAnimations);
}
