///
/// WE32000 COFF File Parsing and Utilities
///

use std::str;
use std::str::Utf8Error;
use std::fmt;
use std::io::Cursor;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::marker::PhantomData;
use crate::errors::{CoffError, Result};

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

impl FileHeader {

    ///
    /// Read a FileHeader from the current cursor position.
    ///

    pub fn read(cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let header = FileHeader {
            magic: cursor.read_u16::<BigEndian>()?,
            section_count: cursor.read_u16::<BigEndian>()?,
            timestamp: cursor.read_u32::<BigEndian>()?,
            symbols_pointer: cursor.read_u32::<BigEndian>()?,
            symbol_count: cursor.read_u32::<BigEndian>()?,
            opt_header: cursor.read_u16::<BigEndian>()?,
            flags: Flags::from_bits_truncate(cursor.read_u16::<BigEndian>()?),
        };

        Ok(header)
    }
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

/// Representation of a Relocation Table Entry
pub struct RelocationEntry {
    pub vaddr: u32,
    pub symndx: u32,
    pub rtype: u16,
}

/// Representation of a Symbol Table Entry
pub struct SymbolEntry {
    pub n_name: [u8; SYM_NAME_LEN],
    pub n_zeroes: u32,
    pub n_offset: u32,
    pub n_value: u32,
    pub n_scnum: i16,
    pub n_type: u16,
    pub n_sclass: u8,
    pub n_numaux: u8,
    pub is_aux: bool,
}

pub struct StringTable<'s> {
    pub data: Vec<u8>,
    pub data_size: u32,
    pub string_count: u32,
    phantom: PhantomData<&'s str>,
}

impl<'s> StringTable<'s> {
    pub fn read(cursor: &mut Cursor<&[u8]>) -> io::Result<Self> {
        let mut data: Vec<u8> = vec!();
        // The first four bytes of data are ALWAYS zeroed.
        let mut pad: Vec<u8> = vec!(0, 0, 0, 0);
        data.append(&mut pad);
        // ... and therefore, the index is always initialized to 4.
        let mut index: u32 = 4;
        let mut string_count: u32 = 0;
        let data_size = cursor.read_u32::<BigEndian>()?;

        while index < data_size {
            let c = cursor.read_u8()?;
            data.push(c);
            if c == 0 {
                string_count += 1;
            }
            index += 1;
        }

        let table = StringTable {
            data,
            data_size,
            string_count,
            phantom: PhantomData,
        };

        Ok(table)
    }

    pub fn string_at(&'s self, index: u32) -> std::result::Result<&'s str, Utf8Error> {
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
    pub relocations: Vec<RelocationEntry>,
    pub data: Vec<u8>,
}

pub struct MetaData<'s> {
    pub header: FileHeader,
    pub timestamp: DateTime<Utc>,
    pub opt_header: Option<OptionalHeader>,
    pub sections: Vec<Section>,
    pub symbols: Vec<SymbolEntry>,
    pub strings: StringTable<'s>,
}

impl<'s> MetaData<'s> {

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
            let mut relocations: Vec<RelocationEntry> = vec!();
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
                    relocations.push(entry);
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
                relocations,
                data,
            };

            sections.push(section);
        }

        Ok(sections)
    }

    fn read_symbol_table(header: &FileHeader, cursor: &mut Cursor<&[u8]>) -> io::Result<Vec<SymbolEntry>> {
        let mut symbols: Vec<SymbolEntry> = vec!();

        if header.symbol_count > 0 {
            cursor.seek(SeekFrom::Start(u64::from(header.symbols_pointer)))?;

            // Keep track of which symbols are aux symbols, and which
            // are not, by tagging them with metadata.
            let mut is_aux = false;
            let mut aux_index: u8 = 0;

            for _ in 0..header.symbol_count {
                let mut name: [u8; SYM_NAME_LEN] = [0; SYM_NAME_LEN];

                cursor.read_exact(&mut name)?;

                let mut zeroes_array: [u8; 4] = [0; 4];
                let mut offset_array: [u8; 4] = [0; 4];

                zeroes_array.clone_from_slice(&name[0..4]);
                offset_array.clone_from_slice(&name[4..]);

                let symbol_entry = SymbolEntry {
                    n_name: name,
                    n_zeroes: unsafe { std::mem::transmute::<[u8; 4], u32>(zeroes_array) }.to_be().into(),
                    n_offset: unsafe { std::mem::transmute::<[u8; 4], u32>(offset_array) }.to_be().into(),
                    n_value: cursor.read_u32::<BigEndian>()?,
                    n_scnum: cursor.read_i16::<BigEndian>()?,
                    n_type: cursor.read_u16::<BigEndian>()?,
                    n_sclass: cursor.read_u8()?,
                    n_numaux: cursor.read_u8()?,
                    is_aux: is_aux,
                };

                // If we just read a numaux > 0, then the next symbol
                // we read will be an aux symbol, down to the last
                // one.

                if is_aux {
                    aux_index -= 1;
                    if aux_index == 0 {
                        is_aux = false;
                    }
                }

                if symbol_entry.n_numaux > 0 {
                    is_aux = true;
                    aux_index = symbol_entry.n_numaux;
                }

                symbols.push(symbol_entry);
            }
        }

        Ok(symbols)
    }

    pub fn read(buf: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(buf);

        // Determine if we're parsing a linked executable
        // or an object file.

        // Read the file header.
        let header = match FileHeader::read(&mut cursor) {
            Ok(h) => {
                if MetaData::bad_metadata(&h) {
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
        let sections = match MetaData::read_sections(&header, &mut cursor) {
            Ok(s) => s,
            Err(_) => return Err(CoffError::BadSections)
        };

        // Load symbols
        let symbols = match MetaData::read_symbol_table(&header, &mut cursor) {
            Ok(s) => s,
            Err(_) => return Err(CoffError::BadSymbols)
        };

        // The cursor is now at the correct position to read string entries.
        let strings = match StringTable::read(&mut cursor) {
            Ok(s) => s,
            Err(_) => return Err(CoffError::BadStrings)
        };

        // Finally, destructure the timestamp
        let timestamp = Utc.timestamp(i64::from(header.timestamp), 0);

        let metadata = MetaData {
            header,
            timestamp,
            opt_header,
            sections,
            symbols,
            strings,
        };

        Ok(metadata)
    }
}
