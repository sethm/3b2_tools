extern crate clap;
#[macro_use] extern crate bitflags;

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::vec::Vec;

use clap::{Arg, App};

use crate::coff::FileContainer;
use crate::decode::Decoder;
use std::io::Cursor;

mod errors;
mod coff;
mod decode;

fn disassemble(buf: &[u8]) {
    match FileContainer::read(buf) {
        Ok(container) => {
            println!("{:?}", container.header);
//
//            if let Some(opt_header) = &container.opt_header {
//                println!("{:?}", opt_header);
//            }
//
//            for (sec_num, section) in container.sections.iter().enumerate() {
//                println!("{:?}", section.header);
//
//                if let Err(e) = container.dump_relocation_table(sec_num) {
//                    println!("Error: Couldn't dump relocation table: {:?}", e);
//                }
//
//                if let Err(e) = container.dump_section_data(sec_num) {
//                    println!("Error: Couldn't dump section data: {:?}", e);
//                }
//            }
//            container.dump_symbol_table();
//            container.dump_strings_table();


            // OK, now let's try to decode some shit.
            if let Some(data) = container.section_data(0) {
                println!("\nSection: .text\n");
                let mut decoder = Decoder::new();
                let mut cursor: Cursor<&[u8]> = Cursor::new(data);

                while let Ok(()) = decoder.decode_instruction(&mut cursor) {
                    println!("{}", decoder.ir);
                }
            }
        },
        Err(e) => {
            println!("Could not parse file: {}", e);
        }
    }
}

fn main() {
    let matches = App::new("WE32100 Disassembler")
        .version("1.0")
        .author("Seth J. Morabito <web@loomcom.com>")
        .about("WE32100 Disassembler")
        .arg(Arg::with_name("offset")
             .value_name("OFFSET")
             .short("o")
             .long("offset")
             .help("Offset within the file to start disassembly")
             .takes_value(true))
        .arg(Arg::with_name("INPUT")
             .value_name("FILE")
             .help("Input file to decompile")
             .required(true)
             .index(1))
        .get_matches();

    let infile = matches.value_of("INPUT").unwrap();

    let path = Path::new(infile);
    let display = path.display();

    let mut file = match File::open(&path) {
        Err(why) => panic!("Couldn't open {}: {}", display, why.description()),
        Ok(file) => file,
    };

    let mut buf = Vec::new();

    if let Err(why) = file.read_to_end(&mut buf) {
        panic!("Couldn't open {}: {}", display, why.description())
    }

    disassemble(&buf);
}
