use std::collections::HashSet;
use std::path::{Path, PathBuf};
use primitives::error::{Error, OverflowCheck, RinghopperResult};
use primitives::primitive::{ColorARGBInt, ColorARGBIntBytes};
use crate::data::bitmap::{autodetect_image_extension, Image, load_image_from_path};

/// Iterator for loose color plates.
///
/// Loose color plates are stored where each bitmap is a separate image file, with the name format
/// of `s<sequence>_<bitmap>`.
///
/// Empty sequences are not allowed in this format; the iterator will immediately end if it runs
/// into a sequence that doesn't exist.
///
/// Additionally, only 65536 sequences are allowed, as well as 65536 bitmaps per sequence.
///
pub struct LooseColorPlateIterator<'a> {
    sequence: u16,
    bitmap: u16,
    directory: &'a Path,
    done: bool
}

impl<'a> LooseColorPlateIterator<'a> {
    /// Iterate on the directory.
    ///
    /// Returns `None` if the directory does not exist.
    pub fn interate_on<P: AsRef<Path>>(directory: &'a P) -> Option<LooseColorPlateIterator<'a>> {
        let directory = directory.as_ref();
        if !directory.is_dir() {
            return None
        }

        Some(Self {
            sequence: 0,
            bitmap: 0,
            directory,
            done: false
        })
    }
}

impl<'a> Iterator for LooseColorPlateIterator<'a> {
    type Item = (PathBuf, u16, u16);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None
        }

        let format_path = |data: &Path, sequence: u16, bitmap: u16| -> Option<(PathBuf, u16, u16)> {
            autodetect_image_extension(&data.join(format!("s{sequence}_{bitmap}")))
                .map(|path| (path, sequence, bitmap))
        };

        let advance_sequence = |what: &mut LooseColorPlateIterator| -> Option<()> {
            if what.sequence == u16::MAX {
                what.done = true;
                return None;
            }
            what.sequence += 1;
            what.bitmap = 0;
            Some(())
        };

        let advance_image = |what: &mut LooseColorPlateIterator| -> Option<()> {
            if what.bitmap == u16::MAX {
                return advance_sequence(what);
            }
            what.bitmap += 1;
            return Some(());
        };

        match format_path(self.directory, self.sequence, self.bitmap) {
            Some(n) => {
                advance_image(self);
                return Some(n);
            }
            None => {
                advance_sequence(self)?;
                let next = format_path(self.directory, self.sequence, self.bitmap);
                if next.is_none() {
                    self.done = true;
                }
                else {
                    advance_image(self)?;
                }
                next
            }
        }
    }
}

fn parse_padding_file(padding_metadata: &Path) -> RinghopperResult<ColorARGBInt> {
    let f = std::fs::read(padding_metadata).map_err(|e| Error::FailedToReadFile(padding_metadata.to_path_buf(), e))?;
    let padding_color = std::str::from_utf8(&f)
        .map_err(|_| Error::Other(format!("{padding_metadata:?} is not valid UTF-8")))?
        .to_ascii_uppercase();

    let padding_color = padding_color.trim();
    if padding_color.is_empty() {
        return Err(Error::Other(format!("{padding_metadata:?} is empty")));
    }

    let (octothorpe, color) = padding_color.split_at(1);

    if octothorpe != "#" || (color.len() != 6 && color.len() != 3) {
        return Err(Error::Other(format!("{padding_metadata:?} is not in the format #RRGGBB or #RGB")));
    }

    let colors: [char; 6] = if color.len() == 3 {
        let mut colors_iterator = color.chars();
        let r = colors_iterator.next().unwrap();
        let g = colors_iterator.next().unwrap();
        let b = colors_iterator.next().unwrap();
        [r,r,g,g,b,b]
    }
    else {
        let mut colors_iterator = color.chars();
        [
            colors_iterator.next().unwrap(),
            colors_iterator.next().unwrap(),
            colors_iterator.next().unwrap(),
            colors_iterator.next().unwrap(),
            colors_iterator.next().unwrap(),
            colors_iterator.next().unwrap(),
        ]
    };

    for c in colors {
        if !c.is_ascii_hexdigit() {
            return Err(Error::Other(format!("{padding_metadata:?} contains non-hexadecimal characters")));
        }
    }

    let mut channel_iterator = colors.chunks(2).map(|c| {
        ((c[0].to_digit(16).unwrap() << 4) | (c[1].to_digit(16).unwrap())) as u8
    });

    let colors = ColorARGBIntBytes {
        alpha: 255,
        red: channel_iterator.next().unwrap(),
        green: channel_iterator.next().unwrap(),
        blue: channel_iterator.next().unwrap(),
    };

    Ok(ColorARGBInt::from(colors))
}

/// Read a loose color plate into a full color plate.
///
/// Returns an `Err` if an error occurred.
pub fn make_color_plate_from_loose(data_dir: &Path) -> RinghopperResult<Image> {
    let padding_metadata = data_dir.join("padding.txt");
    let padding = if padding_metadata.is_file() {
        Some(parse_padding_file(&padding_metadata)?)
    }
    else {
        None
    };

    let iterator = LooseColorPlateIterator::interate_on(&data_dir)
        .ok_or_else(|| Error::Other(format!("{data_dir:?} not found")))?;

    let mut sequences: Vec<Vec<Image>> = Vec::new();

    for (path, sequence, _) in iterator {
        if sequence as usize >= sequences.len() {
            sequences.resize(sequence as usize + 1, Vec::new());
        }
        sequences.last_mut().unwrap().push(load_image_from_path(path)?);
    }

    if sequences.is_empty() {
        return Err(Error::Other(format!("{data_dir:?} is empty")))
    }

    // Calculate dimensions
    let mut width = 4usize;
    let mut height = 2usize;
    let mut heights = Vec::with_capacity(sequences.len());

    let mut all_pixels = HashSet::new();

    for s in &sequences {
        let mut current_width = 0usize;
        let mut current_height = 0usize;

        for i in s {
            current_width = current_width.add_overflow_checked(i.width)?.add_overflow_checked(2)?; // 2 pixels for padding on left and right side of each image
            current_height = current_height.max(i.height);

            for pixel in &i.data {
                let convertified = ColorARGBIntBytes {
                    alpha: 255,
                    ..ColorARGBIntBytes::from(*pixel)
                };
                all_pixels.insert(ColorARGBInt::from(convertified));
            }
        }

        current_height = current_height.add_overflow_checked(3)?;

        width = width.max(current_width);
        height = height.add_overflow_checked(current_height)?;
        heights.push(current_height);
    }

    height -= 1;

    let mut background = ColorARGBInt::from(ColorARGBIntBytes { alpha: 255, red: 0, green: 0, blue: 255 });
    let mut sequence_divider = ColorARGBInt::from(ColorARGBIntBytes { alpha: 255, red: 255, green: 0, blue: 255 });

    let contains_pixel = |color: ColorARGBInt| all_pixels.contains(&color);

    let find_first_available_pixel = |excluding: [Option<ColorARGBInt>; 2], white_to_black: bool| -> Option<ColorARGBInt> {
        for red in 0..=u8::MAX {
            for green in 0..=u8::MAX {
                'blue: for blue in 0..=u8::MAX {
                    let color = if white_to_black {
                        ColorARGBIntBytes {
                            red: 255 - red, green: 255 - green, blue: 255 - blue, alpha: 255
                        }
                    }
                    else {
                        ColorARGBIntBytes {
                            red, green, blue, alpha: 255
                        }
                    };
                    let color = ColorARGBInt::from(color);
                    for i in excluding {
                        if i.is_some_and(|i| i == color) {
                            continue 'blue
                        }
                    }
                    if !contains_pixel(color) {
                        return Some(color)
                    }
                }
            }
        }
        None
    };

    if padding.is_some_and(|p| p == background) || contains_pixel(background) {
        background = match find_first_available_pixel([padding, Some(sequence_divider)], true) {
            Some(n) => n,
            None => return Err(Error::Other("cannot find background color to use: full 8-bit RGB spectrum is exhausted".to_owned()))
        };
    }

    if padding.is_some_and(|p| p == sequence_divider) || contains_pixel(sequence_divider) {
        sequence_divider = match find_first_available_pixel([padding, Some(background)], false) {
            Some(n) => n,
            None => return Err(Error::Other("cannot find sequence divider color to use: full 8-bit RGB spectrum is exhausted".to_owned()))
        };
    }

    let mut full_color_plate = Image {
        width,
        height,
        data: vec![background; width.mul_overflow_checked(height)?]
    };

    // Bake the header for the color plate
    full_color_plate.data[1] = sequence_divider;
    if let Some(p) = padding {
        full_color_plate.data[2] = p;
    }

    // Now bake each image
    let mut y = 3;
    for seq_index in 0..sequences.len() {
        full_color_plate.data[(y-2)*width..(y-1)*width].fill(sequence_divider);
        let mut x = 1;

        for i in &sequences[seq_index] {
            let mut data = i.data.iter();
            for py in (0..i.height).map(|py| py + y) {
                for px in (0..i.width).map(|px| px + x) {
                    full_color_plate.data[px + py * width] = *data.next().unwrap();
                }
            }

            x += i.width + 2;
        }

        y += heights[seq_index];
    }

    Ok(full_color_plate)
}
