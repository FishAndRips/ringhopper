use std::fmt::Display;
use std::path::Path;

pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, String> {
    let path = path.as_ref();
    std::fs::read(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))
}
