use std::{cell::RefCell, rc::{Weak, Rc}};

use super::{cartridge::{self, Cartridge}, cpu::Cpu, input::InputBus, memory_utils::read_word_from_buffer, ppu::Ppu};

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
    //ppu_reg: [u8; PPU_REG_SIZE],
    apu_reg: [u8; APU_REG_SIZE],
    apu_test_reg: [u8; APU_TEST_REG_SIZE],
    cartridge: Option<Box<dyn Cartridge>>,

    // Todo: reorganize
    ppu: Weak<RefCell<Ppu>>,
    cpu: Weak<RefCell<Cpu>>,
    io: Weak<RefCell<InputBus>>
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
            // ppu_reg: [0; PPU_REG_SIZE],
            apu_reg: [0; APU_REG_SIZE],
            apu_test_reg: [0; APU_TEST_REG_SIZE],
            cartridge: None,

            ppu: Weak::new(),
            cpu: Weak::new(),
            io: Weak::new()
        }
    }

    fn map_address(&self, address: u16) -> Address {
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

    pub fn read_byte(&self, address: u16) -> u8 {
        let parsed_addr: Address = self.map_address(address);
        match parsed_addr {
            Address::Ram(offset) => self.ram[offset],
            Address::Ppu(offset) => self.ppu.upgrade().unwrap().borrow_mut().read_reg(offset as u16), // TODO: why does this compile??
            Address::Apu(offset) => {
                match offset {
                    0x16 => {
                        self.io.upgrade().unwrap().borrow_mut().read_4016()
                    },
                    0x17 => {
                        self.io.upgrade().unwrap().borrow_mut().read_4017()
                    }
                    _ => 0 // Open bus
                }
            }
            Address::ApuTest(offset) => self.apu_test_reg[offset],
            Address::Cartridge => self.cartridge.as_ref().unwrap().read_byte(address),
        }
    }

    pub fn read_word(&self, address: u16) -> u16 {
        let parsed_addr: Address = self.map_address(address);
        // NES 6502 CPU is little endian?
        match parsed_addr {
            Address::Ram(offset) => read_word_from_buffer(&self.ram, offset),
            Address::Ppu(_) => panic!("Reading word from ppu reg"),
            Address::Apu(offset) => {
                match offset {
                    0x16 => {
                        todo!("Read word from input");
                    },
                    0x17 => {
                        todo!("Read word from input+1")
                    }
                    _ => 0 // Open bus
                }
            }
            Address::ApuTest(offset) => read_word_from_buffer(&self.apu_test_reg, offset),
            Address::Cartridge => self.cartridge.as_ref().unwrap().read_word(address),
        }
    }

    pub fn write_byte(&mut self, address: u16, val: u8) -> u32 {
        let parsed_addr: Address = self.map_address(address);
        match parsed_addr {
            Address::Ram(offset) => self.ram[offset] = val,
            Address::Ppu(offset) => self.ppu.upgrade().unwrap().borrow_mut().write_reg(offset as u16, val),
            Address::Apu(offset) => {
                match offset {
                    0x14 => {
                        // PPU OAM DMA
                        let mut data = [0u8; 256];
                        let start_addr = (val as u16) << 8;
                        for i in 0..256 {
                            data[i as usize] = self.read_byte(start_addr + i);
                        }
                        self.ppu.upgrade().unwrap().borrow_mut().oam_dma(data);
                    },
                    0x16 => {
                        self.io.upgrade().unwrap().borrow_mut().write(val);
                    }
                    _ => self.apu_reg[offset] = val
                }
            },
            Address::ApuTest(offset) => self.apu_test_reg[offset] = val,
            Address::Cartridge => self.cartridge.as_mut().unwrap().write_byte(address, val),
        };

        // hidden cycles
        match address {
            0x4014 => 513, // TODO or 514??
            _ => 0
        }
    }

    // todo move this
    pub fn load_rom(&mut self, path: String) -> Result<(), String> {
        let loaded_cartridge = cartridge::load_rom(path)?;
        self.cartridge = Some(loaded_cartridge);
        Ok(())
    }

    // todo: reorganize
    pub fn set_refs(&mut self, ppu: Rc<RefCell<Ppu>>, cpu: Rc<RefCell<Cpu>>, inputs: Rc<RefCell<InputBus>>) {
        self.ppu = Rc::downgrade(&ppu);
        self.cpu = Rc::downgrade(&cpu);
        self.io = Rc::downgrade(&inputs)
    }

    pub fn get_chr(&self) -> &[u8; 0x2000] {
        self.cartridge.as_ref().unwrap().read_chr()
    }

    pub fn nmi_requested(&self) -> bool {
        self.ppu.upgrade().unwrap().borrow().nmi_requested()
    }

}
