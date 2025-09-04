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

    pub fn format_info(&self) -> String {
        format!(
            "WAV Header Information:\n\
             ChunkID: {}\n\
             ChunkSize: {} bytes\n\
             Format: {}\n\
             Subchunk1ID: {}\n\
             Subchunk1Size: {} bytes\n\
             Audio Format: {} (1 = PCM)\n\
             Number of Channels: {}\n\
             Sample Rate: {} Hz\n\
             Byte Rate: {} bytes/sec\n\
             Block Align: {} bytes\n\
             Bits per Sample: {} bits\n",
            String::from_utf8_lossy(&self.chunk_id),
            self.chunk_size,
            String::from_utf8_lossy(&self.format),
            String::from_utf8_lossy(&self.subchunk1_id),
            self.subchunk1_size,
            self.audio_format,
            self.num_channels,
            self.sample_rate,
            self.byte_rate,
            self.block_align,
            self.bits_per_sample
        )
    }
}
