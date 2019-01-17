#![allow(clippy::unreadable_literal)]

use std::io::Cursor;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::errors::DecodeError;
use std::fmt;

const R_FP: usize = 9;
const R_AP: usize = 10;

const HALFWORD_MNEMONIC_COUNT: usize = 11;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum AddrMode {
    None,
    Absolute,
    AbsoluteDeferred,
    ByteDisplacement,
    ByteDisplacementDeferred,
    HalfwordDisplacement,
    HalfwordDisplacementDeferred,
    WordDisplacement,
    WordDisplacementDeferred,
    APShortOffset,
    FPShortOffset,
    ByteImmediate,
    HalfwordImmediate,
    WordImmediate,
    PositiveLiteral,
    NegativeLiteral,
    Register,
    RegisterDeferred,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OpType {
    Lit,
    Src,
    Dest,
    None,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Data {
    None,
    Byte,
    Half,
    Word,
    SByte,
    UHalf,
    UWord,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct Operand {
    size: u8,
    mode: AddrMode,
    data_type: Data,
    expanded_type: Option<Data>,
    register: Option<usize>,
    embedded: u32,
    cursor: usize,
    bytes: [u8; 32],
}

impl Operand {
    fn new(
        size: u8,
        mode: AddrMode,
        data_type: Data,
        expanded_type: Option<Data>,
        register: Option<usize>,
        embedded: u32,
    ) -> Operand {
        Operand {
            size,
            mode,
            data_type,
            expanded_type,
            register,
            embedded,
            cursor: 0,
            bytes: [0; 32],
        }
    }

    fn reset(&mut self) {
        self.cursor = 0;
    }

    fn byte_size(&self) -> u8 {
        self.cursor as u8
    }

    fn append_u8(&mut self, b: u8) {
        if self.cursor < 31 {
            self.bytes[self.cursor] = b;
            self.cursor += 1;
        }
    }

    fn append_u16(&mut self, h: u16) {
        if self.cursor < 29 {
            self.bytes[self.cursor] = (h & 0xff) as u8;
            self.bytes[self.cursor + 1] = ((h >> 8) & 0xff) as u8;
            self.cursor += 2;
        }
    }

    fn append_u32(&mut self, w: u32) {
        if self.cursor < 27 {
            self.bytes[self.cursor] = (w & 0xff) as u8            ;
            self.bytes[self.cursor + 1] = ((w >> 8) & 0xff) as u8;
            self.bytes[self.cursor + 2] = ((w >> 16) & 0xff) as u8;
            self.bytes[self.cursor + 3] = ((w >> 24) & 0xff) as u8;
            self.cursor += 4;
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        let reg_string = match self.register {
            Some(0) => "%r0",
            Some(1) => "%r1",
            Some(2) => "%r2",
            Some(3) => "%r3",
            Some(4) => "%r4",
            Some(5) => "%r5",
            Some(6) => "%r6",
            Some(7) => "%r7",
            Some(8) => "%r8",
            Some(9) => "%fp",
            Some(10) => "%ap",
            Some(11) => "%psw",
            Some(12) => "%sp",
            Some(13) => "%pcbp",
            Some(14) => "%isp",
            Some(15) => "%pc",
            _ => "%??",
        };

        match self.mode {
            AddrMode::Absolute => write!(f, "$0x{:x}", self.embedded)?,
            AddrMode::AbsoluteDeferred => write!(f, "*$0x{:x}", self.embedded)?,
            AddrMode::ByteDisplacement => write!(f, "{}({})", (self.embedded as u8) as i8, reg_string)?,
            AddrMode::ByteDisplacementDeferred => write!(f, "*{}({})", (self.embedded as u8) as i8, reg_string)?,
            AddrMode::HalfwordDisplacement => write!(f, "0x{:x}({})", self.embedded as u16, reg_string)?,
            AddrMode::HalfwordDisplacementDeferred => write!(f, "*0x{:x}({})", self.embedded as u16, reg_string)?,
            AddrMode::WordDisplacement => write!(f, "0x{:x}({})", self.embedded, reg_string)?,
            AddrMode::WordDisplacementDeferred => write!(f, "*0x{:x}({})", self.embedded, reg_string)?,
            AddrMode::APShortOffset => write!(f, "{}(%ap)", self.embedded)?,
            AddrMode::FPShortOffset => write!(f, "{}(%fp)", self.embedded)?,
            AddrMode::ByteImmediate => write!(f, "&{}", self.embedded)?,
            AddrMode::HalfwordImmediate => write!(f, "&0x{:x}", self.embedded)?,
            AddrMode::WordImmediate => write!(f, "&0x{:x}", self.embedded)?,
            AddrMode::PositiveLiteral => write!(f, "&{}", self.embedded)?,
            AddrMode::NegativeLiteral => write!(f, "&{}", (self.embedded as u8) as i8)?,
            AddrMode::Register => write!(f, "{}", reg_string)?,
            AddrMode::RegisterDeferred => write!(f, "({})", reg_string)?,
            AddrMode::None => write!(f, "{}", self.embedded)?,
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Mnemonic {
    opcode: u16,
    dtype: Data,
    name: &'static str,
    ops: [OpType; 4],
}

#[derive(Debug, Eq, PartialEq)]
pub struct Instruction {
    pub opcode: u16,
    pub name: &'static str,
    pub data_type: Data,
    pub operand_count: u8,
    pub operands: [Operand; 4],
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {

        // How many characters wide is the byte dump?
        // (At least 2)
        let mut bytes_width: i32 = 2;

        // Print instruction bytes
        write!(f, "{:02x}", self.opcode)?;

        for i in 0..self.operand_count as usize {
            let op: &Operand = &self.operands[i];

            for j in 0..op.cursor {
                write!(f, " {:02x}", op.bytes[j])?;
                bytes_width += 3;
            }
        }

        // Now compute how many spaces we need to write.

        let spaces_needed: i32 = 30 - bytes_width;

        if spaces_needed > 0 {
            for _ in 0..spaces_needed {
                write!(f, " ")?;
            }
        }

        // Now write the mnemonic
        write!(f, " | {}", self.name)?;

        let mut more_spaces: i32 = 10 - self.name.len() as i32;

        if more_spaces > 0 {
            for _ in 0..more_spaces {
                write!(f, " ")?;
            }
        }

        let op_count = self.operand_count as usize;

        for i in 0..op_count {
            write!(f, "{}", self.operands[i])?;
            if i < op_count - 1 {
                write!(f, ",")?;
            }
        }

        return Ok(())
    }
}


macro_rules! mn {
    ($opcode:expr, $dtype:expr, $name:expr, $ops:expr) => {
        Mnemonic {
            opcode: $opcode,
            dtype: $dtype,
            name: $name,
            ops: $ops,
        }
    };
}

static BYTE_MNEMONICS: [Option<Mnemonic>; 256] = [
    Some(mn!(0x00, Data::None, "halt", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x02, Data::Word, "SPOPRD", [OpType::Lit, OpType::Src, OpType::None, OpType::None])),
    Some(mn!(0x03, Data::Word, "SPOPRD2", [OpType::Lit, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0x04, Data::Word, "MOVAW", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0x06, Data::Word, "SPOPRT", [OpType::Lit, OpType::Src, OpType::None, OpType::None])),
    Some(mn!(0x07, Data::Word, "SPOPT2", [OpType::Lit, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0x08, Data::None, "RET", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    None,
    None,
    Some(mn!(0x0C, Data::Word, "MOVTRW", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    None,
    None,
    Some(mn!(0x10, Data::Word, "SAVE", [OpType::Src, OpType::None, OpType::None, OpType::None])),
    None,
    None,
    Some(mn!(0x13, Data::Word, "SPOPWD", [OpType::Lit, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0x14, Data::Byte, "EXTOP", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    None,
    Some(mn!(0x17, Data::Word, "SPOPWT", [OpType::Lit, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0x18, Data::None, "RESTORE", [OpType::Src, OpType::None, OpType::None, OpType::None])),
    None,
    None,
    None,
    Some(mn!(0x1C, Data::Word, "SWAPWI", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x1E, Data::Half, "SWAPHI", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x1F, Data::Byte, "SWAPBI", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x20, Data::Word, "POPW", [OpType::Src, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x22, Data::Word, "SPOPRS", [OpType::Lit, OpType::Src, OpType::None, OpType::None])),
    Some(mn!(0x23, Data::Word, "SPOPS2", [OpType::Lit, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0x24, Data::Word, "JMP", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    None,
    None,
    Some(mn!(0x27, Data::None, "CFLUSH", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x28, Data::Word, "TSTW", [OpType::Src, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x2A, Data::Half, "TSTH", [OpType::Src, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x2B, Data::Byte, "TSTB", [OpType::Src, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x2C, Data::Word, "CALL", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0x2E, Data::None, "BPT", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x2F, Data::None, "WAIT", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    None,
    Some(mn!(0x32, Data::Word, "SPOP", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x33, Data::Word, "SPOPWS", [OpType::Lit, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0x34, Data::Word, "JSB", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x36, Data::Half, "BSBH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x37, Data::Byte, "BSBB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x38, Data::Word, "BITW", [OpType::Src, OpType::Src, OpType::None, OpType::None])),
    None,
    Some(mn!(0x3A, Data::Half, "BITH", [OpType::Src, OpType::Src, OpType::None, OpType::None])),
    Some(mn!(0x3B, Data::Byte, "BITB", [OpType::Src, OpType::Src, OpType::None, OpType::None])),
    Some(mn!(0x3C, Data::Word, "CMPW", [OpType::Src, OpType::Src, OpType::None, OpType::None])),
    None,
    Some(mn!(0x3E, Data::Half, "CMPH", [OpType::Src, OpType::Src, OpType::None, OpType::None])),
    Some(mn!(0x3F, Data::Byte, "CMPB", [OpType::Src, OpType::Src, OpType::None, OpType::None])),
    Some(mn!(0x40, Data::None, "RGEQ", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x42, Data::Half, "BGEH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x43, Data::Byte, "BGEB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x44, Data::None, "RGTR", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x46, Data::Half, "BGH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x47, Data::Byte, "BGB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x48, Data::None, "RLSS", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x4A, Data::Half, "BLH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x4B, Data::Byte, "BLB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x4C, Data::None, "RLEQ", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x4E, Data::Half, "BLEH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x4F, Data::Byte, "BLEB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x50, Data::None, "RGEQU", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x52, Data::Half, "BGEUH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x53, Data::Byte, "BGEUB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x54, Data::None, "RGTRU", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x56, Data::Half, "BGUH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x57, Data::Byte, "BGUB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x58, Data::None, "RLSSU", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x5A, Data::Half, "BLUH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x5B, Data::Byte, "BLUB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x5C, Data::None, "RLEQU", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x5E, Data::Half, "BLEUH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x5F, Data::Byte, "BLEUB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x60, Data::None, "RVC", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x62, Data::Half, "BVCH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x63, Data::Byte, "BVCB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x64, Data::None, "RNEQU", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x66, Data::Half, "BNEH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x67, Data::Byte, "BNEB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x68, Data::None, "RVS", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x6A, Data::Half, "BVSH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x6B, Data::Byte, "BVSB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x6C, Data::None, "REQLU", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x6E, Data::Half, "BEH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x6F, Data::Byte, "BEB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x70, Data::None, "NOP", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x72, Data::None, "NOP3", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x73, Data::None, "NOP2", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x74, Data::None, "RNEQ", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x76, Data::Half, "BNEH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x77, Data::Byte, "BNEB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x78, Data::None, "RSB", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x7A, Data::Half, "BRH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x7B, Data::Byte, "BRB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x7C, Data::None, "REQL", [OpType::None, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x7E, Data::Half, "BEH", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x7F, Data::Byte, "BEB", [OpType::Lit, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x80, Data::Word, "CLRW", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x82, Data::Half, "CLRH", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x83, Data::Byte, "CLRB", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x84, Data::Word, "MOVW", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0x86, Data::Half, "MOVH", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0x87, Data::Byte, "MOVB", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0x88, Data::Word, "MCOMW", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0x8A, Data::Half, "MCOMH", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0x8B, Data::Byte, "MCOMB", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0x8C, Data::Word, "MNEGW", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0x8E, Data::Half, "MNEGH", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0x8F, Data::Byte, "MNEGB", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0x90, Data::Word, "INCW", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x92, Data::Half, "INCH", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x93, Data::Byte, "INCB", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x94, Data::Word, "DECW", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    None,
    Some(mn!(0x96, Data::Half, "DECH", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x97, Data::Byte, "DECB", [OpType::Dest, OpType::None, OpType::None, OpType::None])),
    None,
    None,
    None,
    None,
    Some(mn!(0x9C, Data::Word, "ADDW2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0x9E, Data::Half, "ADDH2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0x9F, Data::Byte, "ADDB2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xA0, Data::Word, "PUSHW", [OpType::Src, OpType::None, OpType::None, OpType::None])),
    None,
    None,
    None,
    Some(mn!(0xA4, Data::Word, "MODW2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0xA6, Data::Half, "MODH2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xA7, Data::Byte, "MODB2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xA8, Data::Word, "MULW2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0xAA, Data::Half, "MULH2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xAB, Data::Byte, "MULB2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xAC, Data::Word, "DIVW2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0xAE, Data::Half, "DIVH2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xAF, Data::Byte, "DIVB2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xB0, Data::Word, "ORW2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0xB2, Data::Half, "ORH2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xB3, Data::Byte, "ORB2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xB4, Data::Word, "XORW2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0xB6, Data::Half, "XORH2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xB7, Data::Byte, "XORB2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xB8, Data::Word, "ANDW2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0xBA, Data::Half, "ANDH2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xBB, Data::Byte, "ANDB2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xBC, Data::Word, "SUBW2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    None,
    Some(mn!(0xBE, Data::Half, "SUBH2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xBF, Data::Byte, "SUBB2", [OpType::Src, OpType::Dest, OpType::None, OpType::None])),
    Some(mn!(0xC0, Data::Word, "ALSW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    None,
    None,
    Some(mn!(0xC4, Data::Word, "ARSW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    Some(mn!(0xC6, Data::Half, "ARSH3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xC7, Data::Byte, "ARSB3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xC8, Data::Word, "INSFW", [OpType::Src, OpType::Src, OpType::Src, OpType::Dest])),
    None,
    Some(mn!(0xCA, Data::Half, "INSFH", [OpType::Src, OpType::Src, OpType::Src, OpType::Dest])),
    Some(mn!(0xCB, Data::Byte, "INSFB", [OpType::Src, OpType::Src, OpType::Src, OpType::Dest])),
    Some(mn!(0xCC, Data::Word, "EXTFW", [OpType::Src, OpType::Src, OpType::Src, OpType::Dest])),
    None,
    Some(mn!(0xCE, Data::Half, "EXTFH", [OpType::Src, OpType::Src, OpType::Src, OpType::Dest])),
    Some(mn!(0xCF, Data::Byte, "EXTFB", [OpType::Src, OpType::Src, OpType::Src, OpType::Dest])),
    Some(mn!(0xD0, Data::Word, "LLSW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    Some(mn!(0xD2, Data::Half, "LLSH3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xD3, Data::Byte, "LLSB3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xD4, Data::Word, "LRSW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    None,
    None,
    Some(mn!(0xD8, Data::Word, "ROTW", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    None,
    None,
    Some(mn!(0xDC, Data::Word, "ADDW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    Some(mn!(0xDE, Data::Half, "ADDH3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xDF, Data::Byte, "ADDB3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xE0, Data::Word, "PUSHAW", [OpType::Src, OpType::None, OpType::None, OpType::None])),
    None,
    None,
    None,
    Some(mn!(0xE4, Data::Word, "MODW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    Some(mn!(0xE6, Data::Half, "MODH3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xE7, Data::Byte, "MODB3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xE8, Data::Word, "MULW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    Some(mn!(0xEA, Data::Half, "MULH3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xEB, Data::Byte, "MULB3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xEC, Data::Word, "DIVW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    Some(mn!(0xEE, Data::Half, "DIVH3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xEF, Data::Byte, "DIVB3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xF0, Data::Word, "ORW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    Some(mn!(0xF2, Data::Half, "ORH3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xF3, Data::Byte, "ORB3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xF4, Data::Word, "XORW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    Some(mn!(0xF6, Data::Half, "XORH3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xF7, Data::Byte, "XORB3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xF8, Data::Word, "ANDW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    Some(mn!(0xFA, Data::Half, "ANDH3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xFB, Data::Byte, "ANDB3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xFC, Data::Word, "SUBW3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    None,
    Some(mn!(0xFE, Data::Half, "SUBH3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None])),
    Some(mn!(0xFF, Data::Byte, "SUBB3", [OpType::Src, OpType::Src, OpType::Dest, OpType::None]))
];


static HALFWORD_MNEMONICS: [Option<Mnemonic>; HALFWORD_MNEMONIC_COUNT] = [
    Some(mn!(0x3009, Data::None, "MVERNO", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x300d, Data::None, "ENBVJMP", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x3013, Data::None, "DISVJMP", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x3019, Data::None, "MOVBLW", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x301f, Data::None, "STREND", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x302f, Data::None, "INTACK", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x303f, Data::None, "STRCPY", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x3045, Data::None, "RETG", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x3061, Data::None, "GATE", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x30ac, Data::None, "CALLPS", [OpType::None, OpType::None, OpType::None, OpType::None])),
    Some(mn!(0x30c8, Data::None, "RETPS", [OpType::None, OpType::None, OpType::None, OpType::None]))
];

static NULL_MNEMONIC: Option<Mnemonic> = None;

pub struct Decoder {
    pub ir: Instruction,
}

impl Default for Decoder {
    fn default() -> Self {
        Decoder::new()
    }
}

impl Decoder {
    pub fn new() -> Self {
        Decoder {
            ir: Instruction {
                opcode: 0,
                name: "???",
                data_type: Data::None,
                operand_count: 0,
                operands: [
                    Operand::new(0, AddrMode::None, Data::None, None, None, 0),
                    Operand::new(0, AddrMode::None, Data::None, None, None, 0),
                    Operand::new(0, AddrMode::None, Data::None, None, None, 0),
                    Operand::new(0, AddrMode::None, Data::None, None, None, 0),
                ]
            }
        }
    }

    /// Decode a literal Operand type.
    ///
    /// These operands belong to only certain instructions, where a word without
    /// a descriptor byte immediately follows the opcode.
    fn decode_literal_operand(&mut self, cursor: &mut Cursor<&[u8]>, index: usize, mn: &Mnemonic) -> Result<(), DecodeError> {
        let op = &mut self.ir.operands[index];

        op.mode = AddrMode::None;
        op.data_type = Data::Byte;
        op.expanded_type = None;
        op.register = None;

        match mn.dtype {
            Data::Byte => {
                let b: u8 = cursor.read_u8()?;
                op.embedded = u32::from(b);
                op.append_u8(b);
            }
            Data::Half => {
                let h: u16 = cursor.read_u16::<LittleEndian>()?;
                op.embedded = u32::from(h);
                op.append_u16(h);
            }
            Data::Word => {
                let w: u32 = cursor.read_u32::<LittleEndian>()?;
                op.embedded = w;
                op.append_u32(w);
            }
            _ => return Err(DecodeError::Parse),
        }

        Ok(())
    }

    /// Decode a descriptor Operand type.
    fn decode_descriptor_operand(
        &mut self,
        cursor: &mut Cursor<&[u8]>,
        index: usize,
        dtype: Data,
        etype: Option<Data>,
        recur: bool,
    ) -> Result<(), DecodeError> {
        let op = &mut self.ir.operands[index];

        op.data_type = dtype;
        op.expanded_type = etype;

        let descriptor_byte: u8 = cursor.read_u8()?;

        op.append_u8(descriptor_byte);

        let m = (descriptor_byte & 0xf0) >> 4;
        let r = descriptor_byte & 0xf;

        match m {
            0 | 1 | 2 | 3 => {
                // Positive Literal
                op.mode = AddrMode::PositiveLiteral;
                op.register = None;
                op.embedded = u32::from(descriptor_byte);
            }
            4 => {
                match r {
                    15 => {
                        // Word Immediate
                        let w = cursor.read_u32::<LittleEndian>()?;
                        op.mode = AddrMode::WordImmediate;
                        op.register = None;
                        op.embedded = w;
                        op.append_u32(w);
                    }
                    _ => {
                        // Register
                        op.mode = AddrMode::Register;
                        op.register = Some(r as usize);
                        op.embedded = 0;
                    }
                }
            }
            5 => {
                match r {
                    15 => {
                        // Halfword Immediate
                        let h = cursor.read_u16::<LittleEndian>()?;
                        op.mode = AddrMode::HalfwordImmediate;
                        op.register = None;
                        op.embedded = u32::from(h);
                        op.append_u16(h);
                    }
                    11 => {
                        // Illegal
                        return Err(DecodeError::Parse);
                    }
                    _ => {
                        // Register Deferred Mode
                        op.mode = AddrMode::RegisterDeferred;
                        op.register = Some(r as usize);
                        op.embedded = 0;
                    }
                }
            }
            6 => {
                match r {
                    15 => {
                        // Byte Immediate
                        let b = cursor.read_u8()?;
                        op.mode = AddrMode::ByteImmediate;
                        op.register = None;
                        op.embedded = u32::from(b);
                        op.append_u8(b);
                    }
                    _ => {
                        // FP Short Offset
                        op.mode = AddrMode::FPShortOffset;
                        op.register = Some(R_FP);
                        op.embedded = u32::from(r);
                    }
                }
            }
            7 => {
                match r {
                    15 => {
                        // Absolute
                        let w = cursor.read_u32::<LittleEndian>()?;
                        op.mode = AddrMode::Absolute;
                        op.register = None;
                        op.embedded = w;
                        op.append_u32(w);
                    }
                    _ => {
                        // AP Short Offset
                        op.mode = AddrMode::APShortOffset;
                        op.register = Some(R_AP);
                        op.embedded = u32::from(r);
                    }
                }
            }
            8 => {
                match r {
                    11 => return Err(DecodeError::Parse),
                    _ => {
                        // Word Displacement
                        let disp = cursor.read_u32::<LittleEndian>()?;
                        op.mode = AddrMode::WordDisplacement;
                        op.register = Some(r as usize);
                        op.embedded = disp;
                        op.append_u32(disp);
                    }
                }
            }
            9 => {
                match r {
                    11 => return Err(DecodeError::Parse),
                    _ => {
                        // Word Displacement Deferred
                        let disp = cursor.read_u32::<LittleEndian>()?;
                        op.mode = AddrMode::WordDisplacementDeferred;
                        op.register = Some(r as usize);
                        op.embedded = disp;
                        op.append_u32(disp);
                    }
                }
            }
            10 => {
                match r {
                    11 => return Err(DecodeError::Parse),
                    _ => {
                        // Halfword Displacement
                        let disp = cursor.read_u16::<LittleEndian>()?;
                        op.mode = AddrMode::HalfwordDisplacement;
                        op.register = Some(r as usize);
                        op.embedded = u32::from(disp);
                        op.append_u16(disp);
                    }
                }
            }
            11 => {
                match r {
                    11 => return Err(DecodeError::Parse),
                    _ => {
                        // Halfword Displacement Deferred
                        let disp = cursor.read_u16::<LittleEndian>()?;
                        op.mode = AddrMode::HalfwordDisplacementDeferred;
                        op.register = Some(r as usize);
                        op.embedded = u32::from(disp);
                        op.append_u16(disp);
                    }
                }
            }
            12 => {
                match r {
                    11 => return Err(DecodeError::Parse),
                    _ => {
                        // Byte Displacement
                        let disp = cursor.read_u8()?;
                        op.mode = AddrMode::ByteDisplacement;
                        op.register = Some(r as usize);
                        op.embedded = u32::from(disp);
                        op.append_u8(disp);
                    }
                }
            }
            13 => {
                match r {
                    11 => return Err(DecodeError::Parse),
                    _ => {
                        // Byte Displacement Deferred
                        let disp = cursor.read_u8()?;
                        op.mode = AddrMode::ByteDisplacementDeferred;
                        op.register = Some(r as usize);
                        op.embedded = u32::from(disp);
                        op.append_u8(disp);
                    }
                }
            }
            14 => match r {
                0 => self.decode_descriptor_operand(cursor, index, dtype, Some(Data::UWord), true)?,
                2 => self.decode_descriptor_operand(cursor, index, dtype, Some(Data::UHalf), true)?,
                3 => self.decode_descriptor_operand(cursor, index, dtype, Some(Data::Byte), true)?,
                4 => self.decode_descriptor_operand(cursor, index, dtype, Some(Data::Word), true)?,
                6 => self.decode_descriptor_operand(cursor, index, dtype, Some(Data::Half), true)?,
                7 => self.decode_descriptor_operand(cursor, index, dtype, Some(Data::SByte), true)?,
                15 => {
                    let w = cursor.read_u32::<LittleEndian>()?;
                    op.mode = AddrMode::AbsoluteDeferred;
                    op.register = None;
                    op.embedded = w;
                    op.append_u32(w);
                }
                _ => { return Err(DecodeError::Parse); }
            },
            15 => {
                // Negative Literal
                op.mode = AddrMode::NegativeLiteral;
                op.register = None;
                op.embedded = u32::from(descriptor_byte);
            },
            _ => { return Err(DecodeError::Parse); }
        };

        Ok(())
    }

    /// Fully decode an Operand
    fn decode_operand(
        &mut self,
        cursor: &mut Cursor<&[u8]>,
        index: usize,
        mn: &Mnemonic,
        ot: OpType,
        etype: Option<Data>,
    ) -> Result<(), DecodeError> {

        self.ir.operands[index].reset();

        match ot {
            OpType::Lit => self.decode_literal_operand(cursor, index, mn),
            OpType::Src | OpType::Dest => self.decode_descriptor_operand(cursor, index, mn.dtype, etype, false),
            OpType::None => Ok(())
        }
    }

    /// Decode the instruction currently pointed at by the cursor.
    pub fn decode_instruction(&mut self, cursor: &mut Cursor<&[u8]>) -> Result<(), DecodeError> {
        // Read the first byte of the instruction. Most instructions are only
        // one byte, so this is usually enough.
        let b1 = cursor.read_u8()?;

        // Map the Mnemonic to the  opcode we just read. But there's a special
        // case if the value we read was '0x30'. This indicates that the instruction
        // we're reading is a halfword, requiring two bytes.

        let mut mn: &Option<Mnemonic> = &NULL_MNEMONIC;

        if b1 == 0x30 {
            let b2 = cursor.read_u8()?;

            let opcode = (u16::from(b1) << 8) | u16::from(b2);

            for m in &HALFWORD_MNEMONICS {
                if m.is_some() && m.as_ref().unwrap().opcode == opcode {
                    mn = m;
                    break;
                }
            }
        } else {
            mn = &BYTE_MNEMONICS[b1 as usize];
        };

        // If we found a valid mnemonic, read in and decode all of its operands.
        match mn {
            Some(mn) => {
                let mut etype: Option<Data> = None;
                let mut index: usize = 0;

                for ot in &mn.ops {
                    if *ot == OpType::None {
                        break;
                    }
                    // Push a decoded operand
                    self.decode_operand(cursor, index, mn, *ot, etype)?;
                    etype = self.ir.operands[index].expanded_type;
                    index += 1;
                }

                self.ir.opcode = mn.opcode;
                self.ir.name = mn.name;
                self.ir.operand_count = index as u8;
                self.ir.data_type = mn.dtype;
            }
            None => return Err(DecodeError::Parse),
        }

        Ok(())
    }
}
