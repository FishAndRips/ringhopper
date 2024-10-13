use std::env::Args;
use crate::cli::CommandLineParser;
use ringhopper::definitions::ScenarioStructureBSP;
use ringhopper::error::RinghopperResult;
use ringhopper::primitives::primitive::{TagGroup, TagPath};
use ringhopper::tag::model::downcast_model_mut;
use ringhopper::tag::scenario_structure_bsp::recompress_scenario_structure_bsp_vertices;
use ringhopper::tag::tree::{TagTree, VirtualTagsDirectory};
use crate::threading::{DisplayMode, do_with_threads, ProcessSuccessType};
use crate::util::make_stdout_logger;

pub fn recompress_vertices(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag.group*> [args]")
        .add_tags(false)
        .add_help()
        .add_cow_tags()
        .add_jobs()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag = parser.get_extra()[0].clone();
    do_with_threads(parser.get_virtual_tags_directory(), parser, &tag, None, (), DisplayMode::ShowAll, make_stdout_logger(), |context, path, _, _| {
        match path.group() {
            TagGroup::Model => handle_model_tag(path, &mut context.tags_directory),
            TagGroup::GBXModel => handle_model_tag(path, &mut context.tags_directory),
            TagGroup::ScenarioStructureBSP => handle_bsp_tag(path, &mut context.tags_directory),
            _ => Ok(ProcessSuccessType::Ignored)
        }
    })
}

fn handle_model_tag(path: &TagPath, dir: &mut VirtualTagsDirectory) -> RinghopperResult<ProcessSuccessType> {
    let mut tag = dir.open_tag_copy(path)?;
    let model = downcast_model_mut(tag.as_mut()).unwrap();
    if model.recompress_vertices() {
        ProcessSuccessType::wrap_write_result(dir.write_tag(path, tag.as_ref()))
    }
    else {
        Ok(ProcessSuccessType::Skipped("cannot recompress vertices for tag"))
    }
}

fn handle_bsp_tag(path: &TagPath, dir: &mut VirtualTagsDirectory) -> RinghopperResult<ProcessSuccessType> {
    let mut tag = dir.open_tag_copy(path)?;
    let bsp = tag.as_any_mut().downcast_mut::<ScenarioStructureBSP>().unwrap();
    for lm in &mut bsp.lightmaps {
        for mat in &mut lm.materials {
            if mat.uncompressed_vertices.bytes.is_empty() && !mat.compressed_vertices.bytes.is_empty() {
                return Ok(ProcessSuccessType::Skipped("missing uncompressed vertices"))
            }
            mat.compressed_vertices.bytes.clear();
        }
    }

    recompress_scenario_structure_bsp_vertices(bsp)?;
    ProcessSuccessType::wrap_write_result(dir.write_tag(path, tag.as_ref()))
}
