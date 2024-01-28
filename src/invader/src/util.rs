use std::path::Path;
use cli::CommandLineArgs;
use ringhopper::error::{Error, RinghopperResult};
use ringhopper::tag::tree::VirtualTagDirectory;

pub fn read_file<P: AsRef<Path>>(path: P) -> RinghopperResult<Vec<u8>> {
    let path = path.as_ref();
    std::fs::read(path).map_err(|e| Error::FailedToReadFile(path.to_owned(), e))
}

pub fn get_tags_directory(args: &CommandLineArgs) -> Result<VirtualTagDirectory, String> {
    VirtualTagDirectory::new(args.get_tags().as_slice()).map_err(|e| format!("Invalid tags directory(s): {e}"))
}
