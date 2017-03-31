extern crate clap;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::vec::Vec;
use clap::{Arg, App};
use we32k::get_opcodes;

mod we32k;

fn disassemble(buf: &Vec<u8>) {
    println!("I'm disassembling a {} byte buffer", buf.len());

    let opcodes = get_opcodes();

    println!("I have {} opcodes to look at.", opcodes.len());

    let mut i = 0;
    let mut op: u16;

    while i < buf.len() {
        op = buf[i] as u16;

        // Not all opcodes are one byte. All opcodes starting with
        // 0x30 are two-bytes long.
        if op == 0x30 {
            // Fetch the second byte of the opcode.
            op = 0x3000 | buf[i + 1] as u16;
            i = i + 1;
        }

        println!("0x{:04x}", op);

        i += 1;
    }

    println!();
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

    println!("I'm decompiling: {}", display);

    let mut file = match File::open(&path) {
        Err(why) => panic!("Couldn't open {}: {}", display, why.description()),
        Ok(file) => file,
    };

    let mut buf = Vec::new();

    match file.read_to_end(&mut buf) {
        Err(why) => panic!("Couldn't open {}: {}", display, why.description()),
        Ok(_) => (),
    }

    disassemble(&buf);
}
