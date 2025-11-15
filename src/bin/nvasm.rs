// src/bin/nvasm.rs
//
// Nova Assembler (nvasm)
// Usage:
//   cargo run --bin nvasm -- program.s
//   cargo run --bin nvasm -- program.s out.nvb
//
// Produces a nova32 .nvb binary (NV32 segmented format)

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

use nova3201::assembler::{
    assemble_nv32,
    AsmError,
    NvSegment,
    SegmentKind,
};

fn main() {
    // -------- argument parsing --------
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.is_empty() || args.len() > 2 {
        eprintln!("Nova Assembler (nvasm)");
        eprintln!("Usage: nvasm <input.s> [output.nvb]");
        process::exit(1);
    }

    let input_path = PathBuf::from(&args[0]);
    let output_path = if args.len() == 2 {
        PathBuf::from(&args[1])
    } else {
        // derive output: foo.s -> foo.nvb
        derive_output_path(&input_path, "nvb")
    };

    if let Err(e) = run(&input_path, &output_path) {
        eprintln!("nvasm: error: {e}");
        process::exit(1);
    }
}

fn run(input: &Path, output: &Path) -> Result<(), String> {
    // -------- read source --------
    let src = fs::read_to_string(input)
        .map_err(|e| format!("Failed to read '{}': {e}", input.display()))?;

    // -------- assemble to NV32 segments --------
    let segments = assemble_nv32(&src).map_err(display_asm_error)?;

    // -------- write NV32 file --------
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory '{}': {e}", parent.display()))?;
        }
    }

    let mut f = fs::File::create(output)
        .map_err(|e| format!("Failed to create '{}': {e}", output.display()))?;

    write_nv32(&mut f, &segments)
        .map_err(|e| format!("Failed to write '{}': {e}", output.display()))?;

    let total_words: u32 = segments.iter().map(|s| s.length_words).sum();
    let total_bytes = f.metadata()
        .map(|m| m.len())
        .unwrap_or(0);

    eprintln!(
        "nvasm: assembled '{}' -> '{}' ({} segments, {} words total, {} bytes)",
        input.display(),
        output.display(),
        segments.len(),
        total_words,
        total_bytes,
    );

    Ok(())
}

// -----------------------------
// NV32 writer
// -----------------------------

fn write_nv32<W: Write>(w: &mut W, segments: &[NvSegment]) -> std::io::Result<()> {
    // Header: magic, version, count, reserved
    w.write_all(b"NV32")?;                // magic
    w.write_all(&1u16.to_le_bytes())?;    // version = 1
    w.write_all(&(segments.len() as u16).to_le_bytes())?; // segment count
    w.write_all(&0u32.to_le_bytes())?;    // reserved

    for seg in segments {
        let kind_byte = match seg.kind {
            SegmentKind::CodeData => 0u8,
            SegmentKind::Bss      => 1u8,
        };

        let flags: u8 = 0;
        let reserved: u16 = 0;
        let reserved2: u32 = 0;

        // Segment header (16 bytes):
        // kind: u8
        // flags: u8
        // reserved: u16
        // base_addr: u32
        // length_words: u32
        // reserved2: u32
        w.write_all(&[kind_byte])?;
        w.write_all(&[flags])?;
        w.write_all(&reserved.to_le_bytes())?;
        w.write_all(&seg.base_addr.to_le_bytes())?;
        w.write_all(&seg.length_words.to_le_bytes())?;
        w.write_all(&reserved2.to_le_bytes())?;

        // Payload for code/data segments
        if seg.kind == SegmentKind::CodeData {
            for &word in &seg.words {
                w.write_all(&word.to_le_bytes())?;
            }
        }
        // BSS segments have no payload; loader zeros them
    }

    Ok(())
}

// -----------------------------
// Helpers
// -----------------------------

fn derive_output_path(input: &Path, new_ext: &str) -> PathBuf {
    let mut out = input.to_path_buf();
    out.set_extension(new_ext);
    out
}

fn display_asm_error(err: AsmError) -> String {
    format!("assembly failed: {err:?}")
}
