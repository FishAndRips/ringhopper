use std::{io::{Cursor, Read, Seek, SeekFrom}, mem::zeroed, os::raw::c_void};

use aotuv_lancer_vorbis_sys::{ov_callbacks, ov_clear, ov_info, ov_open_callbacks, ov_pcm_total, OggVorbis_File};
use primitives::error::{Error, RinghopperResult};
use ringhopper_structs::{Sound, SoundChannelCount, SoundFormat, SoundPermutation, SoundSampleRate};

/// Data that describes a permutation
#[derive(Copy, Clone)]
pub struct SoundPermutationMetadata {
    /// Buffer size in bytes for the permutation.
    pub buffer_size: u32,

    /// Channel count for the permutation.
    pub channel_count: SoundChannelCount,

    /// Sample rate for the permutation.
    pub sample_rate: SoundSampleRate
}

impl SoundPermutationMetadata {
    /// Query the actual sound metadata for the permutation.
    ///
    /// Returns `Err` if it's invalid.
    pub fn read_from_sound_permutation(sound: &Sound, permutation: &SoundPermutation) -> RinghopperResult<Self> {
        let pformat = permutation.format;
        let sformat = sound.format;

        if sound.format != permutation.format {
            return Err(Error::InvalidTagData(format!("Refusing to get sound metadata because its format ({pformat}) does not match the sound ({sformat})")))
        }

        Ok(match pformat {
            SoundFormat::ImaADPCM | SoundFormat::XboxADPCM => Self {
                buffer_size: 0,
                channel_count: sound.channel_count,
                sample_rate: sound.sample_rate
            },
            SoundFormat::OggVorbis => Self::get_sound_metadata_for_ogg(permutation)?,
            SoundFormat::PCM => Self {
                buffer_size: permutation.samples.bytes.len().try_into().map_err(|_| Error::InvalidTagData(format!("Buffer size {} overflows 32-bit integer.", permutation.samples.bytes.len())))?,
                channel_count: sound.channel_count,
                sample_rate: sound.sample_rate
            }
        })
    }

    fn get_sound_metadata_for_ogg(permutation: &SoundPermutation) -> RinghopperResult<Self> {
        let mut vf: OggVorbis_File = unsafe { zeroed() };
        type VecU8Cursor<'a> = Cursor<&'a Vec<u8>>;
        let mut data_source: VecU8Cursor = Cursor::new(&permutation.samples.bytes);

        unsafe extern "C" fn read_data(to: *mut c_void, length: usize, nmem: usize, data_source: *mut c_void) -> usize {
            let data_source = data_source as *mut _ as *mut VecU8Cursor;
            let to_buffer = std::slice::from_raw_parts_mut(to as *mut u8, length * nmem);
            let mut read = 0;

            for chunk in to_buffer.chunks_mut(length) {
                match (*data_source).read(chunk) {
                    Ok(n) if n == length => read += 1,
                    _ => break
                }
            }

            read
        }

        unsafe extern "C" fn seek_data(data_source: *mut c_void, offset: i64, whence: std::ffi::c_int) -> std::ffi::c_int {
            let data_source = &mut *(data_source as *mut _ as *mut VecU8Cursor);
            let seek_style = match whence {
                libc::SEEK_SET => SeekFrom::Start(offset as u64),
                libc::SEEK_END => SeekFrom::End(offset),
                libc::SEEK_CUR => SeekFrom::Current(offset),
                _ => unreachable!("invalid whence passed to seek_data: {whence}")
            };

            if data_source.seek(seek_style).is_ok() {
                0
            }
            else {
                -1
            }
        }

        unsafe extern "C" fn tell_data(data_source: *mut c_void) -> std::ffi::c_long {
            let data_source = data_source as *mut _ as *mut VecU8Cursor;
            (*data_source).position() as std::ffi::c_long
        }

        let callbacks = ov_callbacks {
            read_func: Some(read_data),
            seek_func: Some(seek_data),
            close_func: None,
            tell_func: Some(tell_data)
        };

        let result = unsafe {
            ov_open_callbacks(
                &mut data_source as *mut _ as *mut c_void,
                &mut vf,
                std::ptr::null_mut(),
                0,
                callbacks
            )
        };

        if result < 0 {
            return Err(Error::InvalidTagData(format!("Ogg data is invalid!")));
        }

        let metadata = (|| {
            let info = unsafe { &*ov_info(&mut vf, -1) };
            let channel_count = match info.channels {
                1 => (1, Some(SoundChannelCount::Mono)),
                2 => (2, Some(SoundChannelCount::Stereo)),
                n => (n.try_into().map_err(|_| Error::InvalidTagData(format!("Unable to determine channel count from Ogg")))?, None)
            };

            let sample_rate = match info.rate {
                44100 => (44100, Some(SoundSampleRate::_44100Hz)),
                22050 => (22050, Some(SoundSampleRate::_22050Hz)),
                n => (n.try_into().map_err(|_| Error::InvalidTagData(format!("Unable to determine sample rate from Ogg")))?, None)
            };

            let pcm_count = match unsafe { ov_pcm_total(&mut vf, -1) } {
                n if n < 0 => return Err(Error::InvalidTagData(format!("Unable to determine PCM count because an error occurred"))),
                n => n as u64
            };

            let bytes_per_sample = channel_count.0 * (std::mem::size_of::<u16>() as u32);
            let buffer_size = u32::try_from(pcm_count)
                .ok()
                .and_then(|f| f.checked_mul(bytes_per_sample))
                .ok_or_else(|| Error::InvalidTagData(format!("Unable to determine buffer size because it overflows a 32-bit unsigned integer.")))?;

            Ok(Self {
                channel_count: channel_count.1.ok_or_else(|| Error::InvalidTagData(format!("Channel count ({}) from Ogg is unsupported", channel_count.0)))?,
                sample_rate: sample_rate.1.ok_or_else(|| Error::InvalidTagData(format!("Sample rate ({}) from Ogg is unsupported", sample_rate.0)))?,
                buffer_size
            })
        })();

        unsafe { ov_clear(&mut vf); }

        metadata
    }
}

/// Convert a [`SoundChannelCount`] to its equivalent numerical value.
pub fn channel_count_to_u32(channel_count: SoundChannelCount) -> u32 {
    match channel_count {
        SoundChannelCount::Mono => 1,
        SoundChannelCount::Stereo => 2
    }
}

/// Get a [`SoundChannelCount`] from its equivalent numerical value.
pub fn channel_count_from_u32(channel_count: u32) -> Option<SoundChannelCount> {
    match channel_count {
        1 => Some(SoundChannelCount::Mono),
        2 => Some(SoundChannelCount::Stereo),
        _ => None
    }
}

/// Convert a [`SoundSampleRate`] to its equivalent numerical value.
pub fn sample_rate_to_u32(sample_rate: SoundSampleRate) -> u32 {
    match sample_rate {
        SoundSampleRate::_22050Hz => 22050,
        SoundSampleRate::_44100Hz => 44100
    }
}

/// Get a [`SoundChannelCount`] from its equivalent numerical value.
pub fn sample_rate_from_u32(sample_rate: u32) -> Option<SoundSampleRate> {
    match sample_rate {
        1 => Some(SoundSampleRate::_22050Hz),
        2 => Some(SoundSampleRate::_44100Hz),
        _ => None
    }
}
