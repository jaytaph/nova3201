use crate::NovaBus;
use crate::cpu::Cpu;

pub struct Machine {
    pub cpu: Cpu,
    pub bus: NovaBus,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new()
    }
}

impl Machine {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::new(),
            bus: NovaBus::new(),
        }
    }

    pub fn load_program(&mut self, base: u32, words: &[u32]) {
        for (i, &word) in words.iter().enumerate() {
            self.bus
                .ram
                .write32(base + (i as u32) * 4, word)
                .expect("Failed to load program into memory");
        }
    }
}

/// Structure that holds the current state of IRQ lines
pub struct IrqLines {
    pub timer1: bool,
    pub timer2: bool,
    pub uart: bool,
}

impl Machine {
    pub fn step(&mut self) {
        self.bus.timer1.tick();
        self.bus.timer2.tick();
        self.bus.uart.tick();

        let irq = IrqLines {
            timer1: self.bus.timer1.irq(),
            timer2: self.bus.timer2.irq(),
            uart: self.bus.uart.irq(),
        };

        let _ = self.cpu.step(&mut self.bus, &irq);
    }

    // Copy this function to replace your current inspect() implementation
    // This is the recommended "Boxed, Clean Layout" version

    pub fn inspect(&self) {
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ CPU State                                                       │");
        println!("├─────────────────────────────────────────────────────────────────┤");

        // Program Counter and Status
        println!("│ PC:     0x{:08X}  SR:     0x{:08X}  Halted: {:5}        │",
                 self.cpu.pc(), self.cpu.sr(), self.cpu.halted());
        println!("│ EPC:    0x{:08X}  Cause:  0x{:08X}                      │",
                 self.cpu.epc(), self.cpu.cause());

        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│ Registers                                                       │");
        println!("├─────────────────────────────────────────────────────────────────┤");

        let regs = self.cpu.regs();

        // Print registers in rows of 4
        for row in 0..8 {
            print!("│ ");
            for col in 0..4 {
                let reg_num = row * 4 + col;
                print!("r{:<2}: 0x{:08X} ", reg_num, regs[reg_num]);
            }
            println!("│");
        }

        println!("└─────────────────────────────────────────────────────────────────┘");
        println!();
    }
}
