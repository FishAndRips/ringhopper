use primitives::error::{Error, OverflowCheck, RinghopperResult};
use primitives::primitive::Pixel32;

pub trait Swizzlable: Sized + Copy + Clone + Default {}

impl Swizzlable for u8 {}
impl Swizzlable for u16 {}
impl Swizzlable for Pixel32 {}

pub fn swizzle<S: Swizzlable>(data: &[S], output: &mut [S], width: usize, height: usize, depth: usize, deswizzle: bool) -> RinghopperResult<()> {
    if !width.is_power_of_two() || !height.is_power_of_two() || !depth.is_power_of_two() {
        return Err(Error::InvalidTagData(format!("Cannot swizzle a {width}x{height}x{depth} texture because not all sides are power-of-two.")))
    }

    debug_assert_eq!(data.len(), width.mul_overflow_checked(height).and_then(|c| c.mul_overflow_checked(depth)).expect("width*height*depth shouldn't overflow"), "data input not right size");

    if data.is_empty() {
        return Ok(());
    }

    if depth > 1 {
        if height != width || height != depth {
            return Err(Error::InvalidTagData(format!("Cannot swizzle a 3D {width}x{height}x{depth} texture because not all sides are equal.")))
        }
        swizzle_3d(data, output, width, deswizzle);
    }
    else {
        swizzle_2d(data, output, width, height, deswizzle);
    }

    Ok(())
}

fn swizzle_3d<S: Swizzlable>(data: &[S], output: &mut [S], length: usize, deswizzle: bool) {
    let size_2d = length * length;
    let size_3d = size_2d * length;
    let mask = size_3d - 1;

    for z in 0..length {
        for y in 0..length {
            for x in 0..length {
                let offset = (x + y * length + z * size_2d) & mask;
                let m = morton_encode_3d(x, y, z) & mask;
                if deswizzle {
                    output[offset] = data[m];
                }
                else {
                    output[m] = data[offset];
                }
            }
        }
    }
}

// From https://www.forceflow.be/2013/10/07/morton-encodingdecoding-through-bit-interleaving-implementations/
fn morton_encode_3d(x: usize, y: usize, z: usize) -> usize {
    let mut answer = 0;
    for i in 0..(usize::BITS as usize) / 3 {
        let bit = 1 << i;
        answer |= ((x & bit) << 2*i) | ((y & bit) << (2*i + 1)) | ((z & bit) << (2*i + 2));
    }
    answer
}

fn swizzle_2d<S: Swizzlable>(data: &[S], output: &mut [S], width: usize, height: usize, deswizzle: bool) {
    // Straight copy if the dimensions are this small
    if height <= 1 || width <= 2 {
        for i in 0..width*height {
            output[i] = data[i];
        }
        return;
    }

    // Subdivisions.
    if width < height {
        for y in (0..height).step_by(width) {
            swizzle_2d(&data[y * width..], &mut output[y * width..], width, width, deswizzle);
        }
        return;
    }

    // Do it
    let mut counter = 0;
    for x in (0..width).step_by(height) {
        if !deswizzle {
            counter = swizzle_block(&data[x..], output, height, width, counter, deswizzle);
        }
        else {
            counter = swizzle_block(data, &mut output[x..], height, width, counter, deswizzle);
        }
    }
}

fn swizzle_block_2x2<S: Swizzlable>(values_in: &[S], values_out: &mut [S], stride: usize, counter: usize, deswizzle: bool) -> usize {
    if !deswizzle {
        // values_in  = unswizzled
        // values_out = swizzled
        values_out[counter] = values_in[0];
        values_out[counter + 1] = values_in[1];
        values_out[counter + 2] = values_in[0 + stride];
        values_out[counter + 3] = values_in[1 + stride];
    }
    else {
        // values_in  = swizzled
        // values_out = unswizzled
        values_out[0] = values_in[counter];
        values_out[1] = values_in[counter + 1];
        values_out[0 + stride] = values_in[counter + 2];
        values_out[1 + stride] = values_in[counter + 3];
    }

    counter + 4
}

fn swizzle_block<S: Swizzlable>(values_in: &[S], values_out: &mut [S], width: usize, stride: usize, mut counter: usize, deswizzle: bool) -> usize {
    if width == 2 {
        return swizzle_block_2x2(values_in, values_out, stride, counter, deswizzle);
    }

    let new_width = width / 2;

    if deswizzle {
        counter = swizzle_block(values_in, &mut values_out[..],                               new_width, stride, counter, deswizzle);
        counter = swizzle_block(values_in, &mut values_out[new_width..],                      new_width, stride, counter, deswizzle);
        counter = swizzle_block(values_in, &mut values_out[stride * new_width..],             new_width, stride, counter, deswizzle);
        counter = swizzle_block(values_in, &mut values_out[stride * new_width + new_width..], new_width, stride, counter, deswizzle);
    }
    else {
        counter = swizzle_block(&values_in[..],                               values_out, new_width, stride, counter, deswizzle);
        counter = swizzle_block(&values_in[new_width..],                      values_out, new_width, stride, counter, deswizzle);
        counter = swizzle_block(&values_in[stride * new_width..],             values_out, new_width, stride, counter, deswizzle);
        counter = swizzle_block(&values_in[stride * new_width + new_width..], values_out, new_width, stride, counter, deswizzle);
    }

    counter
}
