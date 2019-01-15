///
/// WE32000 COFF File Parsing and Utilities
///

use std::str::Utf8Error;
use std::fmt;
use std::io::Cursor;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::marker::PhantomData;
use std::str;

use crate::errors::{CoffError, ReadResult, OffsetError};

use chrono::prelude::*;
use chrono::TimeZone;

use byteorder::{BigEndian, ReadBytesExt};

// WE32000 without transfer vector
const MAGIC_WE32K: u16 = 0x170;

// WE32000 with transfer vector
const MAGIC_WE32K_TV: u16 = 0x171;

// Size of the file header
const FILE_HEADER_SIZE: u16 = 20;

// Length of old COFF version symbol names
const SYM_NAME_LEN: usize = 8;

// Maximum size, in bytes, of auxiliary symbol file names.
// const SYM_FILNMLEN: usize = 14;

// File Header flags
bitflags! {
    pub struct FileHeaderFlags: u16 {
        // Relocation info stripped from file
        const REL_STRIPPED = 0x0001;
        // File is executable (i.e. no unresolved external references)
        const EXECUTABLE = 0x0002;
        // Line numbers stripped from file
        const LINE_NUM_STRIPPED = 0x0004;
        // Local symbols stripped from file
        const LSYM_STRIPPED = 0x0010;
        // This is a minimal object file (".m") output of fextract
        const MINMAL_OBJECT = 0x0020;
        // This is a fully bound update file, output of ogen
        const UPDATE_FILE = 0x0040;
        // This file has had its bytes swabbed (in names)
        const SWABBED = 0x0100;
        // This file has the byte ordering of an AR16WR (e.g. 11/70) machine
        const BYTES_AR16WR = 0x0200;
        // This file has the byte ordering of an AR32WR machine (e.g. vax)
        const BYTES_AR32WR = 0x0400;
        // This file has the byte ordering of an AR32W machine (e.g. 3b, maxi)
        const BYTES_AR32W = 0x1000;
        // File contains "patch" list in optional header
        const PATCH_LIST = 0x2000;
        // (minimal file only) no decision functions for replaced functions
        const NO_DECISION_FUNCTIONS = 0x2000;
    }
}

pub struct FileHeader {
    pub magic: u16,
    pub section_count: u16,
    pub timestamp: u32,
    pub datetime: DateTime<Utc>,
    pub symbol_table_offset: u32,
    pub symbol_count: u32,
    pub opt_header: u16,
    pub flags: FileHeaderFlags,
}

fn buf_to_str(buf: &[u8]) -> Result<&str, std::str::Utf8Error> {
    let nul = buf.iter().position( |&c| c == b'\0').unwrap_or(buf.len());
    str::from_utf8(&buf[0..nul])
}

impl FileHeader {
    ///
    /// Read a FileHeader from the current cursor position.
    ///

    pub fn read(cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let magic = cursor.read_u16::<BigEndian>()?;
        let section_count = cursor.read_u16::<BigEndian>()?;
        let timestamp = cursor.read_u32::<BigEndian>()?;
        let symbol_table_offset = cursor.read_u32::<BigEndian>()?;
        let symbol_count = cursor.read_u32::<BigEndian>()?;
        let opt_header = cursor.read_u16::<BigEndian>()?;
        let flags = FileHeaderFlags::from_bits_truncate(cursor.read_u16::<BigEndian>()?);
        let datetime = Utc.timestamp(i64::from(timestamp), 0);

        let header = FileHeader {
            magic,
            section_count,
            timestamp,
            datetime,
            symbol_table_offset,
            symbol_count,
            opt_header,
            flags,
        };

        Ok(header)
    }
}

impl fmt::Debug for FileHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "COFF File Header:")?;

        let magic = match self.magic {
            MAGIC_WE32K => "WE32000 COFF",
            MAGIC_WE32K_TV => "WE32000 COFF (TV)",
            _ => "Unknown"
        };

        write!(f, "    {}", magic)?;

        if self.flags.contains(FileHeaderFlags::EXECUTABLE) {
            write!(f, " executable")?;
        }

        if self.flags.contains(FileHeaderFlags::LSYM_STRIPPED) {
            write!(f, ", symbols stripped")?;
        } else {
            write!(f, ", with symbols")?;
        }

        if self.flags.contains(FileHeaderFlags::REL_STRIPPED) {
            write!(f, ", relocation info stripped")?;
        } else {
            write!(f, ", with relocation info")?;
        }

        writeln!(f, ".")?;

        writeln!(f, "    Magic Number:  0x{:04x}", self.magic)?;
        writeln!(f, "    Num Sections:  {}", self.section_count)?;
        writeln!(f, "    Date:          {}", self.datetime.to_rfc2822())?;
        writeln!(f, "    Symbols Ptr:   0x{:x}", self.symbol_table_offset)?;
        writeln!(f, "    Symbol Count:  {}", self.symbol_count)?;
        writeln!(f, "    Opt Hdr:       {:?}", self.opt_header == 0x1c)?;
        write!(f, "    Flags:         0x{:04x}", self.flags)
    }
}

// Only present in the file if the file header's opt_header == 0x1c (28 bytes)
pub struct OptionalHeader {
    pub magic: u16,
    pub version_stamp: u16,
    pub text_size: u32,
    pub dsize: u32,
    pub bsize: u32,
    pub entry_point: u32,
    pub text_start: u32,
    pub data_start: u32,
}

impl OptionalHeader {
    pub fn read(cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let header = OptionalHeader {
            magic: cursor.read_u16::<BigEndian>()?,
            version_stamp: cursor.read_u16::<BigEndian>()?,
            text_size: cursor.read_u32::<BigEndian>()?,
            dsize: cursor.read_u32::<BigEndian>()?,
            bsize: cursor.read_u32::<BigEndian>()?,
            entry_point: cursor.read_u32::<BigEndian>()?,
            text_start: cursor.read_u32::<BigEndian>()?,
            data_start: cursor.read_u32::<BigEndian>()?
        };

        Ok(header)
    }
}

impl fmt::Debug for OptionalHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Optional Header:")?;
        writeln!(f, "    Magic Number:    0x{:04x}", self.magic)?;
        writeln!(f, "    Version Stamp:   0x{:04x}", self.version_stamp)?;
        writeln!(f, "    Text Size:       0x{:x}", self.text_size)?;
        writeln!(f, "    dsize:           0x{:x}", self.dsize)?;
        writeln!(f, "    bsize:           0x{:x}", self.bsize)?;
        writeln!(f, "    Entry Point:     0x{:x}", self.entry_point)?;
        writeln!(f, "    Text Start:      0x{:x}", self.text_start)?;
        write!(f, "    Data Start:      0x{:x}", self.data_start)
    }
}

pub struct SectionHeader {
    pub name: [u8; 8],
    pub paddr: u32,
    pub vaddr: u32,
    pub size: u32,
    pub scnptr: u32,
    pub relptr: u32,
    pub lnnoptr: u32,
    pub nreloc: u16,
    pub nlnno: u16,
    pub flags: u32,
}

impl SectionHeader {
    pub fn read(cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let mut name: [u8; 8] = [0; 8];
        cursor.read_exact(&mut name)?;

        let header = SectionHeader {
            name: name,
            paddr: cursor.read_u32::<BigEndian>()?,
            vaddr: cursor.read_u32::<BigEndian>()?,
            size: cursor.read_u32::<BigEndian>()?,
            scnptr: cursor.read_u32::<BigEndian>()?,
            relptr: cursor.read_u32::<BigEndian>()?,
            lnnoptr: cursor.read_u32::<BigEndian>()?,
            nreloc: cursor.read_u16::<BigEndian>()?,
            nlnno: cursor.read_u16::<BigEndian>()?,
            flags: cursor.read_u32::<BigEndian>()?,
        };

        Ok(header)
    }
}

impl fmt::Debug for SectionHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = buf_to_str(&self.name).unwrap_or("???");

        writeln!(f, "Section Header:")?;
        writeln!(f, "    Name:              {}", name)?;
        writeln!(f, "    Phys. Addr:        0x{:x}", self.paddr)?;
        writeln!(f, "    Virtual Addr:      0x{:x}", self.vaddr)?;
        writeln!(f, "    Sec. Size:         0x{:x}", self.size)?;
        writeln!(f, "    Data Offset:       0x{:x}", self.scnptr)?;
        writeln!(f, "    Rel. Tab. Offset:  0x{:x}", self.relptr)?;
        writeln!(f, "    Line Num. Offset:  0x{:x}", self.lnnoptr)?;
        writeln!(f, "    Rel. Tab. Entries: {}", self.nreloc)?;
        writeln!(f, "    Line Num. Entries: {}", self.nlnno)?;
        write!(f, "    Flags:             0x{:08x}", self.flags)
    }
}

/// Representation of a Relocation Table Entry
pub struct RelocationEntry {
    pub vaddr: u32,
    pub symndx: u32,
    pub rtype: u16,
}

/// Representation of a Symbol Table Entry
pub enum SymbolTableEntry {
    Primary {
        // Primary Symbol Data
        //
        //   8 bytes: _n {
        //       8 bytes: n_name
        //   } OR {
        //       4 bytes: n_zeroes
        //       4 bytes: n_offset
        //   } OR {
        //       4 bytes: n_nptr[0]
        //       4 bytes: n_nptr[1]
        //   }
        //   4 bytes: n_value
        //   2 bytes: n_scnum
        //   2 bytes: n_type
        //   1 byte:  n_sclass
        //   1 byte:  n_numaux
        //   ------------------
        //   18 bytes total
        n_name: [u8; SYM_NAME_LEN],
        n_zeroes: u32, // may also be n_nptr[0] for overlaying
        n_offset: u32, // may also be n_nptr[1] for overlaying
        n_value: u32,
        n_scnum: i16,
        n_type: u16,
        n_sclass: u8,
        n_numaux: u8,
    },
    Auxiliary {
        // Auxiliary Symbol Data
        //
        //   This is a huge mess because of all the unioning going on. We
        //   just need to deal with it and destrcture the data.
        //
        //   4 bytes: x_tagndx
        //   4 bytes: x_misc {
        //       2 bytes: x_lnno
        //       2 bytes: x_size
        //   } OR {
        //       4 bytes: x_fsize
        //   }
        //   8 bytes: x_fcnary {
        //       4 bytes: x_lnnoptr
        //       4 bytes: x_endndx
        //   } OR {
        //       8 bytes: x_dimen[u16; 4]
        //   }
        //   2 bytes: x_tvndx
        //   ------------------
        //   18 bytes total
        x_tagndx: u32,
        x_lnno: u16,        // Decl. line number
        x_size: u16,        // Str, union, array size
        x_fsize: u32,       // Size of function
        x_lnnoptr: u32,     // Ptr to fcn line #
        x_endndx: u32,      // Entry ndx past block end
        x_dimen: [u16; 4],  // Up to 4 array dimen.
        x_tvndx: u16,       // TV index
    }
}


pub struct StringTable<'s> {
    pub data: Vec<u8>,
    pub data_size: u32,
    pub string_count: u32,
    pub strings: Vec<(u32, String)>,
    phantom: PhantomData<&'s str>,
}

impl<'s> StringTable<'s> {
    pub fn read(cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let mut data: Vec<u8> = vec!();

        // The first four bytes of data are ALWAYS zeroed.
        let mut pad: Vec<u8> = vec!(0, 0, 0, 0);
        data.append(&mut pad);

        // ... and therefore, the string start index is always
        // initialized to 4.
        let mut i: usize = 4;

        // Denormalize the strings as we parse them.
        let mut strings: Vec<(u32, String)> = vec!();

        let data_size = cursor.read_u32::<BigEndian>()?;

        for j in 4..data_size as usize {
            let c = cursor.read_u8()?;
            data.push(c);
            if c == 0 {
                // Push from the last start to here.
                let s = buf_to_str(&data[i..j]).unwrap_or("???");
                strings.push((i as u32, s.to_owned()));
                i = j + 1usize;
            }
        }

        let table = StringTable {
            data,
            data_size,
            string_count: strings.len() as u32,
            strings,
            phantom: PhantomData,
        };

        Ok(table)
    }

    pub fn string_at(&'s self, index: u32) -> Result<&'s str, Utf8Error> {
        let start = index as usize;

        // Index into the vector at the appropriate location, and then
        // find the first nul.
        let nul = self.data[start.. ].iter()
            .position( |&c| c == b'\0')
            .unwrap_or(self.data.len() - start);

        let end = start + nul;

        let s = &self.data[start..end];

        str::from_utf8(&s)
    }
}

pub struct Section {
    pub header: SectionHeader,
    pub relocation_table: Vec<RelocationEntry>,
    pub data: Vec<u8>,
}

pub struct FileContainer<'s> {
    pub header: FileHeader,
    pub opt_header: Option<OptionalHeader>,
    pub sections: Vec<Section>,
    pub symbols: Vec<SymbolTableEntry>,
    pub strings: StringTable<'s>,
}

impl<'s> FileContainer<'s> {

    ///
    /// Read in and destructure a WE32100 COFF file.
    ///

    fn bad_metadata(header: &FileHeader) -> bool {
        !(header.magic == MAGIC_WE32K || header.magic == MAGIC_WE32K_TV)
    }

    fn read_sections(file_header: &FileHeader, cursor: &mut Cursor<&[u8]>) -> io::Result<Vec<Section>> {
        let mut section_headers: Vec<SectionHeader> = vec!();

        // Read the section headers
        for _ in 0..file_header.section_count {
            section_headers.push(SectionHeader::read(cursor)?);
        }

        // Build up the section structures
        let mut sections: Vec<Section> = vec!();

        for header in section_headers {
            let mut relocation_table: Vec<RelocationEntry> = vec!();
            let mut data: Vec<u8> = vec!();

            // Get relocation information
            if header.nreloc > 0 {
                cursor.seek(SeekFrom::Start(u64::from(header.relptr)))?;

                for _ in 0..header.nreloc {
                    let entry = RelocationEntry {
                        vaddr: cursor.read_u32::<BigEndian>()?,
                        symndx: cursor.read_u32::<BigEndian>()?,
                        rtype: cursor.read_u16::<BigEndian>()?,
                    };
                    relocation_table.push(entry);
                }
            }

            // Get data
            if header.size > 0 {
                cursor.seek(SeekFrom::Start(u64::from(header.scnptr)))?;

                for _ in 0..header.size {
                    data.push(cursor.read_u8()?);
                }
            }

            // Done with this section.
            let section = Section {
                header,
                relocation_table,
                data,
            };

            sections.push(section);
        }

        Ok(sections)
    }

    fn read_symbol_table(header: &FileHeader, cursor: &mut Cursor<&[u8]>) -> io::Result<Vec<SymbolTableEntry>> {
        let mut symbols: Vec<SymbolTableEntry> = vec!();

        if header.symbol_count > 0 {
            cursor.seek(SeekFrom::Start(u64::from(header.symbol_table_offset)))?;

            // Keep track of which symbols are aux symbols.
            let mut is_aux = false;
            let mut aux_index: u8 = 0;
            let mut raw_data: [u8; 18] = [0; 18];

            for _ in 0..header.symbol_count {

                // Consume 18 bytes.
                cursor.read_exact(&mut raw_data)?;

                let symbol_table_entry = match is_aux {
                    true => {
                        aux_index -= 1;

                        if aux_index == 0 {
                            is_aux = false;
                        }

                        let mut x_dimen: [u16; 4] = Default::default();

                        // let x_tagndx = read_u32(&raw_data[0..4]);
                        let x_tagndx = (&raw_data[0..4]).read_u32::<BigEndian>()?;
                        let x_lnno = (&raw_data[4..6]).read_u16::<BigEndian>()?;
                        let x_size = (&raw_data[6..8]).read_u16::<BigEndian>()?;
                        let x_fsize = (&raw_data[4..8]).read_u32::<BigEndian>()?;
                        let x_lnnoptr = (&raw_data[8..12]).read_u32::<BigEndian>()?;
                        let x_endndx = (&raw_data[12..16]).read_u32::<BigEndian>()?;
                        x_dimen[0] = (&raw_data[8..10]).read_u16::<BigEndian>()?;
                        x_dimen[1] = (&raw_data[10..12]).read_u16::<BigEndian>()?;
                        x_dimen[2] = (&raw_data[12..14]).read_u16::<BigEndian>()?;
                        x_dimen[3] = (&raw_data[14..16]).read_u16::<BigEndian>()?;
                        let x_tvndx = (&raw_data[16..18]).read_u16::<BigEndian>()?;

                        SymbolTableEntry::Auxiliary {
                            x_tagndx,
                            x_lnno,
                            x_size,
                            x_fsize,
                            x_lnnoptr,
                            x_endndx,
                            x_dimen,
                            x_tvndx,
                        }
                    },
                    false => {
                        let mut n_name: [u8; SYM_NAME_LEN] = Default::default();
                        n_name.copy_from_slice(&raw_data[0..8]);
                        let n_zeroes = (&raw_data[0..4]).read_u32::<BigEndian>()?;
                        let n_offset = (&raw_data[4..8]).read_u32::<BigEndian>()?;
                        let n_value = (&raw_data[8..12]).read_u32::<BigEndian>()?;
                        let n_scnum = (&raw_data[12..14]).read_i16::<BigEndian>()?;
                        let n_type = (&raw_data[14..16]).read_u16::<BigEndian>()?;
                        let n_sclass = raw_data[16];
                        let n_numaux = raw_data[17];

                        if n_numaux > 0 {
                            is_aux = true;
                            aux_index = n_numaux;
                        }

                        SymbolTableEntry::Primary {
                            n_name,
                            n_zeroes,
                            n_offset,
                            n_value,
                            n_scnum,
                            n_type,
                            n_sclass,
                            n_numaux,
                        }
                    },
                };

                symbols.push(symbol_table_entry);
            }
        }

        Ok(symbols)
    }

    ///
    /// Consume the buffer
    ///
    pub fn read(buf: &[u8]) -> ReadResult<Self> {
        let mut cursor = Cursor::new(buf);

        // Read the file header.
        let header = match FileHeader::read(&mut cursor) {
            Ok(h) => {
                if FileContainer::bad_metadata(&h) {
                    return Err(CoffError::BadFileHeader)
                } else {
                    h
                }
            },
            Err(_) => return Err(CoffError::BadFileHeader)
        };

        // If an optional header is indicated in the file header, read
        // it.
        let opt_header = if header.opt_header > 0 {
            match OptionalHeader::read(&mut cursor) {
                Ok(h) => Some(h),
                Err(_) => return Err(CoffError::BadOptionalHeader)
            }
        } else {
            None
        };

        // Now we have to seek to the sections area.
        if let Err(_) = cursor.seek(SeekFrom::Start(u64::from(FILE_HEADER_SIZE + header.opt_header))) {
            return Err(CoffError::BadSections)
        }

        // Read sections
        let sections = match FileContainer::read_sections(&header, &mut cursor) {
            Ok(s) => s,
            Err(_) => return Err(CoffError::BadSections)
        };

        // Load symbols
        let symbols = match FileContainer::read_symbol_table(&header, &mut cursor) {
            Ok(s) => s,
            Err(_) => return Err(CoffError::BadSymbols)
        };

        // The cursor is now at the correct position to read string entries.
        let strings = match StringTable::read(&mut cursor) {
            Ok(s) => s,
            Err(_) => return Err(CoffError::BadStrings)
        };

        let container = FileContainer {
            header,
            opt_header,
            sections,
            symbols,
            strings,
        };

        Ok(container)
    }

    ///
    /// Dump relocation table from the specified section to stdout.
    ///

    pub fn dump_relocation_table(&self, sec_num: usize) -> Result<(), OffsetError> {
        if self.sections.len() == 0 || sec_num > (self.sections.len() - 1) {
            return Err(OffsetError)
        }

        let section = &self.sections[sec_num];

        println!("    Relocation Table:");

        // If there is relocation data, let's dump that too.
        if section.relocation_table.len() > 0 {
            println!("        Num    Vaddr       Symndx  Type");
            println!("        -----  ----------  ------  ----");
            for (i, entry) in section.relocation_table.iter().enumerate() {
                println!("        [{:03}]  0x{:08x}  {:6}  {:3}",
                         i,  entry.vaddr, entry.symndx, entry.rtype);
            }
        } else {
            println!("       No Entries.")
        }

        return Ok(())
    }

    ///
    /// Dump section data from the specified section to stdout.
    ///
    pub fn dump_section_data(&self, sec_num: usize) -> Result<(), OffsetError> {
        if self.sections.len() == 0 || sec_num > (self.sections.len() - 1) {
            return Err(OffsetError)
        }

        let section = &self.sections[sec_num];
        let header = &section.header;
        let sec_name = buf_to_str(&header.name).unwrap_or("???");

        println!("    Section Data (number {}, name {}):", sec_num, sec_name);

        if section.data.len() == 0 {
            println!("        No Data.");
            return Ok(())
        }

        // Make a cute little array for our read data.
        let mut row_bytes: [u8; 16] = [0; 16];
        let end = section.data.len() - 1;

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

            if (i + 1) % 16 == 0 || i == end {

                // How many empty characters do we need to pad out
                // before the summary?
                let spaces = if i == end {
                    15 - (end % 16)
                } else {
                    0
                };

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

        Ok(())
    }

    ///
    /// Dump section data from the specified section to stdout.
    ///
    pub fn dump_symbol_table(&self) {
        println!("Symbol Table:");

        if self.symbols.is_empty() {
            println!("    No Entries");
            return;
        }

        for (i, e) in self.symbols.iter().enumerate() {

            match e {
                SymbolTableEntry::Primary {
                    n_name,
                    n_zeroes,
                    n_offset,
                    n_value,
                    n_scnum,
                    n_type,
                    n_sclass,
                    n_numaux,
                } => {
                    let (name, sclass) = if *n_zeroes == 0 {
                        (self.strings.string_at(*n_offset).unwrap_or("???"), "m2")
                    } else {
                        (buf_to_str(n_name).unwrap_or("???"), "m1")
                    };

                    println!("    [{:4}] {:2} name='{}' value=0x{:x} scnum={} type={:x} sclass={:x} numaux={}",
                             i, sclass, name, n_value, n_scnum, n_type, n_sclass, n_numaux);

                },
                SymbolTableEntry::Auxiliary {
                    x_tagndx,
                    x_lnno,
                    x_size,
                    x_fsize,
                    x_lnnoptr,
                    x_endndx,
                    x_dimen,
                    x_tvndx,
                } => {
                    print!("    [{:4}]  a tagndx={} lnno=0x{:04x} size=0x{:04x} fsize=0x{:04x} ",
                           i, x_tagndx, x_lnno, x_size, x_fsize);

                    println!("lnnoptr=0x{:08x} endndx={} dimen0={} dimen2={} tvndx={}",
                             x_lnnoptr, x_endndx, x_dimen[0], x_dimen[1], x_tvndx);
                }
            }
        }
    }

    pub fn dump_strings_table(&self) {
        println!("Strings Table:");

        let strings = &self.strings;

        if strings.string_count > 0 {
            for s_tuple in &strings.strings {
                println!("    [{:4}]    {}", s_tuple.0, s_tuple.1);
            }
        } else {
            println!("    No Strings");
        }
    }
}
