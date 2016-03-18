extern crate core;
extern crate clap;

pub mod util;
pub mod parser;

// we only use std::fs in our commandline frontend. the parser uses libcore
// only
use std::fs;
use std::io::Read;

fn main() {
    let matches = clap::App::new("device-tree-parser")
                                .arg(clap::Arg::with_name("input_file")
                                    .help("Flattened device tree (.dtb)")
                                    .takes_value(true)
                                    .required(true)
                                    .value_name("FILE"))
                                .get_matches();

    // read file into memory
    let mut input = fs::File::open(matches.value_of("input_file").unwrap())
                                  .unwrap();
    let mut buf = Vec::new();
    input.read_to_end(&mut buf).unwrap();

    let mut parser = parser::DeviceTreeParser::new(&mut buf);

    match parser.parse() {
        Ok(result) => {
            println!("{}", result);
        },
        Err(e) => {
            println!("{:?} @ {:#X}", e, parser.pos());
        }
    }
}
