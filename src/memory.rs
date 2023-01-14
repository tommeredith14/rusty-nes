use std::fs::read;


const RAM_SIZE: usize = 0x0800;
const PPU_REG_SIZE: usize = 0x0008;
const APU_REG_SIZE: usize = 0x0018;
const APU_TEST_REG_SIZE: usize = 0x0008;
const CARTRIDGE_SPACE_SIZE: usize = 0xBFE0;

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
    cartridge_space: [u8; CARTRIDGE_SPACE_SIZE],
}

impl MemoryMap {
    pub fn new() -> Self {
        Self {
            ram: [0; RAM_SIZE],
            ppu_reg: [0; PPU_REG_SIZE],
            apu_reg: [0; APU_REG_SIZE],
            apu_test_reg: [0; APU_TEST_REG_SIZE],
            cartridge_space: [0; CARTRIDGE_SPACE_SIZE],
        }
    }

    fn map_address(&mut self, address: u16) -> (&mut [u8], u16) {
        match address as usize { // TODO: Return enum so io addrs can be sent to module
            RAM_START_ADDR..=RAM_END_ADDR => (&mut self.ram, address - (RAM_START_ADDR as u16)),
            RAM_MIR1_START_ADDR..=RAM_MIR1_END_ADDR => (&mut self.ram, address - (RAM_MIR1_START_ADDR as u16)),
            RAM_MIR2_START_ADDR..=RAM_MIR2_END_ADDR => (&mut self.ram, address - (RAM_MIR1_START_ADDR as u16)),
            RAM_MIR3_START_ADDR..=RAM_MIR3_END_ADDR => (&mut self.ram, address - (RAM_MIR1_START_ADDR as u16)),
            PPU_REG_START_ADDR..=PPU_MIR_END_ADDR => (&mut self.ppu_reg, address % (PPU_REG_SIZE as u16)),
            APU_REG_START_ADDR..=APU_REG_END_ADDR => (&mut self.apu_reg, address - (APU_REG_START_ADDR as u16)),
            APU_TEST_START_ADDR..=APU_TEST_END_ADDR => (&mut self.apu_test_reg, address - (APU_TEST_START_ADDR as u16)),
            CARTRIDGE_SPACE_START_ADDR..=CARTRIDGE_SPACE_END_ADDR => (&mut self.cartridge_space, address - (CARTRIDGE_SPACE_START_ADDR as u16)),
            _ => panic!("Invalid memory read")
        }
    }

    pub fn read_byte(&mut self, address: u16) -> u8 {
        let (section, subaddress) = self.map_address(address);
        section[subaddress as usize]
    }

    pub fn read_word(&mut self, address: u16) -> u16 {
        let (section, subaddress) = self.map_address(address);
        // NES 6502 CPU is little endian?
        (section[subaddress as usize] as u16) | ((section[subaddress as usize + 1] as u16) << 8)
    }

    pub fn write_byte(&mut self, address: u16, val: u8) {
        let (section, subaddress) = self.map_address(address);
        section[subaddress as usize] = val;
    }

    pub fn write_bytes(&mut self, address: u16, data: &[u8]) {
        let (section, subaddress) = self.map_address(address);
        let subaddress = subaddress as usize;
        section[subaddress..(subaddress + data.len())].clone_from_slice(data);
    }
    
    // todo move this
    pub fn load_rom(&mut self, path: String) {
        let Ok(bytes) = read(path) else {
            panic!("failed to read"); // todo
        };

        let header = &bytes[..16];
        if header[0..4] == [b'N',b'E',b'S', 0x1A] {
            println!("Detected NES cartridge!. Size: {}", bytes.len());
        } else {
            panic!("Invalid cartridge!");
        }

        let prg_rom_size = (header[4] as usize) * 16384;
        let chr_rom_size = (header[5] as usize) * 8192;
        println!("PRG size: {}, chr size: {}", prg_rom_size, chr_rom_size);

        let flags6 = header[6];
        let flags7 = header[7];
        let flags8 = header[8];
        let flags9 = header[9];
        
        println!("Flags: 6: {:x}, 7: {:x}, 8: {:x}, 9: {:x}", flags6, flags7, flags8, flags9);

        // TODO: interpret flags, v2
        // TODO: trainer

        let prg_addr = 16;
        let prg_data = &bytes[prg_addr..(prg_addr+prg_rom_size)];
        let chr_addr = prg_addr + prg_rom_size;
        let _chr_data = &bytes[chr_addr..(chr_addr+chr_rom_size)];
        
        // todo: playchoice

        // Write PRG
        self.write_bytes(0x8000, prg_data);
        if prg_rom_size == 0x4000 {
            self.write_bytes(0xC000, prg_data);
        }

        println!("{:x?}", &prg_data);

        // panic!();

    }
}
