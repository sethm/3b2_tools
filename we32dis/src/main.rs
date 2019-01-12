extern crate clap;
#[macro_use] extern crate bitflags;

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::vec::Vec;
use std::io::Cursor;
use std::str;

use clap::{Arg, App};

use crate::coff::*;

mod coff;

fn name(buf: &[u8]) -> Result<&str, std::str::Utf8Error> {
    let nul = buf.iter().position( |&c| c == b'\0').unwrap_or(buf.len());
    str::from_utf8(&buf[0..nul])
}

fn disassemble(buf: &[u8]) {
    let mut cursor = Cursor::new(buf);

    if let Ok(metadata) = MetaData::read(&mut cursor) {
        println!("File Header:");
        println!("    {:?}", metadata.header);
        println!("    Magic Number:  0x{:04x}", metadata.header.magic);
        println!("    # Sections:    {}", metadata.header.section_count);
        println!("    Date:          {}", metadata.timestamp.to_rfc2822());
        println!("    Symbols Ptr:   0x{:x}", metadata.header.symbols_pointer);
        println!("    Symbol Count:  {}", metadata.header.symbol_count);
        println!("    Opt Hdr:       {:?}", metadata.header.opt_header == 0x1c);
        println!("    Flags:         0x{:04x}", metadata.header.flags);

        if let Some(opt_header) = metadata.opt_header {
            println!();
            println!("Optional Header:");
            println!("    Magic Number:    0x{:04x}", opt_header.magic);
            println!("    Version Stamp:   0x{:04x}", opt_header.version_stamp);
            println!("    Text Size:       0x{:x}", opt_header.text_size);
            println!("    dsize:           0x{:x}", opt_header.dsize);
            println!("    bsize:           0x{:x}", opt_header.bsize);
            println!("    Entry Point:     0x{:x}", opt_header.entry_point);
            println!("    Text Start:      0x{:x}", opt_header.text_start);
            println!("    Data Start:      0x{:x}", opt_header.data_start);
        }

        for section in metadata.sections {

            let header = section.header;

            println!();
            println!("Section Header:");
            println!("    Name:              {}", name(&header.name).unwrap());
            println!("    Phys. Addr:        0x{:x}", header.paddr);
            println!("    Virtual Addr:      0x{:x}", header.vaddr);
            println!("    Sec. Size:         0x{:x}", header.size);
            println!("    Data Offset:       0x{:x}", header.scnptr);
            println!("    Rel. Tab. Offset:  0x{:x}", header.relptr);
            println!("    Line Num. Offset:  0x{:x}", header.lnnoptr);
            println!("    Rel. Tab. Entries: {}", header.nreloc);
            println!("    Line Num. Entries: {}", header.nlnno);
            println!("    Flags:             0x{:08x}", header.flags);

            // If there is relocation data, let's dump that too.

            if header.nreloc > 0 {
                println!("    Relocation Table:");

                for (i, entry) in section.relocation_entries.iter().enumerate() {
                    println!("        [{:03}]  vaddr=0x{:08x}, symndx={}, type={}",
                             i,  entry.vaddr, entry.symndx, entry.rtype);
                }
            }
        }
    };
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
