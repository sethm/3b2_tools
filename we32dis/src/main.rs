extern crate clap;
#[macro_use] extern crate bitflags;

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::vec::Vec;
use std::str;

use clap::{Arg, App};

use crate::coff::{MetaData, SectionHeader};

mod errors;
mod coff;

fn name(buf: &[u8]) -> Result<&str, std::str::Utf8Error> {
    let nul = buf.iter().position( |&c| c == b'\0').unwrap_or(buf.len());
    str::from_utf8(&buf[0..nul])
}

fn disassemble(buf: &[u8]) {
    match MetaData::read(buf) {
        Ok(metadata) => {
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
                let header: SectionHeader = section.header;
                let sec_name = name(&header.name).unwrap();

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
                if section.relocations.len() > 0 {
                    println!("    Relocation Table:");

                    println!("        Num    Vaddr       Symndx  Type");
                    println!("        -----  ----------  ------  ----");

                    for (i, entry) in section.relocations.iter().enumerate() {
                        println!("        [{:03}]  0x{:08x}  {:6}  {:3}",
                                 i,  entry.vaddr, entry.symndx, entry.rtype);
                    }
                }

                // If there is data, dump it.
                if section.data.len() > 0 {
                    println!("    Section Data");

                    // Make a cute little array for our read data.
                    let mut row_bytes: [u8; 16] = [0; 16];

                    for (i, b) in section.data.iter().enumerate() {
                        row_bytes[i % 16] = *b;

                        if i % 16 == 0 {
                            let vaddr = header.vaddr + i as u32;
                            print!("        {:08x}:   ", vaddr);
                        }

                        print!("{:02x} ", b);

                        if (i + 1) % 8 == 0 && (i + 1) % 16 != 0 {
                            print!("  ");
                        }

                        // If we need to end a line, it's time to print the
                        // human-readable summary.

                        if (i + 1) % 16 == 0 || i == (header.size - 1) as usize {

                            // How many empty characters do we need to pad out
                            // before the summary?
                            let spaces = if i == (header.size - 1) as usize {
                                16 - (header.size % 16)
                            } else {
                                0
                            };

                            if spaces > 0 {
                                eprintln!("*** section {} line end. spaces = {}",
                                          sec_name, spaces);
                            }

                            for _ in 0..spaces {
                                print!("   ");
                            }

                            if spaces > 8 {
                                print!("  ");
                            }

                            print!("  | ");

                            for (x, c) in row_bytes.iter().enumerate() {
                                if x < (16 - spaces) as usize {
                                    let printable = if *c >= 0x20 && *c < 0x7f {
                                        *c as char
                                    } else {
                                        b'.' as char
                                    };
                                    print!("{}", printable);
                                } else {
                                    print!(" ");
                                }
                            }

                            println!(" |");
                        }
                    }
                }
            }

            // Dump the symbols table
            let strings = metadata.strings;

            if !metadata.symbols.is_empty() {
                println!();
                println!("Symbol Table:");

                println!("    Num       Name             Offset     Value      Scnum Type Class Numaux");
                println!("    ------    ---------------- ---------- ---------- ----- ---- ----- ------");

                for (i, e) in metadata.symbols.iter().enumerate() {
                    let stype = match e.is_aux {
                        true => "a",
                        false => "m"
                    };

                    let name = match e.n_zeroes {
                        0 => {
                            match e.is_aux {
                                true => "",
                                false => strings.string_at(e.n_offset).unwrap(),
                            }
                        }
                        _ => name(&e.n_name).unwrap(),
                    };

                    println!("    [{:4}] {:2} {:16} 0x{:08x} 0x{:08x} {:5} {:04x}    {:02x}     {:2}",
                             i, stype, name, e.n_offset, e.n_value, e.n_scnum, e.n_type,
                             e.n_sclass, e.n_numaux);
                }
            }

            if strings.string_count > 0 {
                println!();
                println!("Strings Table:");
                println!("     data_size:       {}", strings.data_size);
                println!("     string_count:    {}", strings.string_count);
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
