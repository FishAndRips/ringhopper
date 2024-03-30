use std::env::Args;
use cli::CommandLineParser;
use ringhopper::data::bitmap::plate::make_color_plate_from_loose;
use ringhopper::error::Error;
use ringhopper::primitives::primitive::TagGroup;
use threading::{DisplayMode, do_with_threads, ProcessSuccessType};
use util::make_stdout_logger;

pub fn plate(args: Args, description: &'static str) -> Result<(), String> {
    let parser = CommandLineParser::new(description, "<tag*> [args]")
        .add_tags(false)
        .add_data()
        .add_help()
        .add_cow_tags()
        .add_jobs()
        .set_required_extra_parameters(1)
        .parse(args)?;

    let tag = parser.get_extra()[0].clone();
    do_with_threads(parser.get_virtual_tags_directory(), parser, &tag, Some(TagGroup::Bitmap), (), DisplayMode::ShowAll, make_stdout_logger(), |context, path, _, _| {
        let data_dir = context
            .args
            .get_data()
            .join(path.to_native_path())
            .with_extension("");

        if !data_dir.is_dir() {
            return Ok(ProcessSuccessType::Skipped("no directory to import in data"))
        }

        let result = make_color_plate_from_loose(&data_dir)?;
        let image_file = data_dir.with_extension("tif");
        let tiff = result.to_tiff();

        std::fs::write(&image_file, tiff)
            .map(|_| ProcessSuccessType::Success)
            .map_err(|e| Error::FailedToWriteFile(image_file, e))
    })
}
