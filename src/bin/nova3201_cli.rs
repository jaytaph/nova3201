use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process;
use nova3201::bus::Bus;
use nova3201::Machine;

fn load_nv32<P: AsRef<Path>>(mach: &mut Machine, path: P) -> std::io::Result<()> {
    let mut f = fs::File::open(path)?;
    let mut hdr= [0u8; 12];
    f.read_exact(&mut hdr)?;

    if &hdr[0..4] != b"NV32" {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid NV32 header"));
    }

    let version = u16::from_le_bytes(hdr[4..6].try_into().unwrap());
    let count = u16::from_le_bytes(hdr[6..8].try_into().unwrap());
    if version != 1 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unsupported NV32 version"));
    }

    for _ in 0..count {
        let mut sh = [0u8; 16];
        f.read_exact(&mut sh)?;

        let kind = sh[0];
        let base_addr = u32::from_le_bytes(sh[4..8].try_into().unwrap());
        let size_words = u32::from_le_bytes(sh[8..12].try_into().unwrap());

        match kind {
            // Code (or data)
            0 => {
                let mut buf = vec![0u8; (size_words * 4) as usize];
                f.read_exact(&mut buf)?;

                for (i, chunk) in buf.chunks_exact(4).enumerate() {
                    let w = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    let addr = base_addr + (i as u32) * 4;
                    mach.bus.write32(addr, w).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to write to bus at 0x{:08X}: {}", addr, e))
                    })?;
                }
            }
            1 => {
                // BSS
                for i in 0..size_words {
                    let addr = base_addr + i * 4;
                    mach.bus.write32(addr, 0).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to write to bus at 0x{:08X}: {}", addr, e))
                    })?;
                }
            }
            _ => {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Unknown section kind: {}", kind)));
            }
        }
    }

    Ok(())
}

fn main() {
    let path = env::args().nth(1).expect("Usage: nvrun <program.nvb>");

    let mut mach = Machine::new();
    println!("Loading program '{}'", path);

    if let Err(e) = load_nv32(&mut mach, &path) {
        eprintln!("Error loading binary: {e}");
        process::exit(1);
    }

    // Run for some cycles
    for _ in 0..10_000 {
        mach.step();
        if mach.cpu.halted {
            println!("CPU halted.");
            break;
        }
    }
}
