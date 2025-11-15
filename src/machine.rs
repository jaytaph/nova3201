use crate::NovaBus;
use crate::cpu::Cpu;

pub struct Machine {
    pub cpu: Cpu,
    pub bus: NovaBus,
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
}
