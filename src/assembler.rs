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
    LabelHi(String),  // Upper 16 bits of label address
    LabelLo(String),  // Lower 16 bits of label address
}

// -----------------------------
// Public entry points
// -----------------------------

pub fn assemble_nv32(source: &str) -> Result<Vec<NvSegment>, AsmError> {
    // 1) First pass: labels + IR + raw data + BSS
    let mut labels = HashMap::<String, u32>::new();
    let mut equates = HashMap::<String, u32>::new(); // .equ constants (changed to u32)
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
            if rest_trim.starts_with(".equ") {
                // .equ NAME, VALUE
                parse_equ_directive(rest_trim, &mut equates)?;
            } else if rest_trim.starts_with(".string") {
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
            } else if rest_trim.starts_with(".ascii") {
                let bytes = parse_ascii_directive(rest_trim)?;
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
                let addr = parse_u32(parts[1], &equates)?;
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
                let count_words = parse_u32(parts[1], &equates)?;
                bss_segments.push((pc, count_words));
                pc = pc.wrapping_add(count_words * 4);
            } else if rest_trim.starts_with(".text") || rest_trim.starts_with(".data") {
                // Single-section assembler: treat as no-op marker
                continue;
            } else {
                return Err(AsmError::ParseError(format!("Unknown directive: {}", rest_trim)));
            }
        } else {
            // Instruction - may expand into multiple instructions
            let instrs = parse_instruction(rest_trim, &equates)?;
            for instr in instrs {
                lines.push(Line { addr: pc, instr });
                pc = pc.wrapping_add(4);
            }
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
        let word = encode_instruction(line.instr, &labels, &equates, line.addr)?;
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
// .equ directive
// -----------------------------
fn parse_equ_directive(line: &str, equates: &mut HashMap<String, u32>) -> Result<(), AsmError> {
    // Expect: .equ NAME, VALUE
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() < 2 {
        return Err(AsmError::ParseError(format!("Invalid .equ: {}", line)));
    }
    let rest = parts[1].trim();

    // Split by comma
    let pair: Vec<&str> = rest.splitn(2, ',').collect();
    if pair.len() != 2 {
        return Err(AsmError::ParseError(format!("Invalid .equ format (expected NAME, VALUE): {}", line)));
    }

    let name = pair[0].trim().to_string();
    let value_str = pair[1].trim();

    // Parse the value (could be hex or decimal)
    let value = parse_equ_value(value_str, equates)?;

    equates.insert(name, value);
    Ok(())
}

fn parse_equ_value(s: &str, equates: &HashMap<String, u32>) -> Result<u32, AsmError> {
    let s = s.trim();

    // Check if it's a reference to another equate
    if let Some(&val) = equates.get(s) {
        return Ok(val);
    }

    // Try to parse as hex
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u32::from_str_radix(hex, 16)
            .map_err(|_| AsmError::InvalidImmediate(format!("Invalid hex value: {}", s)));
    }

    // Try to parse as decimal
    s.parse::<u32>()
        .map_err(|_| AsmError::InvalidImmediate(format!("Invalid numeric value: {}", s)))
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
                Some('0') => bytes.push(0), // null character
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
// .ascii "..." directive
// -----------------------------
fn parse_ascii_directive(line: &str) -> Result<Vec<u8>, AsmError> {
    // Expect: .ascii "...."  (same as .string but with explicit \0 if needed)
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() < 2 {
        return Err(AsmError::ParseError(format!("Invalid .ascii: {}", line)));
    }
    let rest = parts[1].trim();
    if !rest.starts_with('"') || !rest.ends_with('"') {
        return Err(AsmError::ParseError(format!("Expected quotes in .ascii: {}", line)));
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
                Some('0') => bytes.push(0), // null character
                Some(other) => {
                    return Err(AsmError::ParseError(format!(
                        "Unknown escape sequence: \\{}",
                        other
                    )))
                }
                None => {
                    return Err(AsmError::ParseError(
                        "Dangling backslash in .ascii".to_string(),
                    ))
                }
            }
        } else {
            bytes.push(c as u8);
        }
    }

    // .ascii includes the \0 only if explicitly written in the string
    Ok(bytes)
}

// -----------------------------
// Parsing instructions
// -----------------------------
fn parse_instruction(line: &str, equates: &HashMap<String, u32>) -> Result<Vec<Instruction>, AsmError> {
    let mut parts = line.split_whitespace();
    let mnemonic = parts
        .next()
        .ok_or_else(|| AsmError::ParseError(format!("Empty instruction: {}", line)))?
        .to_lowercase();

    let rest = parts.collect::<Vec<_>>().join(" ");
    let rest = rest.trim();

    match mnemonic.as_str() {
        // Loads / stores: OP rd, imm(rs)
        "sb" => parse_ls(rest, LsKind::Sb, equates).map(|i| vec![i]),
        "sw" => parse_ls(rest, LsKind::Sw, equates).map(|i| vec![i]),
        "lw" => parse_ls(rest, LsKind::Lw, equates).map(|i| vec![i]),
        "lb" => parse_ls(rest, LsKind::Lb, equates).map(|i| vec![i]),

        // ALU register ops (2-operand: rd, rs)
        "add"  => parse_reg_reg(rest).map(|(rd, rs)| vec![Instruction::Add  { rd, rs }]),
        "sub"  => parse_reg_reg(rest).map(|(rd, rs)| vec![Instruction::Sub  { rd, rs }]),
        "and"  => parse_reg_reg(rest).map(|(rd, rs)| vec![Instruction::And  { rd, rs }]),
        "or"   => parse_reg_reg(rest).map(|(rd, rs)| vec![Instruction::Or   { rd, rs }]),
        "xor"  => parse_reg_reg(rest).map(|(rd, rs)| vec![Instruction::Xor  { rd, rs }]),
        "slt"  => parse_reg_reg(rest).map(|(rd, rs)| vec![Instruction::Slt  { rd, rs }]),
        "sltu" => parse_reg_reg(rest).map(|(rd, rs)| vec![Instruction::Sltu { rd, rs }]),
        "shl"  => parse_reg_reg(rest).map(|(rd, rs)| vec![Instruction::Shl  { rd, rs }]),
        "shr"  => parse_reg_reg(rest).map(|(rd, rs)| vec![Instruction::Shr  { rd, rs }]),
        "sar"  => parse_reg_reg(rest).map(|(rd, rs)| vec![Instruction::Sar  { rd, rs }]),

        // ALU immediates: OP rd, rs, imm
        "addi"  => parse_reg_reg_imm(rest, equates).map(|(rd, rs, imm)| vec![Instruction::Addi  { rd, rs, imm }]),
        "andi"  => parse_reg_reg_imm(rest, equates).map(|(rd, rs, imm)| vec![Instruction::Andi  { rd, rs, imm }]),
        "ori"   => parse_reg_reg_imm(rest, equates).map(|(rd, rs, imm)| vec![Instruction::Ori   { rd, rs, imm }]),
        "xori"  => parse_reg_reg_imm(rest, equates).map(|(rd, rs, imm)| vec![Instruction::Xori  { rd, rs, imm }]),
        "slti"  => parse_reg_reg_imm(rest, equates).map(|(rd, rs, imm)| vec![Instruction::Slti  { rd, rs, imm }]),
        "sltiu" => parse_reg_reg_imm(rest, equates).map(|(rd, rs, imm)| vec![Instruction::Sltiu { rd, rs, imm }]),

        // LUI rd, imm
        "lui" => {
            let args = split_args(rest, 2)?;
            let rd = parse_reg(args[0])?;
            let imm = parse_imm_or_label(args[1], equates);
            Ok(vec![Instruction::Lui { rd, imm }])
        }

        // Branches: OP rs, rt, label
        "beq" => parse_branch(rest).map(|(rs, rt, label)| vec![Instruction::Beq { rs, rt, label }]),
        "bne" => parse_branch(rest).map(|(rs, rt, label)| vec![Instruction::Bne { rs, rt, label }]),
        "blt" => parse_branch(rest).map(|(rs, rt, label)| vec![Instruction::Blt { rs, rt, label }]),
        "bge" => parse_branch(rest).map(|(rs, rt, label)| vec![Instruction::Bge { rs, rt, label }]),

        // Jumps / calls
        "j" => {
            let args = split_args(rest, 1)?;
            Ok(vec![Instruction::J { label: args[0].to_string() }])
        }
        "jal" => {
            let args = split_args(rest, 1)?;
            Ok(vec![Instruction::Jal { label: args[0].to_string() }])
        }
        "jr" => {
            let args = split_args(rest, 1)?;
            let rs = parse_reg(args[0])?;
            Ok(vec![Instruction::Jr { rs }])
        }
        "jalr" => {
            // jalr rd, rs
            let args = split_args(rest, 2)?;
            let rd = parse_reg(args[0])?;
            let rs = parse_reg(args[1])?;
            Ok(vec![Instruction::Jalr { rd, rs }])
        }

        // Pseudoinstructions
        "move" | "mv" => {
            // move rd, rs => addi rd, rs, 0
            let args = split_args(rest, 2)?;
            let rd = parse_reg(args[0])?;
            let rs = parse_reg(args[1])?;
            Ok(vec![Instruction::Addi { rd, rs, imm: Imm::Value(0) }])
        }

        "li" => {
            // li rd, imm
            // Smart expansion:
            //   - If imm fits in 16 bits (signed): addi rd, r0, imm
            //   - Otherwise: lui rd, upper16; ori rd, rd, lower16
            let args = split_args(rest, 2)?;
            let rd = parse_reg(args[0])?;
            let imm_or_label = parse_imm_or_label(args[1], equates);

            // Try to resolve the immediate to a concrete value
            match imm_or_label {
                Imm::Value(v) => {
                    // Already a 16-bit value, use addi
                    Ok(vec![Instruction::Addi { rd, rs: 0, imm: Imm::Value(v) }])
                }
                Imm::Label(ref name) => {
                    // Check if it's an equate
                    if let Some(&val) = equates.get(name) {
                        // We have the value, check if it needs expansion
                        if val <= i16::MAX as u32 {
                            Ok(vec![Instruction::Addi { rd, rs: 0, imm: Imm::Value(val as u16 as i16) }])
                        } else {
                            // Need lui + ori expansion for 32-bit values
                            let upper = (val >> 16) as i16;
                            let lower = (val as u16) as i16;
                            Ok(vec![
                                Instruction::Lui { rd, imm: Imm::Value(upper) },
                                Instruction::Ori { rd, rs: rd, imm: Imm::Value(lower) },
                            ])
                        }
                    } else {
                        // Try parsing as a numeric literal
                        if let Ok(val) = parse_u32_literal(name) {
                            // It's a large numeric literal, expand it
                            if val <= i16::MAX as u32 {
                                Ok(vec![Instruction::Addi { rd, rs: 0, imm: Imm::Value(val as i16) }])
                            } else if val <= u16::MAX as u32 {
                                Ok(vec![Instruction::Addi { rd, rs: 0, imm: Imm::Value(val as u16 as i16) }])
                            } else {
                                // Need lui + ori expansion
                                let upper = (val >> 16) as i16;
                                let lower = (val as u16) as i16;
                                Ok(vec![
                                    Instruction::Lui { rd, imm: Imm::Value(upper) },
                                    Instruction::Ori { rd, rs: rd, imm: Imm::Value(lower) },
                                ])
                            }
                        } else {
                            // It's a real label reference (address), use addi for now
                            // This will be resolved later, and fail if the address doesn't fit
                            Ok(vec![Instruction::Addi { rd, rs: 0, imm: imm_or_label }])
                        }
                    }
                }
                Imm::LabelHi(_) | Imm::LabelLo(_) => {
                    // Shouldn't happen from parse_imm_or_label
                    Err(AsmError::ParseError("Unexpected LabelHi/Lo in li".to_string()))
                }
            }
        }

        "la" => {
            // la rd, label/imm - Load Address (always uses lui + ori for full 32-bit)
            let args = split_args(rest, 2)?;
            let rd = parse_reg(args[0])?;
            let imm_or_label = parse_imm_or_label(args[1], equates);

            // Try to resolve to get the actual value
            match imm_or_label {
                Imm::Value(v) => {
                    // Expand to lui + ori even for small values
                    let val = v as i32 as u32;
                    let upper = (val >> 16) as i16;
                    let lower = (val as u16) as i16;
                    Ok(vec![
                        Instruction::Lui { rd, imm: Imm::Value(upper) },
                        Instruction::Ori { rd, rs: rd, imm: Imm::Value(lower) },
                    ])
                }
                Imm::Label(name) => {
                    // Check if it's an equate with a known value
                    if let Some(&val) = equates.get(&name) {
                        let upper = ((val as u32) >> 16) as i16;
                        let lower = (val as u16) as i16;
                        Ok(vec![
                            Instruction::Lui { rd, imm: Imm::Value(upper) },
                            Instruction::Ori { rd, rs: rd, imm: Imm::Value(lower) },
                        ])
                    } else {
                        // Label address unknown, will be resolved later
                        // Use LabelHi/LabelLo markers for proper splitting
                        Ok(vec![
                            Instruction::Lui { rd, imm: Imm::LabelHi(name.clone()) },
                            Instruction::Ori { rd, rs: rd, imm: Imm::LabelLo(name) },
                        ])
                    }
                }
                Imm::LabelHi(_) | Imm::LabelLo(_) => {
                    // Shouldn't happen from parse_imm_or_label, but handle it
                    Err(AsmError::ParseError("Unexpected LabelHi/Lo in la".to_string()))
                }
            }
        }

        "nop"  => Ok(vec![Instruction::Nop]),
        "halt" => Ok(vec![Instruction::Halt]),

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
fn parse_ls(rest: &str, kind: LsKind, equates: &HashMap<String, u32>) -> Result<Instruction, AsmError> {
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
        parse_imm_or_label(imm_str, equates)
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
fn parse_reg_reg_imm(rest: &str, equates: &HashMap<String, u32>) -> Result<(u8, u8, Imm), AsmError> {
    let args = split_args(rest, 3)?;
    let rd = parse_reg(args[0])?;
    let rs = parse_reg(args[1])?;
    let imm = parse_imm_or_label(args[2], equates);
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

fn resolve_imm(imm: Imm, labels: &HashMap<String, u32>, equates: &HashMap<String, u32>) -> Result<i16, AsmError> {
    match imm {
        Imm::Value(v) => Ok(v),
        Imm::Label(name) => {
            // First check if it's an equate
            if let Some(&val) = equates.get(&name) {
                // Check if it fits in i16 range
                if val > i16::MAX as u32 && val < (u16::MAX as u32 - i16::MAX as u32) {
                    return Err(AsmError::InvalidImmediate(format!(
                        "Equate '{}' value 0x{:X} does not fit in 16-bit immediate",
                        name, val
                    )));
                }
                // Reinterpret as i16 (handles both positive and two's complement negative)
                return Ok(val as u16 as i16);
            }

            // Otherwise treat as address label
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
        Imm::LabelHi(name) => {
            // Get upper 16 bits of label address
            // First check equates
            if let Some(&val) = equates.get(&name) {
                let upper = (val >> 16) as i16;
                return Ok(upper);
            }

            // Otherwise it's an address label
            let addr = get_label_addr(&name, labels)?;
            let upper = (addr >> 16) as i16;
            Ok(upper)
        }
        Imm::LabelLo(name) => {
            // Get lower 16 bits of label address
            // First check equates
            if let Some(&val) = equates.get(&name) {
                let lower = (val as u16) as i16;
                return Ok(lower);
            }

            // Otherwise it's an address label
            let addr = get_label_addr(&name, labels)?;
            let lower = (addr as u16) as i16;
            Ok(lower)
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

fn parse_imm_or_label(s: &str, equates: &HashMap<String, u32>) -> Imm {
    // Check for character literal first
    if let Some(ch) = parse_char_literal(s) {
        return Imm::Value(ch as i16);
    }

    // Check if it's a known equate
    let trimmed = s.trim();
    if let Some(&val) = equates.get(trimmed) {
        // For values that fit in signed i16 range, convert directly
        if val <= i16::MAX as u32 {
            return Imm::Value(val as i16);
        }
        // For values in upper u16 range (0x8000-0xFFFF), reinterpret as signed
        // This handles small negative values properly
        if val <= u16::MAX as u32 {
            return Imm::Value(val as u16 as i16);
        }
        // For large 32-bit values, return as Label so li can expand to lui+ori
        return Imm::Label(trimmed.to_string());
    }

    // Try parsing as immediate value
    if let Ok(v) = parse_imm(s) {
        Imm::Value(v)
    } else {
        // treat it as a label name
        Imm::Label(trimmed.to_string())
    }
}

/// Parse character literal like '1' or 'A'
fn parse_char_literal(s: &str) -> Option<u8> {
    let s = s.trim();
    if s.len() >= 3 && s.starts_with('\'') && s.ends_with('\'') {
        let inner = &s[1..s.len()-1];
        if inner.len() == 1 {
            return Some(inner.chars().next().unwrap() as u8);
        }
    }
    None
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
fn parse_u32(s: &str, equates: &HashMap<String, u32>) -> Result<u32, AsmError> {
    let s = s.trim();

    // Check if it's an equate
    if let Some(&val) = equates.get(s) {
        return Ok(val);
    }

    if let Some(hex) = s.strip_prefix("0x") {
        u32::from_str_radix(hex, 16)
            .map_err(|_| AsmError::InvalidImmediate(s.to_string()))
    } else {
        s.parse::<u32>()
            .map_err(|_| AsmError::InvalidImmediate(s.to_string()))
    }
}

/// Parse a numeric literal (hex or decimal) as u32, without checking equates
fn parse_u32_literal(s: &str) -> Result<u32, AsmError> {
    let s = s.trim();

    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
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
    equates: &HashMap<String, u32>,
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
            let v = resolve_imm(imm, labels, equates)?;
            Ok(enc_i(opcode::SB, rd, base, v))
        }
        Instruction::Sw { rd, base, imm } => {
            let v = resolve_imm(imm, labels, equates)?;
            Ok(enc_i(opcode::SW, rd, base, v))
        }
        Instruction::Lw { rd, base, imm } => {
            let v = resolve_imm(imm, labels, equates)?;
            Ok(enc_i(opcode::LW, rd, base, v))
        }
        Instruction::Lb { rd, base, imm } => {
            let v = resolve_imm(imm, labels, equates)?;
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
            let v = resolve_imm(imm, labels, equates)?;
            Ok(enc_i(opcode::ADDI, rd, rs, v))
        }
        Instruction::Andi  { rd, rs, imm } => {
            let v = resolve_imm(imm, labels, equates)?;
            Ok(enc_i(opcode::ANDI, rd, rs, v))
        }
        Instruction::Ori   { rd, rs, imm } => {
            let v = resolve_imm(imm, labels, equates)?;
            Ok(enc_i(opcode::ORI, rd, rs, v))
        }
        Instruction::Xori  { rd, rs, imm } => {
            let v = resolve_imm(imm, labels, equates)?;
            Ok(enc_i(opcode::XORI, rd, rs, v))
        }
        Instruction::Slti  { rd, rs, imm } => {
            let v = resolve_imm(imm, labels, equates)?;
            Ok(enc_i(opcode::SLTI, rd, rs, v))
        }
        Instruction::Sltiu { rd, rs, imm } => {
            let v = resolve_imm(imm, labels, equates)?;
            Ok(enc_i(opcode::SLTIU, rd, rs, v))
        }

        // LUI rd, imm   (rs = r0)
        Instruction::Lui { rd, imm } => {
            let v = resolve_imm(imm, labels, equates)?;
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