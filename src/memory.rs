use crate::{cartridge::{Cartridge, self}, memory_utils::read_word_from_buffer};

const RAM_SIZE: usize = 0x0800;
const PPU_REG_SIZE: usize = 0x0008;
const APU_REG_SIZE: usize = 0x0018;
const APU_TEST_REG_SIZE: usize = 0x0008;

const RAM_START_ADDR: usize = 0x0000;
const RAM_MIR1_START_ADDR: usize = 0x0800;
const RAM_MIR2_START_ADDR: usize = 0x1000;
const RAM_MIR3_START_ADDR: usize = 0x1800;
const PPU_REG_START_ADDR: usize = 0x2000;
const _PPU_MIR_START_ADDR: usize = 0x2008;
const APU_REG_START_ADDR: usize = 0x4000;
const APU_TEST_START_ADDR: usize = 0x4018;
const CARTRIDGE_SPACE_START_ADDR: usize = 0x4020;

const RAM_END_ADDR: usize = RAM_START_ADDR + RAM_SIZE - 1;
const RAM_MIR1_END_ADDR: usize = RAM_MIR1_START_ADDR + RAM_SIZE - 1;
const RAM_MIR2_END_ADDR: usize = RAM_MIR2_START_ADDR + RAM_SIZE - 1;
const RAM_MIR3_END_ADDR: usize = RAM_MIR3_START_ADDR + RAM_SIZE - 1;
const _PPU_REG_END_ADDR: usize = PPU_REG_START_ADDR + PPU_REG_SIZE - 1;
const PPU_MIR_END_ADDR: usize = APU_REG_START_ADDR - 1;
const APU_REG_END_ADDR: usize = APU_REG_START_ADDR + APU_REG_SIZE - 1;
const APU_TEST_END_ADDR: usize = APU_TEST_START_ADDR + APU_TEST_REG_SIZE - 1;
const CARTRIDGE_SPACE_END_ADDR: usize = 0xFFFF;

pub struct MemoryMap {
    ram: [u8; RAM_SIZE],
    ppu_reg: [u8; PPU_REG_SIZE],
    apu_reg: [u8; APU_REG_SIZE],
    apu_test_reg: [u8; APU_TEST_REG_SIZE],
    cartridge: Option<Box<dyn Cartridge>>
}



enum Address {
    Ram(usize),
    Ppu(usize),
    Apu(usize),
    ApuTest(usize),
    Cartridge
}

impl MemoryMap {
    pub fn new() -> Self {
        Self {
            ram: [0; RAM_SIZE],
            ppu_reg: [0; PPU_REG_SIZE],
            apu_reg: [0; APU_REG_SIZE],
            apu_test_reg: [0; APU_TEST_REG_SIZE],
            cartridge: None
        }
    }

    fn map_address(&mut self, address: u16) -> Address {
        match address as usize { // TODO: Return enum so io addrs can be sent to module
            RAM_START_ADDR..=RAM_END_ADDR => Address::Ram(address as usize - (RAM_START_ADDR)),
            RAM_MIR1_START_ADDR..=RAM_MIR1_END_ADDR => Address::Ram(address as usize - (RAM_MIR1_START_ADDR)),
            RAM_MIR2_START_ADDR..=RAM_MIR2_END_ADDR => Address::Ram(address as usize - (RAM_MIR1_START_ADDR)),
            RAM_MIR3_START_ADDR..=RAM_MIR3_END_ADDR => Address::Ram(address as usize - (RAM_MIR1_START_ADDR)),
            PPU_REG_START_ADDR..=PPU_MIR_END_ADDR => Address::Ppu(address as usize % (PPU_REG_SIZE)),
            APU_REG_START_ADDR..=APU_REG_END_ADDR => Address::Apu(address as usize - (APU_REG_START_ADDR)),
            APU_TEST_START_ADDR..=APU_TEST_END_ADDR => Address::ApuTest(address as usize - (APU_TEST_START_ADDR)),
            CARTRIDGE_SPACE_START_ADDR..=CARTRIDGE_SPACE_END_ADDR => Address::Cartridge,
            _ => panic!("Invalid memory read")
        }
    }

    pub fn read_byte(&mut self, address: u16) -> u8 {
        let parsed_addr: Address = self.map_address(address);
        match parsed_addr {
            Address::Ram(offset) => self.ram[offset],
            Address::Ppu(offset) => self.ppu_reg[offset],
            Address::Apu(offset) => self.apu_reg[offset],
            Address::ApuTest(offset) => self.apu_test_reg[offset],
            Address::Cartridge => self.cartridge.as_mut().unwrap().read_byte(address),
        }
    }

    pub fn read_word(&mut self, address: u16) -> u16 {
        let parsed_addr: Address = self.map_address(address);
        // NES 6502 CPU is little endian?
        match parsed_addr {
            Address::Ram(offset) => read_word_from_buffer(&self.ram, offset),
            Address::Ppu(offset) => read_word_from_buffer(&self.ppu_reg, offset),
            Address::Apu(offset) => read_word_from_buffer(&self.apu_reg, offset),
            Address::ApuTest(offset) => read_word_from_buffer(&self.apu_test_reg, offset),
            Address::Cartridge => self.cartridge.as_mut().unwrap().read_word(address),
        }
    }

    pub fn write_byte(&mut self, address: u16, val: u8) {
        let parsed_addr: Address = self.map_address(address);
        match parsed_addr {
            Address::Ram(offset) => self.ram[offset] = val,
            Address::Ppu(offset) => self.ppu_reg[offset] = val,
            Address::Apu(offset) => self.apu_reg[offset] = val,
            Address::ApuTest(offset) => self.apu_test_reg[offset] = val,
            Address::Cartridge => self.cartridge.as_mut().unwrap().write_byte(address, val),
        }
    }

    // todo move this
    pub fn load_rom(&mut self, path: String) -> Result<(), String> {
        let loaded_cartridge = cartridge::load_rom(path)?;
        self.cartridge = Some(loaded_cartridge);
        Ok(())
    }
}
