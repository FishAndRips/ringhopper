use std::collections::VecDeque;
use std::env::Args;
use std::sync::{Arc, Mutex};
use cli::{CommandLineParser, CommandLineValueType, Parameter};
use ringhopper::tag::tree::{AtomicTagTree, TagTree, VirtualTagsDirectory};
use threading::do_with_threads;

pub fn compare(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag> [args]")
        .add_help()
        .add_custom_parameter(Parameter::new(
            "tags-source",
            'T',
            "Add a tags directory to compare against.",
            "<tags>",
            Some(CommandLineValueType::Path),
            1,
            None,
            true,
            false
        ))
        .add_custom_parameter(Parameter::new(
            "map-source",
            'M',
            "Add a map to compare against.",
            "<map>",
            Some(CommandLineValueType::Path),
            1,
            None,
            true,
            false
        ))
        .set_required_extra_parameters(1)
        .parse(args)?;

    let mut source: VecDeque<Box<dyn TagTree + Send>> = VecDeque::new();
    if let Some(n) = parser.get_custom("tags-source") {
        for i in n {
            let path = i.path();
            let dir = match VirtualTagsDirectory::new(&[path], None) {
                Ok(n) => n,
                Err(e) => return Err(format!("Error with tags directory {}: {e}", path.display()))
            };
            source.push_back(Box::new(dir));
        }
    }
    if let Some(n) = parser.get_custom("map-source") {
        for _ in n {
            return Err("Map comparison is not yet implemented!".to_string());
        }
    }

    if source.len() != 2 {
        return Err("Only two sources are supported.".to_string());
    }

    #[derive(Clone)]
    struct UserData {
        tags: Arc<Mutex<Box<dyn TagTree + Send>>>
    }

    let primary = source.pop_front().unwrap();
    let secondary = AtomicTagTree::new(source.pop_front().unwrap());
    let tag = parser.get_extra()[0].clone();
    do_with_threads(AtomicTagTree::new(primary), parser, &tag, None, Arc::new(Mutex::new(source)), |context, path, user_data| {


        Ok(false)
    })
}
