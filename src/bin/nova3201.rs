use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;
use nova3201::bus::Bus;
use nova3201::{Machine, NovaBus};
use nova3201::BOOT_LOGO;

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
                println!("Loading section at 0x{:08X}, size {} words", base_addr, size_words);
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
                println!("Zero-initializing section at 0x{:08X}, size {} words", base_addr, size_words);
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
    let path = env::args().nth(1).expect("Usage: nova3201 <program.nvb>");

    let mut mach = Machine::new();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    emulate(&mut mach, path);

    println!("Simulation ended. Press Enter to exit.");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
}


pub fn emulate(mach: &mut Machine, path: String) {
    // Start by printing the boot logo
    uart_println(&mut mach.bus, BOOT_LOGO);

    uart_println(&mut mach.bus, "  - Loading ROM: [OK]\n");
    uart_println(&mut mach.bus, "  - Loading user program: ");
    if let Err(e) = load_nv32(mach, &path) {
        uart_println(&mut mach.bus, &format!("  [ERR]: {e}\n"));
        return;
    }
    uart_println(&mut mach.bus, "[OK]\n");

    uart_println(&mut mach.bus, "  - Starting simulation\n\n\n");
    // Run for some cycles
    for _ in 0..10_000 {
        // mach.inspect();
        mach.step();
        if mach.cpu.halted {
            uart_println(&mut mach.bus, "\n\n\nCPU halted");
            return;
        }
    }

    uart_println(&mut mach.bus, "\n\n\n  - Simulation ended after 10,000 cycles.\n");
}

fn uart_println(bus: &mut NovaBus, s: &str) {
    for c in s.chars() {
        if c == '\n' {
            bus.uart.write_tx(b'\r');
        }
        bus.uart.write_tx(c as u8);
    }
}
