use std::path::Path;
use ringhopper::error::{Error, RinghopperResult};

pub fn read_file<P: AsRef<Path>>(path: P) -> RinghopperResult<Vec<u8>> {
    let path = path.as_ref();
    std::fs::read(path).map_err(|e| Error::FailedToReadFile(path.to_owned(), e))
}
