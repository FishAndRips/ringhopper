use std::io::Cursor;
use primitives::error::{Error, OverflowCheck, RinghopperResult};
use primitives::primitive::{Pixel32, Pixel32Bytes};
use crate::data::bitmap::Image;

impl Image {
    /// Convert the image into a TIFF file.
    pub fn to_tiff(&self) -> Vec<u8> {
        use tiff::encoder::*;

        // Encode into a TIFF
        let mut data = Vec::new();
        let mut encoder = TiffEncoder::new(Cursor::new(&mut data)).unwrap();
        let mut image = encoder.new_image::<colortype::RGBA8>(self.width as u32, self.height as u32).unwrap();

        // Indicate that we use alpha
        image.encoder().write_tag(tiff::tags::Tag::ExtraSamples, &[2u16][..]).unwrap();
        image.rows_per_strip(2).unwrap();

        // Re-encode
        let mut pixels_r8g8b8a8 = Vec::with_capacity(
            self.width.mul_overflow_checked(self.height).unwrap().mul_overflow_checked(4).unwrap()
        );
        for i in &self.data {
            let color: Pixel32Bytes = (*i).into();
            pixels_r8g8b8a8.push(color.red);
            pixels_r8g8b8a8.push(color.green);
            pixels_r8g8b8a8.push(color.blue);
            pixels_r8g8b8a8.push(color.alpha);
        }

        // Write each strip
        let mut idx = 0;
        while image.next_strip_sample_count() > 0 {
            let sample_count = image.next_strip_sample_count() as usize;
            image.write_strip(&pixels_r8g8b8a8[idx..idx+sample_count]).unwrap();
            idx += sample_count;
        }

        // Done
        image.finish().unwrap();

        data
    }

    /// Parse a JPEG-XL image file into an image.
    ///
    /// Returns `Err` if an error occurred.
    pub fn from_jxl(data: &[u8]) -> RinghopperResult<Image> {
        use jxl_oxide::{JxlImage, PixelFormat};

        macro_rules! wrap_jxl_error {
            ($result:expr) => {
                ($result).map_err(|e| Error::Other(format!("jxl-oxide read error: {e}")))
            }
        }

        let jxl = wrap_jxl_error!(JxlImage::builder().read(data))?;
        let render = wrap_jxl_error!(jxl.render_frame(0))?;
        let framebuffer = render.image_all_channels();
        let pixel_bytes : Vec<u8> = framebuffer.buf().iter().map(|f| (f * 255.0) as u8).collect();

        let mut image = Image {
            width: framebuffer.width(),
            height: framebuffer.height(),
            data: Vec::new()
        };

        image.data.reserve_exact(image.width.mul_overflow_checked(image.height)?);

        match jxl.pixel_format() {
            PixelFormat::Gray => {
                for i in pixel_bytes {
                    image.data.push(Pixel32Bytes { alpha: 255, red: i, green: i, blue: i }.into())
                }
            },
            PixelFormat::Graya => {
                for i in pixel_bytes.chunks(2) {
                    image.data.push(Pixel32Bytes { alpha: i[1], red: i[0], green: i[0], blue: i[0] }.into())
                }
            },
            PixelFormat::Rgb => {
                for i in pixel_bytes.chunks(3) {
                    image.data.push(Pixel32Bytes { alpha: 255, red: i[0], green: i[1], blue: i[2] }.into())
                }
            },
            PixelFormat::Rgba => {
                for i in pixel_bytes.chunks(4) {
                    image.data.push(Pixel32Bytes { alpha: i[3], red: i[0], green: i[1], blue: i[2] }.into())
                }
            },
            n => return Err(Error::Other(format!("unsupported jxl pixel format {n:?}")))
        }

        Ok(image)
    }

    /// Parse a TIFF image file into an image.
    ///
    /// Returns `Err` if an error occurred.
    pub fn from_tiff(tiff: &[u8]) -> RinghopperResult<Image> {
        use tiff::decoder::*;
        use tiff::{ColorType, TiffResult};

        macro_rules! wrap_tiff_error {
            ($result:expr) => {
                ($result).map_err(|e| Error::Other(format!("tiff read error: {e}")))
            }
        }

        // Read the image, converting a TIFF error to a Ringhopper error.
        let (raw_pixels, color_type, width, height) = wrap_tiff_error!((|| -> TiffResult<(DecodingResult, ColorType, usize, usize)> {
            let mut decoder = Decoder::new(Cursor::new(&tiff))?;
            let (width,height) = decoder.dimensions()?;
            let image = decoder.read_image()?;
            let color_type = decoder.colortype()?;
            Ok((image, color_type, width as usize, height as usize))
        })())?;

        // Read pixels
        let raw_pixels_vec = match raw_pixels {
            DecodingResult::U8(p) => p,
            _ => return Err(Error::Other("only u8 supported for tiff".to_string()))
        };

        // Read bit depth
        let mut pixels = Vec::with_capacity(width * height);
        let bit_depth = match color_type {
            ColorType::Gray(n) => n,
            ColorType::GrayA(n) => n,
            ColorType::RGB(n) => n,
            ColorType::RGBA(n) => n,
            _ => return Err(Error::Other("only RGB/monochrome supported for tiff".to_string()))
        };
        if bit_depth != 8 {
            return Err(Error::Other("only 8-bit color supported for tiff".to_string()))
        }

        // Convert pixels to ARGB
        let (conversion_function, bytes_per_pixel): (fn (input_bytes: &[u8]) -> Pixel32, usize) = match color_type {
            ColorType::Gray(_) => (|pixels| Pixel32::from_y8(pixels[0]), 1),
            ColorType::GrayA(_) => (|pixels| Pixel32::from_a8y8(((pixels[1] as u16) << 8) | (pixels[0] as u16)), 2),
            ColorType::RGB(_) => (|pixels| Pixel32Bytes { alpha: 255, red: pixels[0], green: pixels[1], blue: pixels[2] }.into(), 3),
            ColorType::RGBA(_) => (|pixels| Pixel32Bytes { alpha: pixels[3], red: pixels[0], green: pixels[1], blue: pixels[2] }.into(), 4),
            _ => unreachable!()
        };
        for i in (0..raw_pixels_vec.len()).step_by(bytes_per_pixel) {
            pixels.push(conversion_function(&raw_pixels_vec[i..]))
        }

        // Assert that we have the correct pixel count
        debug_assert_eq!(width * height, pixels.len());

        // Done!
        Ok(Image { width, height, data: pixels })
    }

    /// Parse a PNG image file into an image.
    ///
    /// Returns `Err` if an error occurred.
    pub fn from_png(png: &[u8]) -> RinghopperResult<Image> {
        use png::*;

        macro_rules! wrap_png_error {
            ($result:expr) => {
                ($result).map_err(|e| Error::Other(format!("png read error: {e}")))
            }
        }

        // Read the image, converting a PNG error to a Ringhopper error.
        let (raw_pixels_vec, color_type, bit_depth, width, height) = wrap_png_error!((|| -> Result<(Vec<u8>, ColorType, BitDepth, usize, usize), DecodingError> {
            let decoder = Decoder::new(Cursor::new(&png));
            let mut reader = decoder.read_info()?;
            let mut raw_pixels = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut raw_pixels)?;
            let color_type = info.color_type;
            let bit_depth = info.bit_depth;
            Ok((raw_pixels, color_type, bit_depth, info.width as usize, info.height as usize))
        })())?;

        // Get the bit depth
        let bit_depth = match bit_depth {
            BitDepth::One => 1,
            BitDepth::Two => 2,
            BitDepth::Four => 4,
            BitDepth::Eight => 8,
            BitDepth::Sixteen => 16
        };
        if bit_depth != 8 {
            return Err(Error::Other("only 8-bit color supported for png".to_string()))
        }

        let mut pixels = Vec::with_capacity(width * height);

        // Convert pixels to ARGB
        let (conversion_function, bytes_per_pixel): (fn (input_bytes: &[u8]) -> Pixel32, usize) = match color_type {
            ColorType::Grayscale => (|pixels| Pixel32::from_y8(pixels[0]), 1),
            ColorType::GrayscaleAlpha => (|pixels| Pixel32::from_a8y8(((pixels[1] as u16) << 8) | (pixels[0] as u16)), 2),
            ColorType::Rgb => (|pixels| Pixel32Bytes { alpha: 255, red: pixels[0], green: pixels[1], blue: pixels[2] }.into(), 3),
            ColorType::Rgba => (|pixels| Pixel32Bytes { alpha: pixels[3], red: pixels[0], green: pixels[1], blue: pixels[2] }.into(), 4),
            _ => return Err(Error::Other("only RGB/monochrome supported for png".to_string()))
        };
        for i in (0..raw_pixels_vec.len()).step_by(bytes_per_pixel) {
            pixels.push(conversion_function(&raw_pixels_vec[i..]))
        }

        // Done!
        Ok(Image { width, height, data: pixels })
    }
}
