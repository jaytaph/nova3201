#[allow(unused)]
// Exception codes
pub mod cause {
    /// Illegal opcode detected
    pub const ILLEGAL_OP: u32 = 0x00;
    /// Misaligned memory access
    pub const MISALIGNED_ACCESS: u32 = 0x01;
    /// Division by zero
    pub const DIVIDE_BY_ZERO: u32 = 0x02;
    /// Breakpoint reached
    pub const BREAKPOINT: u32 = 0x03;
    /// System call invoked
    pub const SYSTEM_CALL: u32 = 0x04;

    /// Timer interrupt
    pub const TIMER1_IRQ: u32 = 0x100;
    pub const TIMER2_IRQ: u32 = 0x101;
    /// UART interrupt
    pub const UART_IRQ: u32 = 0x102;
}

pub mod opcode {
    // ALU operation codes
    pub const ADD: u8 = 0x00; // Addition
    pub const SUB: u8 = 0x01; // Subtract
    pub const AND: u8 = 0x02; // And
    pub const OR: u8 = 0x03; // Or
    pub const XOR: u8 = 0x04; // Exclusive Or
    pub const SLT: u8 = 0x05; // Set Less Than
    pub const SLTU: u8 = 0x06; // Set Less Than Unsigned
    pub const SHL: u8 = 0x07; // Shift left
    pub const SHR: u8 = 0x08; // Shift right
    pub const SAR: u8 = 0x09; // Shift arithmetic right

    // ALU immediate
    pub const ADDI: u8 = 0x10; // Add immediate
    pub const ANDI: u8 = 0x11; // and immediate
    pub const ORI: u8 = 0x12; // or immediate
    pub const XORI: u8 = 0x13; // xor immediate
    pub const SLTI: u8 = 0x14; // set less than immediate
    pub const SLTIU: u8 = 0x15; // set less than immediate unsigned
    pub const LUI: u8 = 0x16; // Load upper immediate

    // Load / store
    pub const LW: u8 = 0x18; // Load word
    pub const SW: u8 = 0x19; // Store word
    pub const LB: u8 = 0x1A; // Load byte
    pub const SB: u8 = 0x1B; // Store byte

    // Branch
    pub const BEQ: u8 = 0x20; // Branch if equal
    pub const BNE: u8 = 0x21; // Branch if not equal
    pub const BLT: u8 = 0x22; // Branch if less than
    pub const BGE: u8 = 0x23; // Branch if greater than or equal

    // Jumps and calls
    pub const J: u8 = 0x28; // Jump
    pub const JAL: u8 = 0x29; // Jump and link
    pub const JR: u8 = 0x2A; // Jump register
    pub const JALR: u8 = 0x2B; // Jump and link register

    // System / misc
    pub const NOP: u8 = 0x3E;
    pub const HALT: u8 = 0x3F;
}

pub fn op_str(opcode: u8) -> &'static str {
    match opcode {
        opcode::ADD => "ADD",
        opcode::SUB => "SUB",
        opcode::AND => "AND",
        opcode::OR => "OR",
        opcode::XOR => "XOR",
        opcode::SLT => "SLT",
        opcode::SLTU => "SLTU",
        opcode::SHL => "SHL",
        opcode::SHR => "SHR",
        opcode::SAR => "SAR",
        opcode::ADDI => "ADDI",
        opcode::ANDI => "ANDI",
        opcode::ORI => "ORI",
        opcode::XORI => "XORI",
        opcode::SLTI => "SLTI",
        opcode::SLTIU => "SLTIU",
        opcode::LUI => "LUI",
        opcode::LW => "LW",
        opcode::SW => "SW",
        opcode::LB => "LB",
        opcode::SB => "SB",
        opcode::BEQ => "BEQ",
        opcode::BNE => "BNE",
        opcode::BLT => "BLT",
        opcode::BGE => "BGE",
        opcode::J => "J",
        opcode::JAL => "JAL",
        opcode::JR => "JR",
        opcode::JALR => "JALR",
        opcode::NOP => "NOP",
        opcode::HALT => "HALT",
        _ => "UNKNOWN",
    }
}
