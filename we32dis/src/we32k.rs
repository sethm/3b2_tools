use std::collections::HashMap;

#[derive (Debug)]
pub struct Opcode {
    pub op:       u16,
    pub mnemonic: &'static str,     // Mnemonic, e.g. "MOVB"
    pub argc:     u8,               // Number of arguments
}

pub fn get_opcodes() -> HashMap<u16,Opcode> {
    let mut opcodes = HashMap::new();

    opcodes.insert(0x84, Opcode {op: 0x84, mnemonic: "MOVW", argc: 2});
    opcodes.insert(0x82, Opcode {op: 0x82, mnemonic: "MOVH", argc: 2});
    opcodes.insert(0x80, Opcode {op: 0x81, mnemonic: "MOVB", argc: 2});

    return opcodes;
}
