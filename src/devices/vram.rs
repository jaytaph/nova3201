use crate::bus::BusError;

pub struct Vram {
    pub data: Vec<u8>,
}
impl Vram {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
        }
    }

    #[inline]
    fn check_range(&self, offset: u32, size: usize) -> Result<usize, BusError> {
        let off = offset as usize;
        if off + size > self.data.len() {
            return Err(BusError::OutOfBounds(offset));
        }
        Ok(off)
    }

    pub fn write8(&mut self, offset: u32, value: u8) -> Result<(), BusError> {
        let off = self.check_range(offset, 1)?;
        self.data[off] = value;
        Ok(())
    }

    pub fn read8(&self, offset: u32) -> Result<u8, BusError> {
        let off = self.check_range(offset, 1)?;
        Ok(self.data[off])
    }

    pub fn write32(&mut self, offset: u32, value: u32) -> Result<(), BusError> {
        let off = self.check_range(offset, 4)?;
        let bytes = value.to_le_bytes();
        self.data[off..off + 4].copy_from_slice(&bytes);
        Ok(())
    }

    pub fn read32(&self, offset: u32) -> Result<u32, BusError> {
        let off = self.check_range(offset, 4)?;
        let bytes = <[u8; 4]>::try_from(&self.data[off..off + 4]).unwrap();
        Ok(u32::from_le_bytes(bytes))
    }
}
