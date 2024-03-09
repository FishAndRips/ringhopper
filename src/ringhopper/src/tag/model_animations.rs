use definitions::{AnimationFrameInfoType, ModelAnimationsAnimation, ModelAnimationsFrameInfoDxDy, ModelAnimationsFrameInfoDxDyDyaw, ModelAnimationsFrameInfoDxDyDzDyaw, ModelAnimationsRotation, ModelAnimationsScale, ModelAnimationsTransform};
use primitives::byteorder::ByteOrder;
use primitives::error::{Error, OverflowCheck, RinghopperResult};
use primitives::parse::SimpleTagData;

#[derive(Default, Clone, Copy, Debug)]
pub enum FrameDataType {
    #[default]
    Rotate,
    Transform,
    Scale
}

#[derive(Copy, Clone)]
pub struct FrameDataIterator {
    scale: u64,
    transform: u64,
    rotation: u64,

    mask: u64,
    remaining_nodes: usize,
    mode: FrameDataType
}

impl Iterator for FrameDataIterator {
    type Item = FrameDataType;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.remaining_nodes == 0 {
                return None
            }

            let old_mode = self.mode;
            let matched = match old_mode {
                FrameDataType::Rotate => {
                    self.mode = FrameDataType::Transform;
                    self.rotation & self.mask != 0
                },
                FrameDataType::Transform => {
                    self.mode = FrameDataType::Scale;
                    self.transform & self.mask != 0
                },
                FrameDataType::Scale => {
                    self.mode = FrameDataType::Rotate;
                    let matched = self.scale & self.mask != 0;
                    self.mask <<= 1;
                    self.remaining_nodes -= 1;
                    matched
                }
            };

            if matched {
                return Some(old_mode);
            }
        }
    }
}

impl FrameDataIterator {
    pub fn for_animation(animation: &ModelAnimationsAnimation) -> FrameDataIterator {
        let flag_data_to_u64 = |data: [u32; 2]| ((data[1] as u64) << 32) | (data[0] as u64);
        FrameDataIterator {
            mode: FrameDataType::default(),
            mask: 1,
            rotation: flag_data_to_u64(animation.node_rotation_flag_data),
            transform: flag_data_to_u64(animation.node_transform_flag_data),
            scale: flag_data_to_u64(animation.node_scale_flag_data),
            remaining_nodes: animation.node_count as usize
        }
    }
    pub fn for_animation_inverted(animation: &ModelAnimationsAnimation) -> FrameDataIterator {
        let flag_data_to_u64 = |data: [u32; 2]| ((data[1] as u64) << 32) | (data[0] as u64);
        FrameDataIterator {
            mode: FrameDataType::default(),
            mask: 1,
            rotation: !flag_data_to_u64(animation.node_rotation_flag_data),
            transform: !flag_data_to_u64(animation.node_transform_flag_data),
            scale: !flag_data_to_u64(animation.node_scale_flag_data),
            remaining_nodes: animation.node_count as usize
        }
    }
    pub fn to_size(self) -> usize {
        let mut size = 0;
        for i in self {
            size += match i {
                FrameDataType::Transform => ModelAnimationsTransform::simple_size(),
                FrameDataType::Scale => ModelAnimationsScale::simple_size(),
                FrameDataType::Rotate => ModelAnimationsRotation::simple_size()
            }
        }
        size
    }
    fn flip_all<From: ByteOrder, To: ByteOrder>(self, data: &mut [u8], offset: &mut usize) {
        for i in self {
            let frame_data = &mut data[*offset..];

            macro_rules! flip {
                ($type:ty) => {{
                    let q = <$type>::read::<From>(frame_data, 0, frame_data.len()).unwrap();
                    q.write::<To>(frame_data, 0, frame_data.len()).unwrap();
                    *offset += <$type>::simple_size();
                }};
            }

            match i {
                FrameDataType::Rotate => flip!(ModelAnimationsRotation),
                FrameDataType::Transform => flip!(ModelAnimationsTransform),
                FrameDataType::Scale => flip!(ModelAnimationsScale)
            }
        }
    }
}

pub(crate) fn flip_endianness_for_model_animations_animation<From: ByteOrder, To: ByteOrder>(animation: &mut ModelAnimationsAnimation) -> RinghopperResult<()> {
    if animation.node_count > 64 {
        return Err(Error::InvalidTagData(format!("model animation has {} nodes (more than 64)", animation.node_count)))
    }

    match animation.frame_info_type {
        AnimationFrameInfoType::None => {
            if !animation.frame_info.bytes.is_empty() {
                return Err(Error::InvalidTagData("model animation has no frame info type, but has frame info data".to_owned()))
            }
        },
        AnimationFrameInfoType::DxDy => flip_endianness_for_frame_info::<ModelAnimationsFrameInfoDxDy, From, To>(animation)?,
        AnimationFrameInfoType::DxDyDyaw => flip_endianness_for_frame_info::<ModelAnimationsFrameInfoDxDyDyaw, From, To>(animation)?,
        AnimationFrameInfoType::DxDyDzDyaw => flip_endianness_for_frame_info::<ModelAnimationsFrameInfoDxDyDzDyaw, From, To>(animation)?,
    }

    flip_endianness_for_frame_data::<From, To>(animation)?;
    flip_endianness_for_default_data::<From, To>(animation)?;

    Ok(())
}


fn flip_endianness_for_frame_info<D: SimpleTagData, From: ByteOrder, To: ByteOrder>(animation: &mut ModelAnimationsAnimation) -> RinghopperResult<()> {
    let frame_count = animation.frame_count as usize;
    let frame_size = D::simple_size();
    let expected_size = frame_size * frame_count;
    let actual_size = animation.frame_info.bytes.len();
    if expected_size != actual_size {
        return Err(Error::InvalidTagData(format!("model animation has {actual_size} byte(s) for frame info when it should be {expected_size}")))
    }

    let data = animation.frame_info.bytes.as_mut_slice();
    for f in (0..actual_size).step_by(frame_size) {
        let frame = D::read::<From>(data, f, actual_size).unwrap();
        frame.write::<To>(data, f, actual_size).unwrap();
    }

    Ok(())
}

fn flip_endianness_for_frame_data<From: ByteOrder, To: ByteOrder>(animation: &mut ModelAnimationsAnimation) -> RinghopperResult<()> {
    let is_compressed = animation.flags.compressed_data;
    let frame_data_iterator = FrameDataIterator::for_animation(animation);

    let frame_size = frame_data_iterator.clone().to_size();
    let frame_count = animation.frame_count as usize;
    let total_size = frame_size.mul_overflow_checked(frame_count)?;

    if is_compressed {
        if animation.offset_to_compressed_data as usize != total_size {
            return Err(Error::InvalidTagData(format!("model animation offset {offset} is not exactly after frame data {total_size}", offset=animation.offset_to_compressed_data as usize)))
        }
    }
    else {
        if total_size != animation.frame_data.bytes.len() {
            return Err(Error::InvalidTagData(format!("model animation frame data size is wrong ({actual} actual != {total_size} expected)", actual=animation.frame_data.bytes.len())))
        }
    }

    let mut offset = 0usize;
    for _ in 0..frame_count {
        frame_data_iterator.clone().flip_all::<From, To>(&mut animation.frame_data.bytes[..total_size], &mut offset);
    }

    debug_assert_eq!(offset, total_size);

    Ok(())
}

fn flip_endianness_for_default_data<From: ByteOrder, To: ByteOrder>(animation: &mut ModelAnimationsAnimation) -> RinghopperResult<()> {
    let frame_data_iterator = FrameDataIterator::for_animation_inverted(animation);

    let frame_size = frame_data_iterator.clone().to_size();

    if frame_size != animation.default_data.bytes.len() {
        return Err(Error::InvalidTagData(format!("model animation default data size is wrong ({actual} != {frame_size})", actual=animation.default_data.bytes.len())))
    }

    let mut offset = 0usize;
    frame_data_iterator.flip_all::<From, To>(&mut animation.default_data.bytes, &mut offset);
    debug_assert_eq!(offset, frame_size);

    Ok(())
}
