use std::path::{Path, PathBuf};
use clap::ArgMatches;

/// Derives output path from input path by changing the file's extension.
pub fn derive_output_from(input: &str) -> PathBuf {
    let stem = Path::new(input)
        .file_stem()
        .expect("input file is not a valid file");

    let mut owned = stem.to_owned();
    owned.push(".bf");
    PathBuf::from(owned)
}

/// Creates Path-like objects for input and output file from the arguments
/// passed to the application.
pub fn derive_input_and_output(matches: &ArgMatches) -> (PathBuf, PathBuf) {
    let input = matches.value_of("input").unwrap();
    let output = match matches.value_of("output") {
        None => derive_output_from(input),
        Some(t) => PathBuf::from(t),
    };
    let input = PathBuf::from(input);
    (input, output)
}
