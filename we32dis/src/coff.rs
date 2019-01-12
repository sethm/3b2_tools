///
/// WE32000 COFF File Parsing and Utilities
///

use std::fmt;
use std::io::Cursor;
use std::io;
use std::io::{Read, Seek, SeekFrom};

use chrono::prelude::*;
use chrono::TimeZone;

use byteorder::{BigEndian, ReadBytesExt};

// WE32000 without transfer vector
const MAGIC_WE32K: u16 = 0x170;

// WE32000 with transfer vector
const MAGIC_WE32K_TV: u16 = 0x171;

// The optional header (if present) is 28 bytes long
const OPT_HEADER_SIZE: u16 = 0x1c;

bitflags! {
    pub struct Flags: u16 {
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
        const F_PATCH = 0x2000;
        // (minimal file only) no decision functions for replaced functions
        const F_NODF = 0x2000;
    }
}



pub struct FileHeader {
    pub magic: u16,
    pub section_count: u16,
    pub timestamp: u32,
    pub symbols_pointer: u32,
    pub symbol_count: u32,
    pub opt_header: u16,
    pub flags: Flags,
}

impl fmt::Debug for FileHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut desc = String::new();

        let magic = match self.magic {
            MAGIC_WE32K => "WE32000 COFF",
            MAGIC_WE32K_TV => "WE32000 COFF (TV)",
            _ => "Unknown"
        };

        desc.push_str(magic);

        if self.flags.contains(Flags::EXECUTABLE) {
            desc.push_str(" executable");
        }

        if self.flags.contains(Flags::LSYM_STRIPPED) {
            desc.push_str(", symbols stripped");
        } else {
            desc.push_str(", with symbols");
        }

        if self.flags.contains(Flags::REL_STRIPPED) {
            desc.push_str(", relocation info stripped");
        } else {
            desc.push_str(", with relocation info");
        }


        write!(f, "{}", desc)
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

pub struct RelocationEntry {
    pub vaddr: u32,
    pub symndx: u32,
    pub rtype: u16,
}

pub struct Section {
    pub header: SectionHeader,
    pub relocation_entries: Vec<RelocationEntry>,
}

pub struct MetaData {
    pub header: FileHeader,
    pub timestamp: DateTime<Utc>,
    pub opt_header: Option<OptionalHeader>,
    pub sections: Vec<Section>,
}

impl MetaData {
    pub fn read(cursor: &mut Cursor<&[u8]>) -> io::Result<MetaData> {
        // FIRST PASS: Pull out File Header, Optional Header (if any),
        // and Section Headers

        cursor.seek(SeekFrom::Start(0))?;

        let header = FileHeader {
            magic: cursor.read_u16::<BigEndian>()?,
            section_count: cursor.read_u16::<BigEndian>()?,
            timestamp: cursor.read_u32::<BigEndian>()?,
            symbols_pointer: cursor.read_u32::<BigEndian>()?,
            symbol_count: cursor.read_u32::<BigEndian>()?,
            opt_header: cursor.read_u16::<BigEndian>()?,
            flags: Flags::from_bits_truncate(cursor.read_u16::<BigEndian>()?),
        };

        let opt_header = if header.opt_header == OPT_HEADER_SIZE {
            Some(
                OptionalHeader {
                    magic: cursor.read_u16::<BigEndian>()?,
                    version_stamp: cursor.read_u16::<BigEndian>()?,
                    text_size: cursor.read_u32::<BigEndian>()?,
                    dsize: cursor.read_u32::<BigEndian>()?,
                    bsize: cursor.read_u32::<BigEndian>()?,
                    entry_point: cursor.read_u32::<BigEndian>()?,
                    text_start: cursor.read_u32::<BigEndian>()?,
                    data_start: cursor.read_u32::<BigEndian>()?
                }
            )
        } else {
            None
        };

        let mut section_headers: Vec<SectionHeader> = vec!();

        for _ in 0..header.section_count {
            let mut name: [u8; 8] = [0; 8];
            cursor.read_exact(&mut name)?;

            let sec_header = SectionHeader {
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

            section_headers.push(sec_header);
        }

        // SECOND PASS: Now that we have decoded the section headers,
        // let's decode the section relocation tables.

        let mut sections: Vec<Section> = vec!();

        for header in section_headers {
            let mut relocation_entries: Vec<RelocationEntry> = vec!();

            if header.nreloc > 0 {
                let offset = header.relptr;

                cursor.seek(SeekFrom::Start(u64::from(offset)))?;

                for _ in 0..header.nreloc {
                    let entry = RelocationEntry {
                        vaddr: cursor.read_u32::<BigEndian>()?,
                        symndx: cursor.read_u32::<BigEndian>()?,
                        rtype: cursor.read_u16::<BigEndian>()?,
                    };

                    relocation_entries.push(entry);
                }
            }

            let section = Section {
                header,
                relocation_entries,
            };

            sections.push(section);
        }

        // FINAL TOUCHUPS

        let timestamp = Utc.timestamp(i64::from(header.timestamp), 0);

        let metadata = MetaData {
            header,
            timestamp,
            opt_header,
            sections,
        };

        Ok(metadata)
    }
}
