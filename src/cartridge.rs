use crate::memory_utils::read_word_from_buffer;
use std::fs::read;

pub trait Cartridge {
    fn read_byte(&mut self, address: u16) -> u8;
    fn read_word(&mut self, address: u16) -> u16;
    fn write_byte(&mut self, address: u16, val: u8);
}
struct CartridgeMapper0 {
    rom: [u8; 0x8000],
    ram: [u8; 0x2000],
    mirrored: bool,
}

impl CartridgeMapper0 {
    pub fn new(data: &[u8]) -> Self {
        let mut ret = Self {
            rom: [0; 0x8000],
            ram: [0; 0x2000],
            mirrored: false,
        };

        if data.len() == 0x4000 {
            ret.mirrored = true;
            ret.rom[0..0x4000].clone_from_slice(data);
        } else if data.len() == 0x8000 {
            ret.mirrored = false;
            ret.rom[0..0x8000].clone_from_slice(data);
        } else {
            panic!("Invalid rom size {:x}", data.len())
        };
        ret
    }
    fn map_address(&mut self, address: u16) -> (&mut [u8], usize) {
        match address {
            0x6000..=0x7FFF => (&mut self.ram, address as usize - 0x6000),
            0x8000..=0xFFFF => {
                let offset = address as usize - 0x8000;
                let offset = if self.mirrored {
                    offset % 0x4000
                } else {
                    offset
                };
                (&mut self.rom, offset)
            }
            _ => panic!("Invalid cartrige address {}", address),
        }
    }
}

impl Cartridge for CartridgeMapper0 {
    fn read_byte(&mut self, address: u16) -> u8 {
        // println!("reading byte {:x}",address);
        let (buf, offset) = self.map_address(address);
        buf[offset]
    }

    fn read_word(&mut self, address: u16) -> u16 {
        // println!("reading word {:x}",address);
        let (buf, offset) = self.map_address(address);
        read_word_from_buffer(buf, offset)
    }

    fn write_byte(&mut self, address: u16, val: u8) {
        // println!("writing byte {:x}",address);
        let (buf, offset) = self.map_address(address);
        buf[offset] = val;
    }
}

mod mmc1 {
    use crate::memory_utils;

    enum Mirroring {
        OneScreen,
        UpperBank,
        Vertical,
        Horizontal,
    }
    enum PRGSwap {
        Swap8000,
        SwapC000,
    }
    enum PRGSize {
        Size32k,
        Size16k(PRGSwap),
    }
    enum CHRSize {
        Size8k,
        Size4k,
    }
    struct ConfigReg {
        pub mirroring: Mirroring,
        pub prg_swapping: PRGSize,
        pub chr_swapping: CHRSize,
    }
    impl ConfigReg {
        pub fn new(reg: u8) -> Self {
            Self {
                mirroring: match reg & 0x03 {
                    0 => Mirroring::OneScreen,
                    1 => Mirroring::UpperBank,
                    2 => Mirroring::Vertical,
                    3 => Mirroring::Horizontal,
                    _ => panic!(),
                },
                prg_swapping: match (reg & 0x08) >> 3 {
                    0 => PRGSize::Size32k,
                    1 => PRGSize::Size16k(match reg & 0x04 >> 2 {
                        0 => PRGSwap::SwapC000,
                        1 => PRGSwap::Swap8000,
                        _ => panic!(),
                    }),
                    _ => panic!(),
                },
                chr_swapping: match (reg & 0x10) >> 4 {
                    0 => CHRSize::Size8k,
                    1 => CHRSize::Size4k,
                    _ => panic!(),
                },
            }
        }
    }

    // PRG Reg
    struct PRGReg {
        pub bank: u8,
        pub wram_enable: bool,
    }

    impl PRGReg {
        pub fn new(reg: u8) -> Self {
            Self {
                bank: reg & 0x0F,
                wram_enable: (reg & 0x10) != 0,
            }
        }
    }

    pub struct CartridgeMapper1 {
        ram: [u8; 0x2000],
        prg_rom: Vec<[u8; 0x4000]>,
        chr_rom: Vec<u8>,
        shift_register: u8,
        control_register: ConfigReg,
        chr_bank0_register: u8,
        chr_bank1_register: u8,
        pgr_bank_register: PRGReg,
    }
    impl CartridgeMapper1 {
        pub fn new(prg: Vec<u8>, chr: Vec<u8>) -> Self {
            let banks = prg.len() / 0x4000;
            let mut prg_rom: Vec<[u8; 0x4000]> = Vec::new();
            for bank in 0..banks {
                prg_rom.push(
                    prg[0x4000 * bank..0x4000 * (bank + 1)]
                        .try_into()
                        .expect("Expected PRG to be divisible by 0x4000"),
                );
            }
            Self {
                ram: [0; 0x2000],
                prg_rom,
                chr_rom: Vec::new(), // todo
                shift_register: 0x10,
                control_register: ConfigReg {
                    mirroring: Mirroring::Vertical,
                    prg_swapping: PRGSize::Size16k(PRGSwap::Swap8000),
                    chr_swapping: CHRSize::Size8k, // TODO: whats the default??
                },
                chr_bank0_register: 0,
                chr_bank1_register: 0,
                pgr_bank_register: PRGReg { bank: 0, wram_enable: true },
            }
        }

        fn map_address(&self, address: u16) -> (&[u8], usize) {
            match address {
                0x6000..=0x7FFF => (&self.ram, address as usize - 0x6000),
                0x8000..=0xFFFF => {
                    match &self.control_register.prg_swapping {
                        PRGSize::Size32k => {
                            let bank = self.pgr_bank_register.bank & 0x0E; // drop last bit
                            let bank = bank as usize;
                            if address < 0xC000 {
                                (&self.prg_rom[bank], address as usize - 0x8000)
                            } else {
                                (&self.prg_rom[bank + 1], address as usize - 0xC000)
                            }
                        }
                        PRGSize::Size16k(swap) => {
                            let bank = self.pgr_bank_register.bank as usize;
                            match swap {
                                PRGSwap::Swap8000 => {
                                    if address < 0xC000 {
                                        (&self.prg_rom[bank], address as usize - 0x8000)
                                    } else {
                                        (self.prg_rom.last().unwrap(), address as usize - 0xC000)
                                    }
                                }
                                PRGSwap::SwapC000 => {
                                    if address < 0xC000 {
                                        (&self.prg_rom[0], address as usize - 0x8000)
                                    } else {
                                        (&self.prg_rom[bank], address as usize - 0xC000)
                                    }
                                }
                            }
                        }
                    }
                }
                _ => panic!("Invalid address to map"),
            }
        }
    }

    impl super::Cartridge for CartridgeMapper1 {
        fn read_byte(&mut self, address: u16) -> u8 {
            // println!("reading byte {:x}",address);
            let (buf, offset) = self.map_address(address);
            buf[offset]
        }

        fn read_word(&mut self, address: u16) -> u16 {
            // println!("reading word {:x}",address);
            let (buf, offset) = self.map_address(address);
            memory_utils::read_word_from_buffer(buf, offset)
        }

        fn write_byte(&mut self, address: u16, val: u8) {
            // println!("writing byte {:x}",address);
            match address {
                0x6000..=0x7FFF => self.ram[address as usize - 0x6000] = val,
                0x8000..=0xFFFF => {
                    if val & 0x80 != 0 {
                        self.shift_register = 0x10;
                    } else {
                        let done = (self.shift_register & 0x01) != 0;
                        self.shift_register >>= 1;
                        self.shift_register |= (val & 0x01) << 4;
                        if done {
                            println!("MMC1 CFG SWRITCH: {:x} -> {:x}", address, self.shift_register);
                            match address {
                                0x8000..=0x9FFF => {
                                    self.control_register = ConfigReg::new(self.shift_register)
                                }
                                0xA000..=0xBFFF => self.chr_bank0_register = self.shift_register, // TODO: use it correctly
                                0xC000..=0xDFFF => self.chr_bank1_register = self.shift_register, // TODO: use it correctly todo!(),
                                0xE000..=0xFFFF => self.pgr_bank_register = PRGReg::new(self.shift_register),// todo!(),
                                _ => panic!(),
                            }

                            self.shift_register = 0x10;
                        }
                    }
                }
                _ => panic!("Invalid cartridge address {:x}", address),
            };
            // todo: consecutive write cycles
        }
    }
}

// todo move this
pub fn load_rom(path: String) -> Result<Box<dyn Cartridge>, String> {
    let Ok(bytes) = read(path) else {
        panic!("failed to read"); // todo
    };

    let header = &bytes[..16];
    if header[0..4] == [b'N', b'E', b'S', 0x1A] {
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

    println!(
        "Flags: 6: {:x}, 7: {:x}, 8: {:x}, 9: {:x}",
        flags6, flags7, flags8, flags9
    );

    let cartridge_type = ((flags6 & 0xF0) >> 4) | (flags7 & 0xF0);

    // TODO: interpret flags, v2
    // TODO: trainer

    let prg_addr = 16;
    let prg_data = &bytes[prg_addr..(prg_addr + prg_rom_size)];
    let chr_addr = prg_addr + prg_rom_size;
    let chr_data = &bytes[chr_addr..(chr_addr + chr_rom_size)];

    // todo: playchoice

    // Write PRG
    match cartridge_type {
        0 => Ok(Box::new(CartridgeMapper0::new(prg_data))),
        1 => Ok(Box::new(mmc1::CartridgeMapper1::new(prg_data.to_vec(), chr_data.to_vec()))),
        _ => panic!("Unimplemented cartridge type {}", cartridge_type),
    }
    // self.write_bytes(0x8000, prg_data);
    // if prg_rom_size == 0x4000 {
    //     self.write_bytes(0xC000, prg_data);
    // }

    // println!("{:x?}", &prg_data);

    // panic!();
}
