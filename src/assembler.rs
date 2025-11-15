use std::collections::HashMap;
use crate::cpu::isa::opcode;

// -----------------------------
// Errors
// -----------------------------
#[derive(Debug)]
pub enum AsmError {
    LexError(String),
    ParseError(String),
    UnknownLabel(String),
    InvalidRegister(String),
    InvalidImmediate(String),
}

// -----------------------------
// NV32 segments
// -----------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    CodeData,
    Bss,
}

#[derive(Debug, Clone)]
pub struct NvSegment {
    pub kind: SegmentKind,
    pub base_addr: u32,
    pub length_words: u32,
    pub words: Vec<u32>, // empty for BSS
}

// -----------------------------
// Instruction IR
// -----------------------------
#[derive(Debug, Clone)]
enum Instruction {
    // Loads / stores: OP rd, imm(rs)
    Sb  { rd: u8, base: u8, imm: Imm },
    Sw  { rd: u8, base: u8, imm: Imm },
    Lw  { rd: u8, base: u8, imm: Imm },
    Lb  { rd: u8, base: u8, imm: Imm },

    // ALU register ops (2-operand: rd = rd OP rs)
    Add  { rd: u8, rs: u8 },
    Sub  { rd: u8, rs: u8 },
    And  { rd: u8, rs: u8 },
    Or   { rd: u8, rs: u8 },
    Xor  { rd: u8, rs: u8 },
    Slt  { rd: u8, rs: u8 },
    Sltu { rd: u8, rs: u8 },
    Shl  { rd: u8, rs: u8 },
    Shr  { rd: u8, rs: u8 },
    Sar  { rd: u8, rs: u8 },

    // ALU immediates: OP rd, rs, imm
    Addi  { rd: u8, rs: u8, imm: Imm },
    Andi  { rd: u8, rs: u8, imm: Imm },
    Ori   { rd: u8, rs: u8, imm: Imm },
    Xori  { rd: u8, rs: u8, imm: Imm },
    Slti  { rd: u8, rs: u8, imm: Imm },
    Sltiu { rd: u8, rs: u8, imm: Imm },

    // LUI rd, imm  (rs will be r0 in encoding)
    Lui { rd: u8, imm: Imm },

    // Branches: OP rs, rt, label
    Beq { rs: u8, rt: u8, label: String },
    Bne { rs: u8, rt: u8, label: String },
    Blt { rs: u8, rt: u8, label: String },
    Bge { rs: u8, rt: u8, label: String },

    // Jumps/calls
    J    { label: String },
    Jal  { label: String },      // link in r31 (hardware)
    Jr   { rs: u8 },             // JR rs
    Jalr { rd: u8, rs: u8 },     // JALR rd, rs

    // System / misc
    Nop,
    Halt,
}

// One line after first pass
#[derive(Debug)]
struct Line {
    addr: u32,                  // PC in bytes (absolute)
    instr: Instruction,
}

#[derive(Debug, Clone)]
enum Imm {
    Value(i16),
    Label(String),
}

// -----------------------------
// Public entry points
// -----------------------------

pub fn assemble_nv32(source: &str) -> Result<Vec<NvSegment>, AsmError> {
    // 1) First pass: labels + IR + raw data + BSS
    let mut labels = HashMap::<String, u32>::new();
    let mut lines = Vec::<Line>::new();
    let mut data_words = Vec::<(u32, u32)>::new(); // (addr, word)
    let mut bss_segments = Vec::<(u32, u32)>::new(); // (base_addr, length_words)
    let mut pc: u32 = 0;

    for raw_line in source.lines() {
        let line = strip_comment(raw_line);

        if line.trim().is_empty() {
            continue;
        }

        let (label_opt, rest) = split_label(&line)?;

        if let Some(label) = label_opt {
            if labels.insert(label.clone(), pc).is_some() {
                return Err(AsmError::ParseError(format!("Duplicate label: {}", label)));
            }
        }

        let rest_trim = rest.trim();
        if rest_trim.is_empty() {
            // Label-only line
            continue;
        }

        if rest_trim.starts_with('.') {
            // Directive
            if rest_trim.starts_with(".string") {
                let bytes = parse_string_directive(rest_trim)?;
                // pack bytes into u32 words (little-endian)
                let mut idx = 0;
                while idx < bytes.len() {
                    let b0 = bytes.get(idx).copied().unwrap_or(0);
                    let b1 = bytes.get(idx + 1).copied().unwrap_or(0);
                    let b2 = bytes.get(idx + 2).copied().unwrap_or(0);
                    let b3 = bytes.get(idx + 3).copied().unwrap_or(0);

                    let word = (b0 as u32)
                        | ((b1 as u32) << 8)
                        | ((b2 as u32) << 16)
                        | ((b3 as u32) << 24);

                    data_words.push((pc, word));
                    pc = pc.wrapping_add(4);
                    idx += 4;
                }
            } else if rest_trim.starts_with(".org") {
                // .org <addr>
                // Move PC to absolute address
                let parts: Vec<&str> = rest_trim.split_whitespace().collect();
                if parts.len() != 2 {
                    return Err(AsmError::ParseError(format!(
                        "Invalid .org directive: {}",
                        rest_trim
                    )));
                }
                let addr = parse_u32(parts[1])?;
                pc = addr;
            } else if rest_trim.starts_with(".bss") {
                // .bss <words>   ; reserve N 32-bit words, zero-initialized
                let parts: Vec<&str> = rest_trim.split_whitespace().collect();
                if parts.len() != 2 {
                    return Err(AsmError::ParseError(format!(
                        "Invalid .bss directive: {}",
                        rest_trim
                    )));
                }
                let count_words = parse_u32(parts[1])?;
                bss_segments.push((pc, count_words));
                pc = pc.wrapping_add(count_words * 4);
            } else if rest_trim.starts_with(".text") {
                // Single-section assembler: treat as no-op marker
                continue;
            } else {
                return Err(AsmError::ParseError(format!("Unknown directive: {}", rest_trim)));
            }
        } else {
            // Instruction
            let instr = parse_instruction(rest_trim)?;
            lines.push(Line { addr: pc, instr });
            pc = pc.wrapping_add(4);
        }
    }

    // 2) Second pass: encode into sparse memory map
    let mut mem = HashMap::<u32, u32>::new();

    // data
    for (addr, word) in data_words {
        mem.insert(addr, word);
    }

    // code
    for line in lines {
        let word = encode_instruction(line.instr, &labels, line.addr)?;
        mem.insert(line.addr, word);
    }

    // 3) Build code/data segments by grouping contiguous addresses
    let mut segments = Vec::<NvSegment>::new();

    let mut addrs: Vec<u32> = mem.keys().copied().collect();
    addrs.sort_unstable();

    if !addrs.is_empty() {
        let mut cur_base = addrs[0];
        let mut cur_words = Vec::<u32>::new();
        let mut prev_addr = addrs[0].wrapping_sub(4); // so first addr != prev+4

        for addr in addrs {
            if addr != prev_addr.wrapping_add(4) {
                // flush previous segment if any
                if !cur_words.is_empty() {
                    let len = cur_words.len() as u32;
                    segments.push(NvSegment {
                        kind: SegmentKind::CodeData,
                        base_addr: cur_base,
                        length_words: len,
                        words: cur_words,
                    });
                }
                cur_base = addr;
                cur_words = Vec::new();
            }

            let w = mem
                .get(&addr)
                .copied()
                .expect("address disappeared from mem");
            cur_words.push(w);
            prev_addr = addr;
        }

        if !cur_words.is_empty() {
            segments.push(NvSegment {
                kind: SegmentKind::CodeData,
                base_addr: cur_base,
                length_words: cur_words.len() as u32,
                words: cur_words,
            });
        }
    }

    // 4) Add BSS segments
    for (base, len_words) in bss_segments {
        segments.push(NvSegment {
            kind: SegmentKind::Bss,
            base_addr: base,
            length_words: len_words,
            words: Vec::new(),
        });
    }

    // Deterministic ordering
    segments.sort_by_key(|s| s.base_addr);

    Ok(segments)
}

// -----------------------------
// Helpers: comments / labels
// -----------------------------
fn strip_comment(s: &str) -> String {
    if let Some(idx) = s.find(';') {
        s[..idx].to_string()
    } else if let Some(idx) = s.find('#') {
        s[..idx].to_string()
    } else {
        s.to_string()
    }
}

fn split_label(line: &str) -> Result<(Option<String>, String), AsmError> {
    if let Some(idx) = line.find(':') {
        let (left, right) = line.split_at(idx);
        let label = left.trim().to_string();
        if label.is_empty() {
            return Err(AsmError::ParseError(format!("Empty label in line: {}", line)));
        }
        let rest = right[1..].to_string(); // skip ':'
        Ok((Some(label), rest))
    } else {
        Ok((None, line.to_string()))
    }
}

// -----------------------------
// .string "..." directive
// -----------------------------
fn parse_string_directive(line: &str) -> Result<Vec<u8>, AsmError> {
    // Expect: .string "...."
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() < 2 {
        return Err(AsmError::ParseError(format!("Invalid .string: {}", line)));
    }
    let rest = parts[1].trim();
    if !rest.starts_with('"') || !rest.ends_with('"') {
        return Err(AsmError::ParseError(format!("Expected quotes in .string: {}", line)));
    }
    let inner = &rest[1..rest.len() - 1];

    let mut bytes = Vec::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => bytes.push(b'\n'),
                Some('t') => bytes.push(b'\t'),
                Some('r') => bytes.push(b'\r'),
                Some('\\') => bytes.push(b'\\'),
                Some('"') => bytes.push(b'"'),
                Some(other) => {
                    return Err(AsmError::ParseError(format!(
                        "Unknown escape sequence: \\{}",
                        other
                    )))
                }
                None => {
                    return Err(AsmError::ParseError(
                        "Dangling backslash in .string".to_string(),
                    ))
                }
            }
        } else {
            bytes.push(c as u8);
        }
    }

    // C-style zero terminator
    bytes.push(0);
    Ok(bytes)
}

// -----------------------------
// Parsing instructions
// -----------------------------
fn parse_instruction(line: &str) -> Result<Instruction, AsmError> {
    let mut parts = line.split_whitespace();
    let mnemonic = parts
        .next()
        .ok_or_else(|| AsmError::ParseError(format!("Empty instruction: {}", line)))?
        .to_lowercase();

    let rest = parts.collect::<Vec<_>>().join(" ");
    let rest = rest.trim();

    match mnemonic.as_str() {
        // Loads / stores: OP rd, imm(rs)
        "sb" => parse_ls(rest, LsKind::Sb),
        "sw" => parse_ls(rest, LsKind::Sw),
        "lw" => parse_ls(rest, LsKind::Lw),
        "lb" => parse_ls(rest, LsKind::Lb),

        // ALU register ops (2-operand: rd, rs)
        "add"  => parse_reg_reg(rest).map(|(rd, rs)| Instruction::Add  { rd, rs }),
        "sub"  => parse_reg_reg(rest).map(|(rd, rs)| Instruction::Sub  { rd, rs }),
        "and"  => parse_reg_reg(rest).map(|(rd, rs)| Instruction::And  { rd, rs }),
        "or"   => parse_reg_reg(rest).map(|(rd, rs)| Instruction::Or   { rd, rs }),
        "xor"  => parse_reg_reg(rest).map(|(rd, rs)| Instruction::Xor  { rd, rs }),
        "slt"  => parse_reg_reg(rest).map(|(rd, rs)| Instruction::Slt  { rd, rs }),
        "sltu" => parse_reg_reg(rest).map(|(rd, rs)| Instruction::Sltu { rd, rs }),
        "shl"  => parse_reg_reg(rest).map(|(rd, rs)| Instruction::Shl  { rd, rs }),
        "shr"  => parse_reg_reg(rest).map(|(rd, rs)| Instruction::Shr  { rd, rs }),
        "sar"  => parse_reg_reg(rest).map(|(rd, rs)| Instruction::Sar  { rd, rs }),

        // ALU immediates: OP rd, rs, imm
        "addi"  => parse_reg_reg_imm(rest).map(|(rd, rs, imm)| Instruction::Addi  { rd, rs, imm }),
        "andi"  => parse_reg_reg_imm(rest).map(|(rd, rs, imm)| Instruction::Andi  { rd, rs, imm }),
        "ori"   => parse_reg_reg_imm(rest).map(|(rd, rs, imm)| Instruction::Ori   { rd, rs, imm }),
        "xori"  => parse_reg_reg_imm(rest).map(|(rd, rs, imm)| Instruction::Xori  { rd, rs, imm }),
        "slti"  => parse_reg_reg_imm(rest).map(|(rd, rs, imm)| Instruction::Slti  { rd, rs, imm }),
        "sltiu" => parse_reg_reg_imm(rest).map(|(rd, rs, imm)| Instruction::Sltiu { rd, rs, imm }),

        // LUI rd, imm
        "lui" => {
            let args = split_args(rest, 2)?;
            let rd = parse_reg(args[0])?;
            let imm = parse_imm_or_label(args[1]);
            Ok(Instruction::Lui { rd, imm })
        }

        // Branches: OP rs, rt, label
        "beq" => parse_branch(rest).map(|(rs, rt, label)| Instruction::Beq { rs, rt, label }),
        "bne" => parse_branch(rest).map(|(rs, rt, label)| Instruction::Bne { rs, rt, label }),
        "blt" => parse_branch(rest).map(|(rs, rt, label)| Instruction::Blt { rs, rt, label }),
        "bge" => parse_branch(rest).map(|(rs, rt, label)| Instruction::Bge { rs, rt, label }),

        // Jumps / calls
        "j" => {
            let args = split_args(rest, 1)?;
            Ok(Instruction::J { label: args[0].to_string() })
        }
        "jal" => {
            let args = split_args(rest, 1)?;
            Ok(Instruction::Jal { label: args[0].to_string() })
        }
        "jr" => {
            let args = split_args(rest, 1)?;
            let rs = parse_reg(args[0])?;
            Ok(Instruction::Jr { rs })
        }
        "jalr" => {
            // jalr rd, rs
            let args = split_args(rest, 2)?;
            let rd = parse_reg(args[0])?;
            let rs = parse_reg(args[1])?;
            Ok(Instruction::Jalr { rd, rs })
        }

        "nop"  => Ok(Instruction::Nop),
        "halt" => Ok(Instruction::Halt),

        _ => Err(AsmError::ParseError(format!(
            "Unknown mnemonic: {}",
            mnemonic
        ))),
    }
}

enum LsKind {
    Sb,
    Sw,
    Lw,
    Lb,
}

// OP rd, imm(rs)
fn parse_ls(rest: &str, kind: LsKind) -> Result<Instruction, AsmError> {
    // Format: OP rd, imm(rs)  e.g. SB r3, 0(r1)
    let args = split_args(rest, 2)?;
    let rd = parse_reg(args[0])?;

    let addr = args[1].trim();
    let (imm_str, reg_str) = if let Some(open) = addr.find('(') {
        let close = addr
            .find(')')
            .ok_or_else(|| AsmError::ParseError(format!("Missing ')' in addr: {}", addr)))?;
        let imm_part = addr[..open].trim();
        let reg_part = addr[open + 1..close].trim();
        (imm_part, reg_part)
    } else {
        return Err(AsmError::ParseError(format!(
            "Expected imm(base) addressing, got: {}",
            addr
        )));
    };

    let base = parse_reg(reg_str)?;
    let imm = if imm_str.is_empty() {
        Imm::Value(0)
    } else {
        parse_imm_or_label(imm_str)
    };

    Ok(match kind {
        LsKind::Sb => Instruction::Sb { rd, base, imm },
        LsKind::Sw => Instruction::Sw { rd, base, imm },
        LsKind::Lw => Instruction::Lw { rd, base, imm },
        LsKind::Lb => Instruction::Lb { rd, base, imm },
    })
}

// rd, rs
fn parse_reg_reg(rest: &str) -> Result<(u8, u8), AsmError> {
    let args = split_args(rest, 2)?;
    let rd = parse_reg(args[0])?;
    let rs = parse_reg(args[1])?;
    Ok((rd, rs))
}

// rd, rs, imm
fn parse_reg_reg_imm(rest: &str) -> Result<(u8, u8, Imm), AsmError> {
    let args = split_args(rest, 3)?;
    let rd = parse_reg(args[0])?;
    let rs = parse_reg(args[1])?;
    let imm = parse_imm_or_label(args[2]);
    Ok((rd, rs, imm))
}

// rs, rt, label
fn parse_branch(rest: &str) -> Result<(u8, u8, String), AsmError> {
    let args = split_args(rest, 3)?;
    let rs = parse_reg(args[0])?;
    let rt = parse_reg(args[1])?;
    let label = args[2].to_string();
    Ok((rs, rt, label))
}

fn resolve_imm(imm: Imm, labels: &HashMap<String, u32>) -> Result<i16, AsmError> {
    match imm {
        Imm::Value(v) => Ok(v),
        Imm::Label(name) => {
            let addr = get_label_addr(&name, labels)?;
            // For now, require address to fit in signed 16-bit
            if addr > i16::MAX as u32 {
                return Err(AsmError::InvalidImmediate(format!(
                    "Label '{}' address 0x{:08X} does not fit in 16-bit immediate",
                    name, addr
                )));
            }
            Ok(addr as i16)
        }
    }
}

// -----------------------------
// Arg / register / immediate parsing
// -----------------------------
fn split_args(rest: &str, expected: usize) -> Result<Vec<&str>, AsmError> {
    let mut args = Vec::new();
    for part in rest.split(',') {
        let arg = part.trim();
        if !arg.is_empty() {
            args.push(arg);
        }
    }
    if args.len() != expected {
        return Err(AsmError::ParseError(format!(
            "Expected {} args, got {} in '{}'",
            expected,
            args.len(),
            rest
        )));
    }
    Ok(args)
}

fn parse_reg(s: &str) -> Result<u8, AsmError> {
    let s = s.trim();
    let s = s
        .strip_prefix('r')
        .or_else(|| s.strip_prefix('R'))
        .ok_or_else(|| AsmError::InvalidRegister(format!("Register must start with r: '{}'", s)))?;
    let n: u8 = s
        .parse()
        .map_err(|_| AsmError::InvalidRegister(format!("Bad register number: '{}'", s)))?;
    if n >= 32 {
        return Err(AsmError::InvalidRegister(format!(
            "Register out of range: r{}",
            n
        )));
    }
    Ok(n)
}

fn parse_imm_or_label(s: &str) -> Imm {
    if let Ok(v) = parse_imm(s) {
        Imm::Value(v)
    } else {
        // treat it as a label name
        Imm::Label(s.trim().to_string())
    }
}

/// Parse a 16-bit immediate used in the instruction encoding.
/// Hex `0xXXXX` is treated as raw 16-bit pattern, reinterpreted as i16.
fn parse_imm(s: &str) -> Result<i16, AsmError> {
    let s = s.trim();

    // Hex, positive: 0x0000 .. 0xFFFF
    if let Some(hex) = s.strip_prefix("0x") {
        let val = i32::from_str_radix(hex, 16)
            .map_err(|_| AsmError::InvalidImmediate(s.to_string()))?;

        if val < 0 || val > 0xFFFF {
            return Err(AsmError::InvalidImmediate(format!(
                "Hex immediate out of 16-bit range: {}",
                s
            )));
        }

        // Interpret as raw 16-bit pattern, then cast to i16 (two's complement)
        let v16 = val as u16;
        return Ok(v16 as i16);
    }

    // Hex, negative: -0x...
    if let Some(hex) = s.strip_prefix("-0x") {
        let mag = i32::from_str_radix(hex, 16)
            .map_err(|_| AsmError::InvalidImmediate(s.to_string()))?;
        let val = -mag;

        if val < i16::MIN as i32 || val > i16::MAX as i32 {
            return Err(AsmError::InvalidImmediate(format!(
                "Immediate out of 16-bit range: {}",
                s
            )));
        }

        return Ok(val as i16);
    }

    // Decimal (signed)
    let val: i32 = s
        .parse()
        .map_err(|_| AsmError::InvalidImmediate(s.to_string()))?;

    if val < i16::MIN as i32 || val > i16::MAX as i32 {
        return Err(AsmError::InvalidImmediate(format!(
            "Immediate out of 16-bit range: {}",
            s
        )));
    }

    Ok(val as i16)
}

/// Parse a 32-bit unsigned value for .org / .bss addresses and sizes.
fn parse_u32(s: &str) -> Result<u32, AsmError> {
    let s = s.trim();

    if let Some(hex) = s.strip_prefix("0x") {
        u32::from_str_radix(hex, 16)
            .map_err(|_| AsmError::InvalidImmediate(s.to_string()))
    } else {
        s.parse::<u32>()
            .map_err(|_| AsmError::InvalidImmediate(s.to_string()))
    }
}

// -----------------------------
// Encoding
// -----------------------------
fn encode_instruction(
    instr: Instruction,
    labels: &HashMap<String, u32>,
    pc: u32,
) -> Result<u32, AsmError> {
    // Generic I-type encoder: [31:26] op, [25:21] rd, [20:16] rs, [15:0] imm
    fn enc_i(op: u8, rd: u8, rs: u8, imm: i16) -> u32 {
        ((op as u32) << 26)
            | ((rd as u32) << 21)
            | ((rs as u32) << 16)
            | (imm as u16 as u32)
    }

    match instr {
        // Loads / stores
        Instruction::Sb { rd, base, imm } => {
            let v = resolve_imm(imm, labels)?;
            Ok(enc_i(opcode::SB, rd, base, v))
        }
        Instruction::Sw { rd, base, imm } => {
            let v = resolve_imm(imm, labels)?;
            Ok(enc_i(opcode::SW, rd, base, v))
        }
        Instruction::Lw { rd, base, imm } => {
            let v = resolve_imm(imm, labels)?;
            Ok(enc_i(opcode::LW, rd, base, v))
        }
        Instruction::Lb { rd, base, imm } => {
            let v = resolve_imm(imm, labels)?;
            Ok(enc_i(opcode::LB, rd, base, v))
        }

        // ALU register ops: rd = rd OP rs
        Instruction::Add  { rd, rs } => Ok(enc_i(opcode::ADD,  rd, rs, 0)),
        Instruction::Sub  { rd, rs } => Ok(enc_i(opcode::SUB,  rd, rs, 0)),
        Instruction::And  { rd, rs } => Ok(enc_i(opcode::AND,  rd, rs, 0)),
        Instruction::Or   { rd, rs } => Ok(enc_i(opcode::OR,   rd, rs, 0)),
        Instruction::Xor  { rd, rs } => Ok(enc_i(opcode::XOR,  rd, rs, 0)),
        Instruction::Slt  { rd, rs } => Ok(enc_i(opcode::SLT,  rd, rs, 0)),
        Instruction::Sltu { rd, rs } => Ok(enc_i(opcode::SLTU, rd, rs, 0)),
        Instruction::Shl  { rd, rs } => Ok(enc_i(opcode::SHL,  rd, rs, 0)),
        Instruction::Shr  { rd, rs } => Ok(enc_i(opcode::SHR,  rd, rs, 0)),
        Instruction::Sar  { rd, rs } => Ok(enc_i(opcode::SAR,  rd, rs, 0)),

        // ALU immediates
        Instruction::Addi  { rd, rs, imm } => {
            let v = resolve_imm(imm, labels)?;
            Ok(enc_i(opcode::ADDI, rd, rs, v))
        }
        Instruction::Andi  { rd, rs, imm } => {
            let v = resolve_imm(imm, labels)?;
            Ok(enc_i(opcode::ANDI, rd, rs, v))
        }
        Instruction::Ori   { rd, rs, imm } => {
            let v = resolve_imm(imm, labels)?;
            Ok(enc_i(opcode::ORI, rd, rs, v))
        }
        Instruction::Xori  { rd, rs, imm } => {
            let v = resolve_imm(imm, labels)?;
            Ok(enc_i(opcode::XORI, rd, rs, v))
        }
        Instruction::Slti  { rd, rs, imm } => {
            let v = resolve_imm(imm, labels)?;
            Ok(enc_i(opcode::SLTI, rd, rs, v))
        }
        Instruction::Sltiu { rd, rs, imm } => {
            let v = resolve_imm(imm, labels)?;
            Ok(enc_i(opcode::SLTIU, rd, rs, v))
        }

        // LUI rd, imm   (rs = r0)
        Instruction::Lui { rd, imm } => {
            let v = resolve_imm(imm, labels)?;
            Ok(enc_i(opcode::LUI, rd, 0, v))
        }

        // Branches: OP rs, rt, label
        Instruction::Beq { rs, rt, label } => {
            let imm = branch_imm(&label, labels, pc)?;
            Ok(enc_i(opcode::BEQ, rs, rt, imm))
        }
        Instruction::Bne { rs, rt, label } => {
            let imm = branch_imm(&label, labels, pc)?;
            Ok(enc_i(opcode::BNE, rs, rt, imm))
        }
        Instruction::Blt { rs, rt, label } => {
            let imm = branch_imm(&label, labels, pc)?;
            Ok(enc_i(opcode::BLT, rs, rt, imm))
        }
        Instruction::Bge { rs, rt, label } => {
            let imm = branch_imm(&label, labels, pc)?;
            Ok(enc_i(opcode::BGE, rs, rt, imm))
        }

        // Jumps / calls
        Instruction::J { label } => {
            let target = get_label_addr(&label, labels)?;
            let addr = target >> 2;
            Ok(((opcode::J as u32) << 26) | (addr & 0x03FF_FFFF))
        }
        Instruction::Jal { label } => {
            let target = get_label_addr(&label, labels)?;
            let addr = target >> 2;
            Ok(((opcode::JAL as u32) << 26) | (addr & 0x03FF_FFFF))
        }
        Instruction::Jr { rs } => {
            // JR rs : rd is unused, imm = 0
            Ok(enc_i(opcode::JR, 0, rs, 0))
        }
        Instruction::Jalr { rd, rs } => {
            // JALR rd, rs : rd = link, rs = target
            Ok(enc_i(opcode::JALR, rd, rs, 0))
        }

        // System
        Instruction::Nop  => Ok(enc_i(opcode::NOP,  0, 0, 0)),
        Instruction::Halt => Ok(enc_i(opcode::HALT, 0, 0, 0)),
    }
}

fn get_label_addr(label: &str, labels: &HashMap<String, u32>) -> Result<u32, AsmError> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| AsmError::UnknownLabel(label.to_string()))
}

fn branch_imm(label: &str, labels: &HashMap<String, u32>, pc: u32) -> Result<i16, AsmError> {
    let target = get_label_addr(label, labels)?;
    let pc_next = pc.wrapping_add(4);
    let diff = (target as i32 - pc_next as i32) / 4;
    if diff < i16::MIN as i32 || diff > i16::MAX as i32 {
        return Err(AsmError::InvalidImmediate(format!(
            "Branch target out of range: {}",
            label
        )));
    }
    Ok(diff as i16)
}
