use std::env;
use std::fs::File;
use std::io::Read;
use vk_test::bf::{Kind, BfImageAdditional};
use std::convert::TryFrom;
use lz4::block::decompress;

fn main() {
    let args: Vec<String> = env::args().collect();
    let input = &args[1];

    let mut file = File::open(input)
        .map_err(|e| panic!("cannot open input file: {}", e))
        .unwrap();

    println!("file={}", input);

    let mut cnts = vec![];
    file.read_to_end(&mut cnts);

    let file = vk_test::bf::load_bf_from_bytes(&cnts)
        .map_err(|e| panic!("cannot decode input file: {:?}", e))
        .unwrap();
    let header = file.header;
    let payload = file.data;

    let kind = Kind::try_from(header.kind)
        .map_err(|e| panic!("invalid kind value: {}", header.kind))
        .unwrap();


    println!("magic={}", header.magic);
    println!("version={}", header.version);
    println!("kind={:?}", kind);

    match kind {
        Kind::Image => println!("additional={:?}", BfImageAdditional::from_u64(header.additional)),
        _ => println!("additional={}", header.additional)
    }

    println!("uncompressed={}", header.uncompressed);
    println!("compressed={}", header.compressed);

    let uncompressed = decompress(payload, Some(header.uncompressed as i32))
        .map_err(|e| panic!("payload decompression failed: {}", header.kind))
        .unwrap();


}