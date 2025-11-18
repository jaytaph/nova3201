use crate::bus::Bus;
use crate::cpu::isa::op_str;
use crate::machine::IrqLines;
use std::fmt::{Debug, Formatter};

pub mod isa;

// Special register (SR) flags
pub const SR_EI: u32 = 1 << 0; // Exception In Progress
pub const SR_U: u32 = 1 << 2; // User Mode
pub const SR_IE: u32 = 1 << 4; // Interrupt Enable

pub const LINK_REGISTER: usize = 31; // Where the CPU wil store return addresses

const RESET_VECTOR: u32 = 0x0000_0000; // Reset vector where the CPU starts execution
const EXCEPTION_VECTOR: u32 = 0x0000_0100; // Exception handler vector

pub struct Cpu {
    /// Our general-purpose registers
    regs: [u32; 32],
    /// Program counter
    pc: u32,
    /// Special registers
    sr: u32,
    /// Exception Program Counter
    epc: u32,
    /// Cause of the last exception
    cause: u32,
    /// Is the CPU halted
    pub halted: bool,
}

impl Cpu {
    pub fn regs(&self) -> &[u32; 32] {
        &self.regs
    }
    pub fn pc(&self) -> u32 {
        self.pc
    }
    pub fn sr(&self) -> u32 {
        self.sr
    }
    pub fn epc(&self) -> u32 {
        self.epc
    }
    pub fn cause(&self) -> u32 {
        self.cause
    }
    pub fn halted(&self) -> bool {
        self.halted
    }
}

pub struct Instruction {
    opcode: u8,
    rd: usize,
    rs: usize,
    rt: usize,
    imm16: u16,  // Overlaps with rt
    target: u32, // Overlaps with imm16, rs, rt, rd
}

impl Debug for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ opcode: {:06}, rd: {:02}, rs: {:02}, rt: {:02}, imm16: {:04X}, target: {:08X} }}",
            op_str(self.opcode),
            self.rd,
            self.rs,
            self.rt,
            self.imm16,
            self.target
        )
    }
}

impl Instruction {
    pub fn nop() -> Self {
        Self {
            opcode: isa::opcode::NOP,
            rd: 0,
            rs: 0,
            rt: 0,
            imm16: 0,
            target: 0,
        }
    }
    pub fn decode(raw: u32) -> Self {
        let opcode = ((raw >> 26) & 0x3F) as u8;
        let rd = ((raw >> 21) & 0x1F) as usize;
        let rs = ((raw >> 16) & 0x1F) as usize;
        let rt = ((raw >> 11) & 0x1F) as usize;
        let imm16 = (raw & 0xFFFF) as u16; // Note this overlaps the bits with rt
        let target = raw & 0x03FF_FFFF; // Note this overlaps the bits with imm16, rd, rs, rt

        Self {
            opcode,
            rs,
            rt,
            rd,
            imm16,
            target,
        }
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            regs: [0; 32],
            pc: RESET_VECTOR,
            sr: 0,
            epc: 0,
            cause: 0,
            halted: false,
        }
    }

    #[inline]
    fn sign_extend_16(x: u16) -> u32 {
        (x as i16) as i32 as u32
    }

    pub fn step<B: Bus>(&mut self, bus: &mut B, irq: &IrqLines) -> Result<(), B::Error> {
        if self.halted {
            // CPU is halted; do nothing
            return Ok(());
        }

        // Store next register states
        let mut next_regs = self.regs;
        let mut next_pc = self.pc;
        let mut next_sr = self.sr;
        let mut next_epc = self.epc;
        let mut next_cause = self.cause;
        let mut next_halted = self.halted;

        let mut take_exception = false;
        let mut exc_cause = 0;
        let mut exc_pc = self.pc;

        // Fetch instruction
        let raw = bus.read32(self.pc)?;

        // Decode instruction
        let instr = Instruction::decode(raw);
        // println!("[{:08X}] Instr: {:?} (raw: {:08X})", self.pc, instr, raw);

        // Check IRQ lines for pending interrupts
        if !take_exception {
            if irq.timer1 {
                take_exception = true;
                exc_cause = isa::cause::TIMER1_IRQ;
                exc_pc = self.pc;
            } else if irq.timer2 {
                take_exception = true;
                exc_cause = isa::cause::TIMER2_IRQ;
                exc_pc = self.pc;
            } else if irq.uart {
                take_exception = true;
                exc_cause = isa::cause::UART_IRQ;
                exc_pc = self.pc;
            }
        }

        // Handle any exceptions
        if take_exception {
            next_epc = exc_pc;
            next_cause = exc_cause;
            next_pc = EXCEPTION_VECTOR;
            next_sr = self.sr & !SR_IE; // Disable interrupts
        } else {
            // println!("[{:08X}] Instr: {:?}", self.pc, instr);
            // Execute instruction
            match instr.opcode {
                // -----------------------------
                // ALU Operations
                isa::opcode::ADD => {
                    // rd = rs + rt
                    let rs_val = self.regs[instr.rs];
                    let rt_val = self.regs[instr.rt];

                    next_regs[instr.rd] = rs_val.wrapping_add(rt_val);
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::SUB => {
                    // rd = rs - rt
                    let rs_val = self.regs[instr.rs];
                    let rt_val = self.regs[instr.rt];

                    next_regs[instr.rd] = rs_val.wrapping_sub(rt_val);
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::AND => {
                    // rd = rs & rt
                    let rs_val = self.regs[instr.rs];
                    let rt_val = self.regs[instr.rt];

                    next_regs[instr.rd] = rs_val & rt_val;
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::OR => {
                    // rd = rs | rt
                    let rs_val = self.regs[instr.rs];
                    let rt_val = self.regs[instr.rt];

                    next_regs[instr.rd] = rs_val | rt_val;
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::XOR => {
                    // rd = rs ^ rt
                    let rs_val = self.regs[instr.rs];
                    let rt_val = self.regs[instr.rt];

                    next_regs[instr.rd] = rs_val ^ rt_val;
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::SLT => {
                    // rd = (rs < rt) ? 1 : 0
                    let rs_val = self.regs[instr.rs];
                    let rt_val = self.regs[instr.rt];

                    next_regs[instr.rd] = if (rs_val as i32) < (rt_val as i32) {
                        1
                    } else {
                        0
                    };
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::SLTU => {
                    // rd = (rs < rt) ? 1 : 0 (unsigned)
                    let rs_val = self.regs[instr.rs];
                    let rt_val = self.regs[instr.rt];

                    next_regs[instr.rd] = if rs_val < rt_val { 1 } else { 0 };
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::SHL => {
                    // rd = rs << rt
                    let rs_val = self.regs[instr.rs];
                    let rt_val = self.regs[instr.rt];

                    next_regs[instr.rd] = rs_val.wrapping_shl(rt_val & 0x1F);
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::SHR => {
                    // rd = rs >> rt
                    let rs_val = self.regs[instr.rs];
                    let rt_val = self.regs[instr.rt];

                    next_regs[instr.rd] = rs_val.wrapping_shr(rt_val & 0x1F);
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::SAR => {
                    // rd = rs >> rt (arithmetic)
                    let rs_val = self.regs[instr.rs];
                    let rt_val = self.regs[instr.rt];

                    next_regs[instr.rd] = (rs_val as i32).wrapping_shr(rt_val & 0x1F) as u32;
                    next_pc = next_pc.wrapping_add(4);
                }

                // -----------------------------
                // Immediate ALU Operations
                isa::opcode::ADDI => {
                    // rd = rs + imm16 (sign-extended)
                    let rs_val = self.regs[instr.rs];
                    let imm = Self::sign_extend_16(instr.imm16);

                    next_regs[instr.rd] = rs_val.wrapping_add(imm);
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::ANDI => {
                    // rd = rs & imm16 (zero-extended)
                    let rs_val = self.regs[instr.rs];
                    let imm = instr.imm16 as u32;

                    next_regs[instr.rd] = rs_val & imm;
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::ORI => {
                    // rd = rs | imm16 (zero-extended)
                    let rs_val = self.regs[instr.rs];
                    let imm = instr.imm16 as u32;

                    next_regs[instr.rd] = rs_val | imm;
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::XORI => {
                    // rd = rs ^ imm16 (zero-extended)
                    let rs_val = self.regs[instr.rs];
                    let imm = instr.imm16 as u32;

                    next_regs[instr.rd] = rs_val ^ imm;
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::SLTI => {
                    // rd = (rs < imm16) ? 1 : 0 (sign-extended)
                    let rs_val = self.regs[instr.rs];
                    let imm = Self::sign_extend_16(instr.imm16);

                    next_regs[instr.rd] = if (rs_val as i32) < (imm as i32) { 1 } else { 0 };
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::SLTIU => {
                    // rd = (rs < imm16) ? 1 : 0 (zero-extended)
                    let rs_val = self.regs[instr.rs];
                    let imm = instr.imm16 as u32;

                    next_regs[instr.rd] = if rs_val < imm { 1 } else { 0 };
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::LUI => {
                    // rd = imm16 << 16
                    let imm = instr.imm16 as u32;

                    next_regs[instr.rd] = imm.wrapping_shl(16);
                    next_pc = next_pc.wrapping_add(4);
                }

                // -----------------------------
                // Load / Store Operations
                isa::opcode::LW => {
                    // rd = Mem[rs + imm16]
                    let rs_val = self.regs[instr.rs];
                    let imm = Self::sign_extend_16(instr.imm16);
                    let addr = rs_val.wrapping_add(imm);

                    let value = bus.read32(addr)?;

                    next_regs[instr.rd] = value;
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::SW => {
                    // Mem[rs + imm16] = rd
                    let rs_val = self.regs[instr.rs];
                    let imm = Self::sign_extend_16(instr.imm16);
                    let addr = rs_val.wrapping_add(imm);

                    let value = self.regs[instr.rd];
                    bus.write32(addr, value)?;
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::LB => {
                    // rd = sign-extended Mem[rs + imm16]
                    let rs_val = self.regs[instr.rs];
                    let imm = Self::sign_extend_16(instr.imm16);

                    let addr = rs_val.wrapping_add(imm);
                    let byte = bus.read8(addr)?;
                    next_regs[instr.rd] = (byte as i8) as i32 as u32; // sign-extend
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::SB => {
                    // Mem[rs + imm16] = least-significant byte of rd
                    let rs_val = self.regs[instr.rs];
                    let imm = Self::sign_extend_16(instr.imm16);
                    let addr = rs_val.wrapping_add(imm);

                    let rd_val = self.regs[instr.rd];
                    let byte = (rd_val & 0xFF) as u8;
                    bus.write8(addr, byte)?;
                    next_pc = next_pc.wrapping_add(4);
                }

                // -----------------------------
                // Branch Operations
                isa::opcode::BEQ => {
                    // if (rd == rs) pc += imm16 << 2
                    let rd_val = self.regs[instr.rd];
                    let rs_val = self.regs[instr.rs];

                    if rd_val == rs_val {
                        let imm = Self::sign_extend_16(instr.imm16);
                        next_pc = next_pc.wrapping_add(4).wrapping_add(imm.wrapping_shl(2));
                    } else {
                        next_pc = next_pc.wrapping_add(4);
                    }
                }
                isa::opcode::BNE => {
                    // if (rd != rs) pc += imm16 << 2
                    let rd_val = self.regs[instr.rd];
                    let rs_val = self.regs[instr.rs];

                    if rd_val != rs_val {
                        let imm = Self::sign_extend_16(instr.imm16);
                        next_pc = next_pc.wrapping_add(4).wrapping_add(imm.wrapping_shl(2));
                    } else {
                        next_pc = next_pc.wrapping_add(4);
                    }
                }
                isa::opcode::BLT => {
                    // if (rd < rs) pc += imm16 << 2
                    let rd_val = self.regs[instr.rd];
                    let rs_val = self.regs[instr.rs];

                    if (rd_val as i32) < (rs_val as i32) {
                        let imm = Self::sign_extend_16(instr.imm16);
                        next_pc = next_pc.wrapping_add(4).wrapping_add(imm.wrapping_shl(2));
                    } else {
                        next_pc = next_pc.wrapping_add(4);
                    }
                }
                isa::opcode::BGE => {
                    // if (rd >= rs) pc += imm16 << 2
                    let rd_val = self.regs[instr.rd];
                    let rs_val = self.regs[instr.rs];

                    if (rd_val as i32) >= (rs_val as i32) {
                        let imm = Self::sign_extend_16(instr.imm16);
                        next_pc = next_pc.wrapping_add(4).wrapping_add(imm.wrapping_shl(2));
                    } else {
                        next_pc = next_pc.wrapping_add(4);
                    }
                }

                // -----------------------------
                // Jumps and Calls
                isa::opcode::J => {
                    // pc = (pc & 0xF0000000) | (target << 2)
                    let target_addr = (next_pc & 0xF000_0000) | (instr.target.wrapping_shl(2));
                    next_pc = target_addr;
                }
                isa::opcode::JAL => {
                    // pc = (pc & 0xF0000000) | (target << 2)
                    // R31 = pc + 4
                    let target_addr = (next_pc & 0xF000_0000) | (instr.target.wrapping_shl(2));
                    next_regs[LINK_REGISTER] = next_pc.wrapping_add(4); // Link
                    next_pc = target_addr;
                }
                isa::opcode::JR => {
                    // pc = rs
                    let rs_val = self.regs[instr.rs];
                    next_pc = rs_val;
                }
                isa::opcode::JALR => {
                    // pc = rs
                    // rd = pc + 4
                    let rs_val = self.regs[instr.rs];
                    next_regs[instr.rd] = next_pc.wrapping_add(4); // Link
                    next_pc = rs_val;
                }

                // -----------------------------
                // System / Misc Operations
                isa::opcode::NOP => {
                    next_pc = next_pc.wrapping_add(4);
                }
                isa::opcode::HALT => {
                    next_halted = true;
                }
                _ => {
                    next_epc = self.pc;
                    next_cause = isa::cause::ILLEGAL_OP;
                    next_pc = EXCEPTION_VECTOR;
                }
            }
        }

        // Ensure R0 is always zero
        next_regs[0] = 0; // R0 is always zero

        // Commit registers
        self.regs = next_regs;
        self.pc = next_pc;
        self.sr = next_sr;
        self.epc = next_epc;
        self.cause = next_cause;
        self.halted = next_halted;

        Ok(())
    }
}
