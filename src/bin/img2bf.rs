use clap::{App, Arg};
use vk_test::perf::Stopwatch;
use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use image::GenericImageView;
use lz4::block::compress;
use lz4::block::CompressionMode::HIGHCOMPRESSION;
use vk_test::bf::{BfHeader, Type};
use std::fs::File;
use std::io::Write;
use zerocopy::AsBytes;

fn derive_output_from(input: &str) -> PathBuf {
    let stem = Path::new(input)
        .file_stem()
        .expect("input file is not a valid file");

    let mut owned = stem.to_owned();
    owned.push(".bf");
    PathBuf::from(owned)
}

struct Timers<'a> {
    load: Stopwatch<'a>,
    lz4: Stopwatch<'a>,
    save: Stopwatch<'a>,
}

impl<'a> Default for Timers<'a> {
    fn default() -> Self {
        Timers {
            load: Stopwatch::new("load"),
            lz4: Stopwatch::new("lz4"),
            save: Stopwatch::new("save"),
        }
    }
}

fn main() {
    let mut timers = Timers::default();

    let matches = App::new("img2bf")
        .version("1.0")
        .author("Matej K. <dobrakmato@gmail.com>")
        .about("Converts basic image format to BF optimized format")
        .arg(Arg::with_name("content")
            .long("content")
            .value_name("CONTENT_PATH")
            .help("Specifies the content root directory to import the file into")
            .takes_value(true))
        .arg(Arg::with_name("input")
            .short("in")
            .long("input")
            .value_name("INPUT_FILE")
            .help("Path to file to convert / import")
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("output")
            .short("out")
            .long("output")
            .value_name("OUTPUT_FILE")
            .help("Path to output file to generate")
            .takes_value(true))
        .get_matches();

    // todo: specify color space, format, dxt threads, encoding, compression, vflip,

    let input = matches.value_of("input").unwrap();
    let output = match matches.value_of("output") {
        None => derive_output_from(input),
        Some(t) => PathBuf::from(t),
    };

    let input = Path::new(input);

    timers.load.start();
    let pixel_data = load_input_image(input);
    timers.load.end();
    timers.lz4.start();
    let mut compressed = compress_image_data(&pixel_data);
    timers.lz4.end();
    timers.save.start();
    let bf_header = save_output_image(output, pixel_data, &mut compressed);
    timers.save.end();

    println!("raw={} compressed={} ratio={}", bf_header.uncompressed, bf_header.compressed, 100.0 * bf_header.compressed as f32 / bf_header.uncompressed as f32);
    println!("time load={}ms", timers.load.total_time().as_millis());
    println!("time lz4={}ms", timers.lz4.total_time().as_millis());
    println!("time save={}ms", timers.save.total_time().as_millis());
}

fn save_output_image(output: PathBuf, pixel_data: Vec<u8>, compressed: &mut Vec<u8>) -> BfHeader {
    let bf_header = BfHeader::new(
        Type::Image,
        1,
        pixel_data.len() as u64,
        compressed.len() as u64,
    );
    let mut out_file = File::create(output)
        .map_err(|e| panic!("cannot open output file: {}", e))
        .unwrap();
    out_file.write_all(&bf_header.as_bytes());
    out_file.write_all(&compressed.as_mut_slice());
    out_file.flush();
    bf_header
}

fn compress_image_data(pixel_data: &Vec<u8>) -> Vec<u8> {
    let mut compressed = compress(pixel_data.as_slice(), Some(HIGHCOMPRESSION(16)), false)
        .map_err(|e| panic!("compression failed: {}", e))
        .unwrap();
    compressed
}

fn load_input_image(input: &Path) -> Vec<u8> {
    let input_image = image::open(input)
        .map_err(|e| panic!("cannot load input file as image: {}", e))
        .unwrap();
    let pixel_data = input_image.raw_pixels();
    pixel_data
}