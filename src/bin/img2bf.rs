use clap::{App, Arg};
use vk_test::perf::Stopwatch;

fn main() {
    let total_time = Stopwatch::new("time");
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


}