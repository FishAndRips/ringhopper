use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use definitions::{Model, ModelAnimations, Weapon};
use primitives::error::RinghopperResult;
use primitives::primitive::{TagGroup, TagPath};
use primitives::tag::PrimaryTagStructDyn;
use tag::tree::{CachingTagTree, CachingTagTreeWriteStrategy, TagTree, TagTreeItem, VirtualTagDirectory};

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

#[test]
fn caching_tag_tree() {
    #[derive(Default)]
    struct MockTagTree {
        items: HashMap<String, Option<Box<dyn PrimaryTagStructDyn>>>,
        get_tag_calls: Mutex<Vec<TagPath>>,
        write_tag_calls: Mutex<Vec<TagPath>>
    }

    impl TagTree for MockTagTree {
        fn open_tag(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
            let b = self.items.get(&path.to_string()).unwrap().as_ref().unwrap().clone_inner();
            self.get_tag_calls.lock().unwrap().push(path.to_owned());
            Ok(b)
        }
        fn files_in_path(&self, _path: &str) -> Option<Vec<TagTreeItem>> {
            unimplemented!()
        }
        fn write_tag(&mut self, path: &TagPath, tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<()> {
            self.write_tag_calls.lock().unwrap().push(path.to_owned());
            self.items.insert(path.to_string(), Some(tag.clone_inner()));
            Ok(())
        }
    }

    fn generate_test_data(write_strategy: CachingTagTreeWriteStrategy) -> CachingTagTree<MockTagTree> {
        let mut tag_tree = MockTagTree::default();
        tag_tree.items.insert("weapons".to_owned(), None);
        tag_tree.items.insert("weapons\\dummy".to_owned(), None);
        tag_tree.items.insert("weapons\\dummy\\fp".to_owned(), None);
        tag_tree.items.insert("weapons\\dummy\\fp\\fp.model".to_owned(), Some(Box::new(Model::default())));
        tag_tree.items.insert("weapons\\dummy\\fp\\fp.model_animations".to_owned(), Some(Box::new(ModelAnimations::default())));
        tag_tree.items.insert("weapons\\dummy\\dummy.model".to_owned(), Some(Box::new(Model::default())));
        tag_tree.items.insert("weapons\\dummy\\dummy.weapon".to_owned(), Some(Box::new(Weapon::default())));
        CachingTagTree::new(tag_tree, write_strategy)
    }

    let dummy_model = TagPath::from_path("weapons\\dummy\\dummy.model").unwrap();

    let mut instant = generate_test_data(CachingTagTreeWriteStrategy::Instant);
    let mut tag_data = instant.open_tag(&TagPath::from_path("weapons\\dummy\\dummy.model").unwrap()).unwrap();
    let model = tag_data.get_mut::<Model>().unwrap();
    model.flags.blend_shared_normals = true;
    instant.write_tag(&dummy_model, model).unwrap();
    assert_eq!(instant.inner().get_tag_calls.lock().unwrap().as_slice(), &[dummy_model.clone()]);
    assert_eq!(instant.inner().write_tag_calls.lock().unwrap().as_slice(), &[dummy_model.clone()]);
    assert!(instant.inner.open_tag(&dummy_model).unwrap().get_ref::<Model>().unwrap().flags.blend_shared_normals);

    let mut manual = generate_test_data(CachingTagTreeWriteStrategy::Manual);
    let mut tag_data = manual.open_tag(&TagPath::from_path("weapons\\dummy\\dummy.model").unwrap()).unwrap();
    let model = tag_data.get_mut::<Model>().unwrap();
    model.flags.blend_shared_normals = true;
    manual.write_tag(&dummy_model, model).unwrap();
    assert_eq!(manual.inner().get_tag_calls.lock().unwrap().as_slice(), &[dummy_model.clone()]);
    assert_eq!(manual.inner().write_tag_calls.lock().unwrap().as_slice(), &[]); // should not have actually been written yet
    assert!(!manual.inner.open_tag(&dummy_model).unwrap().get_ref::<Model>().unwrap().flags.blend_shared_normals);
    assert!(manual.open_tag(&dummy_model).unwrap().get_ref::<Model>().unwrap().flags.blend_shared_normals); // but the tag in the cache should be modified at least
    assert!(manual.commit_all().is_empty());
    assert_eq!(manual.inner().write_tag_calls.lock().unwrap().as_slice(), &[dummy_model.clone()]); // now that we've written it, it should've been saved
    assert!(manual.inner.open_tag(&dummy_model).unwrap().get_ref::<Model>().unwrap().flags.blend_shared_normals); // and it has

    // Editing tags in the cache directly should also work
    manual.get(&dummy_model).unwrap().lock().unwrap().get_mut::<Model>().unwrap().flags.blend_shared_normals = false;
    assert!(!manual.open_tag(&dummy_model).unwrap().get_ref::<Model>().unwrap().flags.blend_shared_normals);
}
