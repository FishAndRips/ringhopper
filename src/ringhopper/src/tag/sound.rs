use std::{io::{Cursor, Read, Seek, SeekFrom}, mem::zeroed, os::raw::c_void};

use aotuv_lancer_vorbis_sys::{ov_callbacks, ov_clear, ov_info, ov_open_callbacks, ov_pcm_total, OggVorbis_File};
use primitives::error::{Error, OverflowCheck, RinghopperResult};
use ringhopper_structs::{Sound, SoundChannelCount, SoundFormat, SoundPermutation};

pub fn get_correct_sound_buffer_size(sound: &Sound, permutation: &SoundPermutation) -> RinghopperResult<usize> {
    let pformat = permutation.format;
    let sformat = sound.format;

    if sformat != pformat {
        return Err(Error::InvalidTagData(format!("Refusing to check permutation because its format ({pformat}) does not match the sound ({sformat})")))
    }

    match pformat {
        SoundFormat::XboxADPCM => Ok(0),
        SoundFormat::ImaADPCM => Ok(0),
        SoundFormat::PCM => Ok(permutation.samples.bytes.len()),
        SoundFormat::OggVorbis => get_sample_count_for_ogg_vorbis(sound, permutation)
    }
}

fn get_sample_count_for_ogg_vorbis(sound: &Sound, permutation: &SoundPermutation) -> RinghopperResult<usize> {
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

    let short_circuit = (|| {
        let info = unsafe { &*ov_info(&mut vf, -1) };
        let (cc, ogg_channel_count) = match info.channels {
            1 => (1usize, SoundChannelCount::Mono),
            2 => (2usize, SoundChannelCount::Stereo),
            n => return Err(Error::InvalidTagData(format!("Ogg data has {n} channels which is unsupported")))
        };

        let actual_channel_count = sound.channel_count;
        if ogg_channel_count != actual_channel_count {
            return Err(Error::InvalidTagData(format!("Ogg data reports as {ogg_channel_count} when the sound is {actual_channel_count}")))
        }

        let pcm_count = match unsafe { ov_pcm_total(&mut vf, -1) } {
            n if n < 0 => return Err(Error::InvalidTagData(format!("Unable to determine PCM count because an error occurred"))),
            n => n as u64
        };

        usize::try_from(pcm_count)
            .ok()
            .and_then(|f| f.mul_overflow_checked(cc * std::mem::size_of::<u16>()).ok())
            .ok_or_else(|| Error::InvalidTagData(format!("Unable to determine sample count from PCM count ({pcm_count} x {cc}) because it is not valid for the platform's usize")))
    })();

    unsafe { ov_clear(&mut vf); }

    short_circuit
}
