use std::fs::File;
use std::io::BufReader;

fn main() {
    let file = File::open("test.wav").unwrap();
    let decoder = rodio::Decoder::new(BufReader::new(file)).unwrap();

    // rodio::Decoder implements Iterator<Item = i16>
    let channels = rodio::Source::channels(&decoder);
    let sample_rate = rodio::Source::sample_rate(&decoder);

    let spec = hound::WavSpec {
        channels: channels as u16,
        sample_rate: sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create("output.wav", spec).unwrap();
    for sample in decoder {
        writer.write_sample(sample).unwrap();
    }
}
