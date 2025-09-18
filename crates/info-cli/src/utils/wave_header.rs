use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::Read;

#[derive(Debug)]
pub struct WavHeader {
    chunk_id: [u8; 4],
    chunk_size: u32,
    format: [u8; 4],
    subchunk1_id: [u8; 4],
    subchunk1_size: u32,
    audio_format: u16,
    num_channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

impl WavHeader {
    pub fn read_from_file(file: &mut File) -> Result<Self, std::io::Error> {
        let mut header = WavHeader {
            chunk_id: [0; 4],
            chunk_size: 0,
            format: [0; 4],
            subchunk1_id: [0; 4],
            subchunk1_size: 0,
            audio_format: 0,
            num_channels: 0,
            sample_rate: 0,
            byte_rate: 0,
            block_align: 0,
            bits_per_sample: 0,
        };

        file.read_exact(&mut header.chunk_id)?;
        header.chunk_size = file.read_u32::<LittleEndian>()?;
        file.read_exact(&mut header.format)?;
        file.read_exact(&mut header.subchunk1_id)?;
        header.subchunk1_size = file.read_u32::<LittleEndian>()?;
        header.audio_format = file.read_u16::<LittleEndian>()?;
        header.num_channels = file.read_u16::<LittleEndian>()?;
        header.sample_rate = file.read_u32::<LittleEndian>()?;
        header.byte_rate = file.read_u32::<LittleEndian>()?;
        header.block_align = file.read_u16::<LittleEndian>()?;
        header.bits_per_sample = file.read_u16::<LittleEndian>()?;

        Ok(header)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn bits_per_sample(&self) -> u16 {
        self.bits_per_sample
    }

    pub fn num_channels(&self) -> u16 {
        self.num_channels
    }

    pub fn total_samples(&self) -> Option<u64> {
        // Assuming a standard 44-byte WAV header, the data chunk size is chunk_size - 36.
        // This might need to be more robust for non-standard WAV files.
        let data_size = self.chunk_size.checked_sub(36)?;
        let bytes_per_sample_per_channel = self.bits_per_sample as u32 / 8;
        let bytes_per_frame = self.num_channels as u32 * bytes_per_sample_per_channel;

        if bytes_per_frame == 0 {
            None
        } else {
            Some((data_size / bytes_per_frame) as u64)
        }
    }

    pub fn time_precision(&self) -> Option<f64> {
        if self.sample_rate == 0 {
            None
        } else {
            Some(1.0 / self.sample_rate as f64)
        }
    }
}
