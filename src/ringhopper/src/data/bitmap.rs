use std::path::{Path, PathBuf};
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::ColorARGBInt;

mod parse;
pub mod plate;

/// Represents a collection of pixels, with a width and height.
#[derive(Clone, Default)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub data: Vec<ColorARGBInt>
}

const IMAGE_LOADING_FUNCTIONS: &'static [(&'static str, fn(data: &[u8]) -> RinghopperResult<Image>)] = &[
    ("jxl", Image::from_jxl),
    ("tif", Image::from_tiff),
    ("tiff", Image::from_tiff),
    ("png", Image::from_png),
];

/// Load an image at the given path.
///
/// Returns `Err` if the image is unsupported or an error occurred.
pub fn load_image_from_path<P: AsRef<Path>>(path: P) -> RinghopperResult<Image> {
    let path = path.as_ref();
    let extension = path
        .extension()
        .ok_or_else(|| Error::Other(format!("no extension for image `{path:?}`")))?
        .to_str()
        .ok_or_else(|| Error::Other(format!("unreadable image extension for `{path:?}`")))?;

    let data = std::fs::read(path)
        .map_err(|e| Error::FailedToReadFile(path.to_path_buf(), e))?;

    for (load_ext, loader) in IMAGE_LOADING_FUNCTIONS {
        if load_ext.eq_ignore_ascii_case(extension) {
            return loader(&data)
        }
    }

    Err(Error::Other(format!("unrecognized image extension {extension:?}")))
}

/// Find an image at the given path, appending the extension to it if a supported image is found.
///
/// Returns `None` if no supported images were found at that path.
pub fn autodetect_image_extension(file: &Path) -> Option<PathBuf> {
    let mut p = file.to_path_buf();

    // If the filename contains a dot, prevent replacing the dot.
    if p.extension().is_some() {
        p.set_extension(p.extension().unwrap().to_str()?.to_string() + ".jxl");
    }

    for (extension, _) in IMAGE_LOADING_FUNCTIONS {
        p.set_extension(extension);
        if p.is_file() {
            return Some(p)
        }
    }
    None
}
