use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{App, Arg, ArgMatches};
use image::{ColorType, FilterType, GenericImageView, DynamicImage};
use image::dxt::{DXTEncoder, DXTVariant};
use lz4::block::compress;
use lz4::block::CompressionMode::HIGHCOMPRESSION;
use zerocopy::AsBytes;

use vk_test::bf::{BfHeader, BfImageFormat, Kind, BfImageAdditional};
use vk_test::perf::Stopwatch;
use std::convert::TryFrom;
use vk_test::cli::derive_input_and_output;

struct Timers<'a> {
    load: Stopwatch<'a>,
    vflip: Stopwatch<'a>,
    channels: Stopwatch<'a>,
    mipmaps: Stopwatch<'a>,
    dxt: Stopwatch<'a>,
    lz4: Stopwatch<'a>,
    save: Stopwatch<'a>,
}

impl<'a> Default for Timers<'a> {
    fn default() -> Self {
        Timers {
            load: Stopwatch::new("load"),
            vflip: Stopwatch::new("vflip"),
            channels: Stopwatch::new("channels"),
            mipmaps: Stopwatch::new("mipmaps"),
            dxt: Stopwatch::new("dxt"),
            lz4: Stopwatch::new("lz4"),
            save: Stopwatch::new("save"),
        }
    }
}

// copied from image create because it is not exported
pub fn num_components(c: ColorType) -> usize {
    match c {
        ColorType::Gray(_) => 1,
        ColorType::GrayA(_) => 2,
        ColorType::RGB(_) | ColorType::Palette(_) | ColorType::BGR(_) => 3,
        ColorType::RGBA(_) | ColorType::BGRA(_) => 4,
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
        .arg(Arg::with_name("format")
            .short("f")
            .long("format")
            .value_name("FORMAT")
            .help("One of: DXT1, DXT3, DXT5, RGB8, RGBA8") // todo: generate variants from enum
            .required(true)
            .takes_value(true))
        .arg(Arg::with_name("not-vflip")
            .short("v")
            .long("not-vflip")
            .help("Do not vertically flip image during conversion"))
        .get_matches();

    let (input, output) = derive_input_and_output(&matches);

    // 1. load image
    timers.load.start();
    let mut input_image = image::open(input)
        .map_err(|e| panic!("cannot load input file as image: {}", e))
        .unwrap();
    timers.load.end();

    let (width, height) = (input_image.width(), input_image.height());

    println!("width={}", width);
    println!("height={}", height);
    println!("color={:?}", input_image.color());

    // 2. vflip
    timers.vflip.start();
    if !matches.is_present("not-vflip") {
        input_image = input_image.flipv();
    }
    timers.vflip.end();

    // 3. rgba <-> rgb
    let output_format = BfImageFormat::try_from(matches.value_of("format").unwrap())
        .expect("invalid output format specified");

    // todo: remove and use the function from `image` crate when PR 1002 is merged
    timers.channels.start();
    if num_components(input_image.color()) != output_format.channels() {
        if num_components(input_image.color()) > output_format.channels() {
            input_image = DynamicImage::ImageRgb8(input_image.to_rgb());
        } else {
            input_image = DynamicImage::ImageRgba8(input_image.to_rgba());
        }
    }
    timers.channels.end();

    // 4. mipmaps
    timers.mipmaps.start();
    let mut mipmaps = vec![input_image];
    while mipmaps.last().unwrap().width() > 4 { // 4 is the minimal size for dxt texture
        let higher = mipmaps.last().unwrap();
        let lower = higher
            .clone()
            .resize(
                higher.width() / 2,
                higher.height() / 2,
                FilterType::Lanczos3,
            );
        mipmaps.push(lower);
    }
    timers.mipmaps.end();

    // 5. convert to output format
    timers.dxt.start();
    let mut payload = vec![];
    for img in mipmaps {
        let raw = img.raw_pixels();
        let raw = raw.as_slice();

        let dxt = |variant| {
            let mut storage: Vec<u8> = vec![];
            DXTEncoder::new(&mut storage)
                .encode(raw, img.width(), img.height(), variant)
                .map_err(|e| panic!("dxt compression failed: {}", e))
                .unwrap();
            storage
        };

        let result = match output_format {
            // we need to perform dxt compression
            BfImageFormat::SrgbDxt1 | BfImageFormat::Dxt1 => dxt(DXTVariant::DXT1),
            BfImageFormat::SrgbDxt3 | BfImageFormat::Dxt3 => dxt(DXTVariant::DXT3),
            BfImageFormat::SrgbDxt5 | BfImageFormat::Dxt5 => dxt(DXTVariant::DXT5),

            // for uncompressed formats we just copy the buffer
            _ => Vec::from(raw), // todo: optimize needless copy
        };

        payload.extend(result);
    }
    timers.dxt.end();


    // 5. compress with lz4
    timers.lz4.start();
    let mut compressed = compress(payload.as_slice(), Some(HIGHCOMPRESSION(16)), false)
        .map_err(|e| panic!("compression failed: {}", e))
        .unwrap();
    timers.lz4.end();

    // 6. write file_out
    timers.save.start();
    let bf_header = BfHeader::new(
        Kind::Image,
        1,
        BfImageAdditional::new(width as u16, height as u16, output_format as u8).into_u64(),
        payload.len() as u64,
        compressed.len() as u64,
    );
    let mut out_file = File::create(output)
        .map_err(|e| panic!("cannot open output file: {}", e))
        .unwrap();
    out_file.write_all(&bf_header.as_bytes()).expect("cannot write to output file");
    out_file.write_all(&compressed.as_mut_slice()).expect("cannot write to output file");
    out_file.flush().expect("cannot write to output file");
    ;
    timers.save.end();

    println!("raw={} compressed={} ratio={}", bf_header.uncompressed, bf_header.compressed, 100.0 * bf_header.compressed as f32 / bf_header.uncompressed as f32);
    println!("time load={}ms", timers.load.total_time().as_millis());
    println!("time vflip={}ms", timers.vflip.total_time().as_millis());
    println!("time channels={}ms", timers.channels.total_time().as_millis());
    println!("time mipmaps={}ms", timers.mipmaps.total_time().as_millis());
    println!("time dxt={}ms", timers.dxt.total_time().as_millis());
    println!("time lz4={}ms", timers.lz4.total_time().as_millis());
    println!("time save={}ms", timers.save.total_time().as_millis());
}
