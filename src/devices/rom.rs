const ROM_SIZE: usize = 64 * 1024;

pub struct Rom {
    pub data: [u8; ROM_SIZE],
}

impl Rom {
    #[allow(unused)]
    fn read8(&self, offset: u32) -> u8 {
        self.data[offset as usize]
    }
}
