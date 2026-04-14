use std::fs::File;
use std::io::BufReader;

fn main() {
    let file = File::open("test.wav").unwrap();
    let source = rodio::Decoder::new(BufReader::new(file)).unwrap();
    println!("channels: {}, sample_rate: {}", source.channels(), source.sample_rate());
}
