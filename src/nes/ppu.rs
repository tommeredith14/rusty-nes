use std::{cell::RefCell, os::unix::raw::off_t, rc::Rc};

use super::{cartridge::Cartridge, cartridge::Mirroring, memory::MemoryMap};

use eframe::glow::MAX_FRAGMENT_ATOMIC_COUNTERS;
use iced::alignment::Horizontal;
use image::{self, RgbImage};
// use show_image;

static SYSTEM_PALETTE: [(u8,u8,u8); 64] = [
    (0x80, 0x80, 0x80), (0x00, 0x3D, 0xA6), (0x00, 0x12, 0xB0), (0x44, 0x00, 0x96), (0xA1, 0x00, 0x5E), 
    (0xC7, 0x00, 0x28), (0xBA, 0x06, 0x00), (0x8C, 0x17, 0x00), (0x5C, 0x2F, 0x00), (0x10, 0x45, 0x00), 
    (0x05, 0x4A, 0x00), (0x00, 0x47, 0x2E), (0x00, 0x41, 0x66), (0x00, 0x00, 0x00), (0x05, 0x05, 0x05), 
    (0x05, 0x05, 0x05), (0xC7, 0xC7, 0xC7), (0x00, 0x77, 0xFF), (0x21, 0x55, 0xFF), (0x82, 0x37, 0xFA), 
    (0xEB, 0x2F, 0xB5), (0xFF, 0x29, 0x50), (0xFF, 0x22, 0x00), (0xD6, 0x32, 0x00), (0xC4, 0x62, 0x00), 
    (0x35, 0x80, 0x00), (0x05, 0x8F, 0x00), (0x00, 0x8A, 0x55), (0x00, 0x99, 0xCC), (0x21, 0x21, 0x21), 
    (0x09, 0x09, 0x09), (0x09, 0x09, 0x09), (0xFF, 0xFF, 0xFF), (0x0F, 0xD7, 0xFF), (0x69, 0xA2, 0xFF), 
    (0xD4, 0x80, 0xFF), (0xFF, 0x45, 0xF3), (0xFF, 0x61, 0x8B), (0xFF, 0x88, 0x33), (0xFF, 0x9C, 0x12), 
    (0xFA, 0xBC, 0x20), (0x9F, 0xE3, 0x0E), (0x2B, 0xF0, 0x35), (0x0C, 0xF0, 0xA4), (0x05, 0xFB, 0xFF), 
    (0x5E, 0x5E, 0x5E), (0x0D, 0x0D, 0x0D), (0x0D, 0x0D, 0x0D), (0xFF, 0xFF, 0xFF), (0xA6, 0xFC, 0xFF), 
    (0xB3, 0xEC, 0xFF), (0xDA, 0xAB, 0xEB), (0xFF, 0xA8, 0xF9), (0xFF, 0xAB, 0xB3), (0xFF, 0xD2, 0xB0), 
    (0xFF, 0xEF, 0xA6), (0xFF, 0xF7, 0x9C), (0xD7, 0xE8, 0x95), (0xA6, 0xED, 0xAF), (0xA2, 0xF2, 0xDA), 
    (0x99, 0xFF, 0xFC), (0xDD, 0xDD, 0xDD), (0x11, 0x11, 0x11), (0x11, 0x11, 0x11),
    ];
enum NtBase {
    Base2000,
    Base2400,
    Base2800,
    Base2C00
}

#[derive(Clone, Copy)]
enum VramInc {
    Inc1,
    Inc32
}
enum SpriteSize {
    Sprite8x8,
    Sprite8x16
}
struct PpuCtrl {
    //nt_base_x: bool,
    //nt_base_y: bool,
    vram_inc: VramInc,
    sprite_pt_addr: u16,
    bg_pt_addr: u16,
    sprite_size: SpriteSize,
    ext_out: bool,
    nmi: bool
}
impl From<u8> for PpuCtrl {
    fn from(value: u8) -> Self {
        let sprite_size = match (value & 0x20) != 0 {
            false => SpriteSize::Sprite8x8,
            true => SpriteSize::Sprite8x16
        };
              
        Self { 
            //nt_base_x: value & 0x01 != 0,
            //nt_base_y: value & 0x02 != 0,
            vram_inc: match (value & 0x04) != 0 {
                false => VramInc::Inc1,
                true  => VramInc::Inc32
            },
            sprite_pt_addr: match sprite_size {
                SpriteSize::Sprite8x16 => 0x0000,
                SpriteSize::Sprite8x8 => match (value & 0x08) != 0 {
                    false => 0x0000,
                    true  => 0x1000
                }
            },
            bg_pt_addr: match (value & 0x10) != 0 {
                false => 0x0000,
                true  => 0x1000
            },
            sprite_size,
            ext_out: (value & 0x40) != 0,
            nmi:  (value & 0x80) != 0
        }
    }
}
impl Default for PpuCtrl {
    fn default() -> Self {
        Self::from(0)
    }
}

struct PpuMask {
    grayscale: bool,
    show_left_bg: bool,
    show_left_sprite: bool,
    show_bg: bool,
    show_sprites: bool,
    emph_red: bool,
    emph_blue: bool,
    emph_green: bool
}
impl From<u8> for PpuMask {
    fn from(value: u8) -> Self {
        Self { 
            grayscale:        (value & 0x01) != 0,
            show_left_bg:     (value & 0x02) != 0,
            show_left_sprite: (value & 0x04) != 0,
            show_bg:          (value & 0x08) != 0,
            show_sprites:     (value & 0x10) != 0,
            emph_red:         (value & 0x20) != 0,
            emph_blue:        (value & 0x40) != 0,
            emph_green:       (value & 0x80) != 0 
        }
    }
}
impl Default for PpuMask {
    fn default() -> Self {
        Self::from(0)
    }
}

#[derive(Default)]
struct PpuStatus {
    sprite_overflow: bool,
    sprite_0_hit: bool,
    vblank: bool
}
impl From<&PpuStatus> for u8 {
    fn from(value: &PpuStatus) -> Self {
        (if value.sprite_overflow {0x20} else {0}) |
        (if value.sprite_0_hit {0x40} else {0}) |
        (if value.vblank {0x80} else {0})
    }
}


// struct PpuScroll {
//     scroll_x: u8,
//     scroll_y: u8,
//     next_write_x: bool
// }
// impl Default for PpuScroll {
//     fn default() -> Self {
//         Self { scroll_x: 0, scroll_y: 0, next_write_x: true }
//     }
// }
// impl PpuScroll {
//     pub fn unlatch(&mut self) {
//         // self.scroll_x = 0;
//         // self.scroll_y = 0;
//         self.next_write_x = true;
//     }
//     pub fn get_x(&self) -> u8 {
//         self.scroll_x
//     }
//     pub fn get_y(&self) -> u8 {
//         self.scroll_y
//     }
//     pub fn write(&mut self, val: u8) {
//         if self.next_write_x {
//             self.scroll_x = val;
//         } else {
//             self.scroll_y = val;
//         }
//         self.next_write_x = !self.next_write_x;
//     }
// }

// struct PpuAddr {
//     hi: u8,
//     lo: u8,
//     next_write_hi: bool
// }
// impl Default for PpuAddr {
//     fn default() -> Self {
//         Self { hi: 0, lo: 0, next_write_hi: true }
//     }
// }
// impl PpuAddr {
//     pub fn unlatch(&mut self) {
//         // self.lo = 0;
//         // self.lo = 0;
//         self.next_write_hi = true;
//     }
//     pub fn get(&self) -> u16 {
//         let hi = self.hi as u16;
//         let lo = self.lo as u16;
//         (hi << 8) | lo
//     }
//     pub fn write(&mut self, val: u8) {
//         if self.next_write_hi {
//             self.hi = val;
//         } else {
//             self.lo = val;
//         }
//         self.next_write_hi = !self.next_write_hi;
//         // match self.hi {
//         //     None => self.hi = Some(val),
//         //     Some(_) => match self.lo {
//         //         None => self.lo = Some(val),
//         //         Some(_) => {} // TODO: Ignore further writes??
//         //     }
//         // }
//     }
//     pub fn inc(&mut self, inc: VramInc) {
//         let addr = self.get();
//         let addr = match inc {
//             VramInc::Inc1 => addr.wrapping_add(1), // TODO: wrap to 0x4000?
//             VramInc::Inc32 => addr.wrapping_add(32)
//         };
//         self.hi = ((addr & 0xFF00) >> 8) as u8;
//         self.lo = (addr & 0x00FF) as u8;
//     }
// }

#[derive(Default,Clone, Copy)]
struct VRamAddr {
    coarse_x: u8,
    coarse_y: u8,
    nt_x: bool,
    nt_y: bool,
    fine_y: u8
}
impl From<&VRamAddr> for u16 {
    fn from(v: &VRamAddr) -> Self {
        ((v.fine_y as u16) << 12) |
        (if v.nt_y {0x800} else {0}) |
        (if v.nt_x {0x400} else {0}) |
        ((v.coarse_y as u16) << 5) |
        (v.coarse_x as u16)
    }
}
impl From<u16> for VRamAddr {
    fn from(v: u16) -> Self {
        Self {
            coarse_x: (v & 0b1_1111) as u8,
            coarse_y: ((v & 0b11_1110_0000) >> 5) as u8,
            nt_x: (v & 0x400) != 0,
            nt_y: (v & 0x800) != 0,
            fine_y: ((v & 0x3800) >> 12) as u8
        }
    }
}

#[derive(Default)]
struct InternalRegisters {
    v: VRamAddr,
    t: VRamAddr,
    x: u8,
    w: bool
}

impl InternalRegisters {
    fn write_nt(&mut self, d: u8) {
        self.t.nt_x = d & 0x01 != 0;
        self.t.nt_y = d & 0x02 != 0;
    }

    fn unlatch(&mut self) {
        self.w = false
    }

    fn write_scroll(&mut self, d: u8) {
        if !self.w {
            // first write (x)
            self.t.coarse_x = d >> 3;
            self.x = d & 0b111;
            self.w = true;
        } else {
            // second write (y)
            self.t.coarse_y = d >> 3;
            self.t.fine_y = d & 0b111;
            self.w = false;
        }
    }

    fn write_addr(&mut self, d: u8) {
        if !self.w {
            // first write (hi)
            let d = d & 0x3F;
            self.t.coarse_y = (self.t.coarse_y & 0b111) | ((d & 0b11) << 3);
            self.t.nt_x = d & 0b00_0100 != 0;
            self.t.nt_y = d & 0b00_1000 != 0;
            self.t.fine_y = (d & 0b111_0000) >> 4;
            self.w = true;
        } else {
            // second write (lo)
            self.t.coarse_x = d & 0b01_1111;
            self.t.coarse_y = (self.t.coarse_y & 0b1_1000) | ((d & 0b1110_0000) >> 5);
            self.w = false;
            self.v = self.t;
        }
    }

    fn inc_x(&mut self) {
        self.v.coarse_x += 1;
        if self.v.coarse_x >= 32 {
            self.v.coarse_x = 0;
            self.v.nt_x = !self.v.nt_x;
        }
    }
    fn inc_y(&mut self) {
        self.v.fine_y += 1;
        if self.v.fine_y >= 8 {
            self.v.fine_y = 0;
            let mut y = self.v.coarse_y;
            y = match y {
                29 => {
                    self.v.nt_y = !self.v.nt_y;
                    0
                },
                31 => {
                    0
                },
                _ => {
                    y + 1
                },
            };
            self.v.coarse_y = y;
        }
    }

    fn reset_x(&mut self) {
        self.v.coarse_x = self.t.coarse_x;
        self.v.nt_x = self.t.nt_x;
    }
    fn reset_y(&mut self) {
        self.v.coarse_y = self.t.coarse_y;
        self.v.fine_y = self.t.fine_y;
        self.v.nt_y = self.t.nt_y;
    }

    fn inc_addr(&mut self, inc: VramInc) {
        // TODO: races during rendering
        let mut v = u16::from(&self.v);
        v += match inc {
            VramInc::Inc1 => 1,
            VramInc::Inc32 => 32
        };
        self.v = v.into()
    }

    fn get_addr(&self) -> u16 {
        u16::from(&self.v) & 0x3FFF
    }
    fn get_tile_addr(&self) -> u16 {
        0x2000u16 | (u16::from(&self.v) & 0xFFFu16)
    }
    fn get_attr_addr(&self) -> u16 {
        let v = u16::from(&self.v);
        0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07)
    }
}

#[derive(Default)]
struct PpuRegisters {
    ppuctrl: PpuCtrl,
    ppumask: PpuMask,
    ppustatus: PpuStatus,
    oamaddr: u8,
    //ppuscroll: PpuScroll,
    //ppuaddr: PpuAddr,
    internal: InternalRegisters,
    // oamdma: u8
}

enum PpuAddress {
    Chr(u16),
    Nametable(u16), /// TODO: nametable id?
    Pallette(u16)
}
fn map_ppu_addr(addr: u16) -> PpuAddress {
    let addr = addr % 0x4000;
    match addr {
        0x0000..=0x1FFF => PpuAddress::Chr(addr),
        0x2000..=0x3EFF => PpuAddress::Nametable((addr-0x2000) % 0x1000),
        0x3F00..=0x3FFF =>PpuAddress::Pallette((addr-0x3F00) % 0x20),
        _ => panic!("Invalid ppu addr")
    }
}

#[derive(Default,Clone, Copy)]
struct Pipeline {
    nt: u8,
    at_hi: u8,
    at_lo: u8,
    pt_lo: u16,
    pt_hi: u16,
    at_latch_hi: u8,
    at_latch_lo: u8
}
impl Pipeline {
    fn transfer(&mut self, pt_hi: u8, pt_lo: u8, at: u8) {
        self.pt_hi = ((pt_hi as u16)) | (self.pt_hi & 0xFF00);
        self.pt_lo = ((pt_lo as u16)) | (self.pt_lo & 0xFF00);
        self.at_latch_hi = (at & 0x02) >>1;
        self.at_latch_lo = at & 0x01;
        // println!("{:02x}<-{},{:02x}<-{}",self.at_hi, self.at_latch_hi, self.at_lo, self.at_latch_lo);
    }

    fn read(&self, fine_x: u8) -> (u8, u8) {
        let mut pt = 0;
        let mut at = 0;
        let shift = 7-fine_x;
        let mask = 1 << shift;

        pt |= (self.pt_hi & ((mask as u16) << 8));
        at |= (self.at_hi & (mask));

        pt >>= ((shift + 8)- 1);
        if shift == 0 {
            at <<= 1;
        } else {
            at >>= (shift - 1);
        }

        pt |= (self.pt_lo & ((mask as u16)<<8)) >> (shift+8);        
        at |= (self.at_lo & (mask)) >> (shift);
        (pt as u8, at)
    }

    fn shift(&mut self) {
        self.at_hi = (self.at_hi << 1) | self.at_latch_hi;
        self.at_lo = (self.at_lo << 1) | self.at_latch_lo;

        self.pt_hi <<= 1;
        self.pt_lo <<= 1;
    }
}


#[derive(Default)]
struct PpuState {
    frame: u32,
    scanline: usize,
    cycle: usize,
    pipeline: Pipeline,
    num_2oam: usize,
    sprite0_det: bool,
    chr_cache: (u8, u8),
    attr_cache: u8
}

pub struct Ppu {
    reg: PpuRegisters,
    cartridge: Option<Rc<RefCell<Box<dyn Cartridge>>>>,
    nametable1: [u8; 0x400],
    nametable2: [u8; 0x400],
    oam: [u8; 256],
    pallette: [u8; 0x20],
    state: PpuState,
    fb: RgbImage,
    mirroring: Mirroring,
    secondary_oam: [[u8; 4];8],
    read_buf: u8
}

struct SpriteAttributes {
    palette: u8,
    bg_priority: bool,
    flip_horz: bool,
    flip_vert: bool
}
impl From<u8> for SpriteAttributes {
    fn from(val: u8) -> Self {
        Self {
            palette: val & 0x03,
            bg_priority: (val & 0x20) != 0,
            flip_horz: (val & 0x40) != 0,
            flip_vert: (val & 0x80) != 0
        }
    }
}

impl Ppu {
    pub fn new() -> Self {
        let mut ppu = Self {
            cartridge: None,
            reg: PpuRegisters::default(),
            nametable1: [0;0x400],
            nametable2: [0;0x400],
            oam: [0; 256],
            pallette: [0; 0x20],
            state: PpuState::default(),
            fb: RgbImage::new(256, 240),
            mirroring: Mirroring::Horizontal,
            secondary_oam: [[0;4];8],
            read_buf: 0
        };
        // ppu.reg.ppustatus.vblank = true; // TODO: remove
        ppu
    }

    pub fn read_reg(&mut self, addr: u16) -> u8 {
        // TODO: invalid reads?
        let ret = match addr {
            0x00 => panic!(),
            0x01 => panic!(),
            0x02 => {
                // TODO: unlatching, clearing
                let ret : u8 = (&self.reg.ppustatus).into();
                self.reg.ppustatus.vblank = false;
                self.reg.internal.unlatch();
                ret
            },
            0x03 => panic!(),
            0x04 => todo!(), // OAMDATA,
            0x05 => panic!(),
            0x06 => panic!(),
            0x07 => {
                let ret = self.read_buf;
                let addr = self.reg.internal.get_addr();
                self.read_buf = self.read_ppu_byte(addr);
                // println!("read {:2x} from {:4x} (buf {:x})",self.read_buf,addr,ret);
                self.reg.internal.inc_addr(self.reg.ppuctrl.vram_inc);
                // println!("now {:4x}",self.reg.internal.get_addr());
                ret
            }, // PPUDATA

            _ => panic!("Invalid ppu read from {:x}",addr)
        };
        //println!("R PPU REG 0x20{:2x} => {:2x}", addr, ret);
        ret
    }

    pub fn write_reg(&mut self, addr: u16, val: u8) {
        // println!("W PPU REG 0x20{:2x} => {:2x}", addr, val);
        match addr {
            0x00 => {
                self.reg.internal.write_nt(val);
                self.reg.ppuctrl = val.into()
            },
            0x01 => self.reg.ppumask = val.into(),
            0x02 => {} // Can't write to status
            0x03 => self.reg.oamaddr = val,
            0x04 => { // OAMDATA,
                self.oam[self.reg.oamaddr as usize] = val;
                self.reg.oamaddr = self.reg.oamaddr.wrapping_add(1)
            },
            0x05 => self.reg.internal.write_scroll(val),
            0x06 => self.reg.internal.write_addr(val),
            0x07 => { // PPUDATA
                let addr = self.reg.internal.get_addr();
                // println!("W {:2x} to ppu {:4x}",val, addr);
                self.write_ppu_byte(addr, val);
                self.reg.internal.inc_addr(self.reg.ppuctrl.vram_inc);
                // println!("now {:4x}",self.reg.internal.get_addr());
            },

            _ => panic!("Invalid ppu write to {:x}",addr)
        }

    }


    pub fn write_ppu_byte(&mut self, addr: u16, val: u8) {
        let parsed_addr = map_ppu_addr(addr);
        match parsed_addr {
            PpuAddress::Chr(offset) => {//panic!("PPU writing to chr"),
                self.cartridge.as_ref().unwrap().borrow_mut().write_byte_chr(offset, val);
            },
            PpuAddress::Nametable(offset) => {
                let nt = match self.mirroring {
                    Mirroring::Horizontal => if offset < 0x800 {
                        &mut self.nametable1
                    } else {
                        &mut self.nametable2
                    },
                    Mirroring::Vertical => if offset % 0x800 <0x400 {
                        &mut self.nametable1
                    } else {
                        &mut self.nametable2
                    }
                    _ => panic!()
                };
                nt[offset as usize & 0x3FF] = val
            },
            PpuAddress::Pallette(offset) => {
                let offset = offset as usize;
                self.pallette[offset] = val;
                if offset == 0x10 {
                    self.pallette[0] = val;
                }
            }
        }
    }

    pub fn read_ppu_byte(&self, addr: u16) -> u8 {
        let parsed_addr = map_ppu_addr(addr);
        match parsed_addr {
            PpuAddress::Chr(offset) => self.cartridge.as_ref().unwrap().borrow().get_chr()[offset as usize],
            PpuAddress::Nametable(offset) => {
                let nt = match self.mirroring {
                    Mirroring::Horizontal => if offset < 0x800 {
                        &self.nametable1
                    } else {
                        &self.nametable2
                    },
                    Mirroring::Vertical => if offset % 0x800 <0x400 {
                        &self.nametable1
                    } else {
                        &self.nametable2
                    }
                    _ => panic!()
                };
                let offset = offset % 0x400;
                nt[offset as usize]
            },
            PpuAddress::Pallette(offset) => {
                let offset = offset as usize;
                if offset == 0x10 {
                    self.pallette[0]
                } else {
                    self.pallette[offset]
                }
            }
        }
    }

    pub fn oam_dma(&mut self, data: [u8; 256]) {
        let base = self.reg.oamaddr as usize;
        for (i, val) in data.iter().enumerate().take(256) {
            self.oam[(base + i) & 0xFF] = *val;
        }
    }

    fn fetch_nametable(&self) -> u8 {//, x: usize, y: usize) -> u8 {
        let addr = self.reg.internal.get_tile_addr();
        self.read_ppu_byte(addr)
    }
    fn read_nametable(&self, x: usize, y: usize) -> u8 {
        let tile_x = x/8;
        let tile_y = y/8;
        
        let nt = match self.mirroring {
            Mirroring::Horizontal => {
                if tile_y < 30 {
                    self.nametable1
                } else {
                    self.nametable2
                }
            }
            Mirroring::Vertical => {
                if tile_x < 32 {
                    self.nametable1
                } else {
                    self.nametable2
                }
            },
            _ => panic!()
        };

        let tile_y = tile_y % 30;
        let tile_x = tile_x % 32;

        let tile_id = tile_y * 0x20 + tile_x;

        nt[tile_id] // TODO: base NT address
    }
    fn fetch_attribute_table(&self) -> u8 {
        let addr = self.reg.internal.get_attr_addr();
        // TODO fine x
        let attr_byte = self.read_ppu_byte(addr);

        let sub_x = (self.reg.internal.v.coarse_x & 0x02) >> 1;
        let sub_y = (self.reg.internal.v.coarse_y & 0x02) >> 1;
        let shift = match sub_y {
            0 => match sub_x {
                0 => 0, // top left
                1 => 2, // top right
                _ => panic!(),
            },
            1 => match sub_x {
                0 => 4, // bottom left
                1 => 6, // bottom right
                _ => panic!(),
            }
            _ => panic!()
        };
        //println!("{:04x}: {:02x} -> {}",addr,attr_byte,(attr_byte & (0x3 << shift)) >> shift);
        (attr_byte & (0x3 << shift)) >> shift

    }
    fn read_attribute_table(&self, x: usize, y: usize) -> u8 {

        // let x = (x + self.reg.ppuscroll.get_x() as usize + if self.reg.ppuctrl.nt_base_x {256} else {0})%512;
        // let y = (y + self.reg.ppuscroll.get_y() as usize + if self.reg.ppuctrl.nt_base_x {240} else {0})%480;
        let block_x = x/16;
        let block_y = y/16;
        
        let nt = match self.mirroring {
            Mirroring::Horizontal => {
                if block_y < 15 {
                    self.nametable1
                } else {
                    self.nametable2
                }
            }
            Mirroring::Vertical => {
                if block_x < 16 {
                    self.nametable1
                } else {
                    self.nametable2
                }
            },
            _ => panic!()
        };

        let block_y = block_y % 15;
        let block_x = block_x % 16;

        let chunk_y = block_y / 2;
        let chunk_x = block_x / 2;

        let attr_byte = nt[960 + chunk_y * 8 + chunk_x];

        let sub_x = block_x - chunk_x*2;
        let sub_y = block_y - chunk_y*2;

        let shift = match sub_y {
            0 => match sub_x {
                0 => 0, // top left
                1 => 2, // top right
                _ => panic!(),
            },
            1 => match sub_x {
                0 => 4, // bottom left
                1 => 6, // bottom right
                _ => panic!(),
            }
            _ => panic!()
        };

        (attr_byte & (0x3 << shift)) >> shift
    }

    fn sprite_evaluation(&mut self) {
        for s in self.secondary_oam.iter_mut() {
            s.fill(0);
        }
        self.state.num_2oam = 0;

        let y = self.state.scanline  + 1; // Assess next row

        // for s in self.oam.chunks(4) {
        //     let sprite_y = s[0] as usize + 1;
        //     // if (sprite_y..sprite_y+8).contains(&y) { // TODO: 8x16 sprites
        //     if sprite_y <= y && y < sprite_y+8 {
        //         self.secondary_oam[self.state.num_2oam].copy_from_slice(s);
        //         self.state.num_2oam += 1;
        //         if self.state.num_2oam >= 8 {
        //             break;
        //         }
        //     }
        //     // TODO: sprite overflow
        // }

        self.state.sprite0_det = false;
        for s_id in 0..64 {
            let addr = s_id * 4;
            let sprite_y = self.oam[addr] as usize + 1;
            // if (sprite_y..sprite_y+8).contains(&y) { // TODO: 8x16 sprites
            let sprite_height = match self.reg.ppuctrl.sprite_size {
                SpriteSize::Sprite8x8 => 8,
                SpriteSize::Sprite8x16 => 16,
            };
            if sprite_y <= y && y < sprite_y+sprite_height {
                self.secondary_oam[self.state.num_2oam].copy_from_slice(&self.oam[addr..addr+4]);
                if s_id == 0 {
                    self.state.sprite0_det = true;
                }
                self.state.num_2oam += 1;
                if self.state.num_2oam >= 8 {
                    break;
                }
            }
        }
        // TODO: proper prefetching
    }

    fn lookup_chr_bg(&self, chr_id: u8, row: u8) -> (u8, u8) {
        let chr_addr = {
            let mut chr_addr = self.reg.ppuctrl.bg_pt_addr as usize; // Which pattern table
            chr_addr += (chr_id as usize) << 4; // Which sprite
            chr_addr += row as usize; // Which row within the tile
            chr_addr
        };

        let cart_binding = self.cartridge.as_ref().unwrap().borrow();
        let chr = cart_binding.get_chr();

        let chr_lo = chr[chr_addr];
        let chr_hi = chr[chr_addr + 0x08];
        (chr_hi, chr_lo)
    }
    fn lookup_chr_sprite(&self, chr_id: u8, row: u8, col: u8) -> u8 {
        let chr_addr = match self.reg.ppuctrl.sprite_size {
            SpriteSize::Sprite8x8 => {
                // TODO: 8x16 sprites
                let mut chr_addr = self.reg.ppuctrl.sprite_pt_addr as usize; // Which pattern table
                chr_addr += (chr_id as usize) << 4; // Which sprite
                chr_addr += row as usize; // Which row within the tile
                chr_addr
            },
            SpriteSize::Sprite8x16 => {
                let mut chr_addr = if chr_id & 0x01 == 0 { 0x0000 } else { 0x1000 };
                chr_addr += ((chr_id & 0xFE) as usize) << 4;
                chr_addr += (row % 8) as usize;
                if row >= 8 {
                    chr_addr += 1 << 4
                }
                chr_addr
            }
        };

        let bit_num = 7 - (col%8);
        // let mem_binding = self.memory.borrow();
        // let chr = self.cartridge.unwrap().borrow().get_chr();
        let cart_binding = self.cartridge.as_ref().unwrap().borrow();
        let chr = cart_binding.get_chr();
        // let chr = mem_binding.get_chr();
        let chr_lo = (chr[chr_addr] & (1 << bit_num)) >> bit_num;
        let chr_hi = (chr[chr_addr + 0x08]& (1 << bit_num)) >> bit_num;
        chr_hi << 1 | chr_lo
    }

    pub fn run_cycle(&mut self) {
        if self.state.scanline >= 240 && self.state.scanline < 261 {
            return;
        }

        let cycle = self.state.cycle;

        let enable = self.reg.ppumask.show_bg || self.reg.ppumask.show_sprites;
        if enable {
            if cycle == 257 {
                self.reg.internal.inc_y();
            }
            if cycle == 258 {
                self.reg.internal.reset_x();
            }
            if self.state.scanline == 261 && (281..=305).contains(&cycle) {
                self.reg.internal.reset_y();
            }
            if ((1..=257).contains(&cycle) && cycle % 8 == 1) || (cycle == 329) {//|| cycle == 329 {
                let chr_id = self.fetch_nametable();
                let (pt_hi, pt_lo) = self.lookup_chr_bg(chr_id, self.reg.internal.v.fine_y);
                let at = self.fetch_attribute_table();
                // println!("{},{}: got at {:02x}",self.state.scanline, cycle, at);
                self.state.pipeline.transfer(pt_hi, pt_lo, at);
                self.reg.internal.inc_x();
            }
        }

        
        
        
        
        if cycle != 0 {
            let x = cycle - 1;
            let y = self.state.scanline;

            if x < 256 && y <240 {
                // println!("Rendering {},{}",x,y);

                let (bg_val, bg_palette) = self.state.pipeline.read(self.reg.internal.x);
                if x % 8 == 0 {
                    // println!("read:{},{}",bg_val,bg_palette);
                }
                // {
                //     // let bg_x = (
                //     //     x + self.reg.ppuscroll.get_x() as usize + 
                //     //     if self.reg.ppuctrl.nt_base_x {256} else {0})%512;
                //     // let bg_y = (y + 
                //     //     self.reg.ppuscroll.get_y() as usize + 
                //     //     if self.reg.ppuctrl.nt_base_x {240} else {0})%480;
                //     if x == 0 || x % 8 == 0 {
                //         let chr_id = self.read_nametable(x, y);
                //         self.state.chr_cache = self.lookup_chr_bg(chr_id, y as u8 % 8, x as u8 % 8);
                //         self.state.attr_cache = self.read_attribute_table(x, y);
                //         println!("[{},{}]: {}, {}/{}, {}",x,y,chr_id,self.state.chr_cache.0, self.state.chr_cache.1, self.state.attr_cache);
                //     }

                //     // let chr_id = self.read_nametable(x, y); // TODO: scrolling/base address
                //     // let palette_id = self.read_attribute_table(x, y);

                //     // let chr_val = self.lookup_chr_bg(chr_id, y as u8 % 8, x as u8 % 8);
                //     let (chr_hi, chr_lo) = self.state.chr_cache;
                //     let bit_num = 7 - (x % 8);
                //     let chr_lo = (chr_lo & (1 << bit_num)) >> bit_num;
                //     let chr_hi = (chr_hi & (1 << bit_num)) >> bit_num;
                //     let chr_val = chr_hi << 1 | chr_lo;

                //     let palette_id = self.state.attr_cache;
                //     println!("{},  {}",chr_val, palette_id);

                //     (chr_val, palette_id)
                // };

                let sprite_data = {
                    let mut sprite_data = None;
                    if y > 0 {
                        for i in 0..self.state.num_2oam {
                            let s = &self.secondary_oam[i];
                            let sx = s[3] as usize;
                            // if (sx..sx+8).contains(&x) {
                            if sx <= x && x < sx+8 {
                                let sy = s[0] as usize + 1;
                                let chr_id = s[1];
                                let attr = SpriteAttributes::from(s[2]);
    // println!("x: {}, sx: {}", x, sx);
                                let cx = match attr.flip_horz {
                                    false => x - sx,
                                    true => 7 - (x - sx)
                                } as u8;
    // println!("y: {}, sy: {}", y, sy);
                                let cy = match attr.flip_vert {
                                    false => y - sy,
                                    true => (match self.reg.ppuctrl.sprite_size {
                                        SpriteSize::Sprite8x8 => 7,
                                        SpriteSize::Sprite8x16 => 15,
                                    }) - (y - sy)
                                } as u8;
                                let chr_val = self.lookup_chr_sprite(chr_id, cy, cx);
                                if chr_val != 0 {
                                    sprite_data = Some((i,chr_val,attr));
                                    break;
                                }
                            }
                        }
                    }

                    sprite_data
                }.unwrap_or((0,0,SpriteAttributes { palette: 0, bg_priority: true , flip_horz: false, flip_vert: false}));

                let bg_enable = self.reg.ppumask.show_bg && !(x < 8 && !self.reg.ppumask.show_left_bg);
                let sprite_enable = self.reg.ppumask.show_sprites && !(x < 8 && !self.reg.ppumask.show_left_sprite);

                let mut color_id = if !self.reg.ppumask.show_bg && !self.reg.ppumask.show_sprites {
                    if (0x3f00..=0x3fff).contains(&self.reg.internal.get_addr()) {
                        self.pallette[(self.reg.internal.get_addr() as usize - 0x3f00) % 0x20]
                    } else {
                        self.pallette[0] // TODO: correct?
                    }
                } else if sprite_enable {
                    // Consider sprite
                    let (sprite_id, sprite_val, sprite_attr) = sprite_data;
                    if bg_enable && sprite_enable {
                        let sprite0_hit = self.state.sprite0_det && sprite_id == 0 && sprite_val != 0 && bg_val != 0;
                        if sprite0_hit {
                            self.reg.ppustatus.sprite_0_hit = true;
                            println!("Sprite0 hit! {} @ {},{}",sprite_id,x,y);

                        }
                    }

                    if bg_enable {
                        if sprite_val == 0 && bg_val == 0 {
                            self.pallette[0]
                        } else if bg_val == 0 {
                            self.pallette[0x10 + sprite_attr.palette as usize*4 + sprite_val as usize]
                        } else if sprite_val == 0 || sprite_attr.bg_priority {
                            self.pallette[4*bg_palette as usize + bg_val as usize]
                        } else {
                            self.pallette[0x10  + sprite_attr.palette as usize*4 + sprite_val as usize]
                        }
                    } else {
                        // TODO?
                        self.pallette[0x10 + sprite_attr.palette as usize*4 + sprite_val as usize]
                    }
                } else {
                    // No sprite
                    self.pallette[4*bg_palette as usize + bg_val as usize]
                };

                // Apply grayscale
                if self.reg.ppumask.grayscale {
                    color_id &= 0x30;
                }
                // TODO: Apply color emphasis

                // Render
                if color_id > 0x3f {println!("weird color {}",color_id)}
                let color = SYSTEM_PALETTE[(color_id & 0x3f) as usize];
                self.fb.put_pixel(x as u32, y as u32, image::Rgb([color.0, color.1, color.2]));


                // if !self.reg.ppumask.show_bg && (0x3f00..=0x3fff).contains(&self.reg.ppuaddr.get()) {
                //     let color_id = self.pallette[(self.reg.ppuaddr.get() as usize - 0x3f00) % 0x20];
                //     let color = SYSTEM_PALETTE[color_id as usize];
                //     self.fb.put_pixel(x as u32, y as u32, image::Rgb([color.0, color.1, color.2]));
                // } else if self.reg.ppumask.show_bg{
                //     let val = chr_val*64;

                //     let color_id = self.pallette[4*palette_id as usize + chr_val as usize];
                //     let color = SYSTEM_PALETTE[color_id as usize];
                //     self.fb.put_pixel(x as u32, y as u32, image::Rgb([color.0, color.1, color.2]));
                //     // self.fb.put_pixel(x as u32, y as u32, image::Rgb([val,val,val]));
                //     // if chr_id != 0x24 {
                //     //     println!("Color id {}, color {},{},{}",color_id, color.0, color.1, color.2);
                //     // }
                // }
            } else if x == 257 && (0..=240).contains(&y) {
                self.sprite_evaluation();
                self.reg.oamaddr = 0; // TODO: technically when x= 257 until 320
            }

            // TODO: more correct timing with shift registers
            // // Fetch data to latches
            // let mut fetch_tilex = (cycle - 1) / 8;
            // let mut fetch_row = self.state.scanline;
            // if cycle >= 321 && cycle <= 340 {
            //     fetch_row += 1;
            //     fetch_tilex = (cycle - 321) / 8;
            // }
            // let fetch_tiley = if fetch_row < 240 {
            //     fetch_row / 8
            // } else {
            //     0
            // };
            // let fetch_tileno =  fetch_tiley * 0x20 + fetch_tilex;
            // if cycle % 8 == 2 {
            //     self.state.latch.nt = self.nametables[(self.reg.ppuctrl.nt_base as usize) + fetch_tileno - 0x2000];
            // }
            // if cycle % 8 == 4 {
            //     let fetch_blockno = (fetch_tiley / 2) * 0x10 + (fetch_tilex/2);
            //     let attr_byte = fetch_blockno / 4;
            //     let attr_idx = (3-(fetch_blockno % 4)) * 2;
            //     self.state.latch.at = (self.nametables[(self.reg.ppuctrl.nt_base as usize) + attr_byte - 0x2000] & (0x03 << attr_idx) >> attr_idx);
            // }
            // if cycle % 8 == 6 {
            //     let mut chr_addr = self.reg.ppuctrl.bg_pt_addr;
            //     chr_addr += (self.state.latch.nt as u16) << 4;
            //     chr_addr += (fetch_row % 8) as u16;
            //     self.state.latch.pt_lo = self.memory.borrow().get_chr()[chr_addr as usize];
            // }
            // if cycle % 8 == 8 {
            //     let mut chr_addr = self.reg.ppuctrl.bg_pt_addr;
            //     chr_addr += (self.state.latch.nt as u16) << 4;
            //     chr_addr += (fetch_row % 8) as u16;
            //     chr_addr += 0x08;
            //     self.state.latch.pt_hi = self.memory.borrow().get_chr()[chr_addr as usize];
            // }
            // if cycle % 8 == 1 {
            //     // TODO: not totally right?
            //     self.state.shift1 = self.state.shift2;
            //     self.state.shift2 = self.state.latch;
            // }

            // if (1..=256).contains(&cycle) && (0..=240).contains(&self.state.scanline) {
            //     // Render a pixel
            //     // TODO scrolling
            //     let tilex = ((cycle - 1) % 8) as u8;
            //     let tiley = self.state.scanline & 8;
            //     let chr_val = (self.state.shift1.pt_lo & tilex) >> tilex | (self.state.shift1.pt_hi & tilex) >> (tilex) << 1;
            //     let pallette_no = self.state.shift1.at;
            //     let color_id = self.pallette[(pallette_no * 4 + chr_val) as usize];
            //     let color = SYSTEM_PALETTE[color_id as usize];
            //     self.fb.put_pixel(cycle as u32 -1, self.state.scanline as u32, image::Rgb([color.0, color.1, color.2]));
            // }
        }
        if cycle < 336 {
            self.state.pipeline.shift();
        }
    }

    pub fn advance_cycles(&mut self, cycles: u64) -> bool {
        let mut frame_complete = false;
        for _ in 0..cycles {
            self.state.cycle += 1;


            if self.state.scanline == 241 && self.state.cycle == 1 {
                self.reg.ppustatus.vblank = true;
                // TODO: Send interrupt
                frame_complete = true;
                //println!("Vblank! nmi: {}", self.reg.ppuctrl.nmi);
            }
            if self.state.scanline == 261 && self.state.cycle == 1 {
                self.reg.ppustatus.vblank = false;
                self.reg.ppustatus.sprite_0_hit = false;
            }
            if self.state.cycle == 341 || (self.state.scanline == 261 && self.state.cycle == 340 && self.state.frame % 2 == 1) {
                self.state.scanline += 1;
                self.state.cycle = 0;
                if [24,32,128].contains(&self.state.scanline) {
                    println!("Starting Line {}",self.state.scanline);
                }
            }

            if self.state.scanline > 261 {
                self.state.frame += 1;
                self.state.scanline = 0;
                println!("Starting Line 0");
            }
            self.run_cycle()
        }
        frame_complete
    }

    pub fn get_frame(&self) -> image::RgbImage {
        self.fb.clone()
    }


    pub fn render_chr(&self) -> image::GrayImage {
        let binding = self.cartridge.as_ref().unwrap().borrow();
        // let chr_data = binding.get_chr();
        let chr_data = binding.get_chr();
        let mut chr_img = image::GrayImage::new(16*9,32*9);
        for tilenum in 0..512 {
            let bit1 = &chr_data[tilenum*16..tilenum*16+8];
            let bit2 = &chr_data[tilenum*16+8..tilenum*16+16];

            let mut bmp = [0u8; 64];
            let mut img = image::GrayImage::new(8,8);
            for row in 0..8 {
                // println!("ROW {row}: {} + {}",bit1[row], bit2[row]);
                for col in 0..8 {
                    let shift = 7-col;
                    let b1 = (bit1[row] & (1u8 << shift)) >> shift;
                    let b2 = (bit2[row] & (1u8 << shift)) >> shift;
                    // println!("({row},{col}): {b1},{b2} = {}",(b1 + 2*b2) * 64);
                    let val = (b1 + 2*b2) * 64;
                    img.put_pixel(col, row as u32, image::Luma([(b1 + 2*b2) * 64]));
                    chr_img.put_pixel(9*(tilenum%16) as u32 + col as u32, 9*(tilenum/16) as u32 +row as u32, image::Luma([val]))
                }
            }
            // img.save(format!("imgs/img_{}.bmp",tilenum)).unwrap();
            // let image = show_image::ImageView::new(show_image::ImageInfo::mono8(8, 8), &bmp);
            // let window = show_image::create_window("image", Default::default()).unwrap();
            // window.set_image("image-001", image);
        }
        chr_img.save("imgs/chr.bmp").unwrap();
        chr_img
    }
    pub fn render_nt(&self) -> image::RgbImage {
        let binding = self.cartridge.as_ref().unwrap().borrow();
        // let chr_data = binding.get_chr();
        let chr_data = binding.get_chr();
        let size: (usize, usize) = match self.mirroring {
            Mirroring::Horizontal => (256,480),
            Mirroring::Vertical => (512,240),
            _ => panic!()
        };
        let mut nt_img = image::RgbImage::new(size.0 as u32,size.1 as u32);

        // println!("SCROLL: {},{} ,   offset : {},{}",self.reg.ppuscroll.get_x(),self.reg.ppuscroll.get_y(), self.reg.ppuctrl.nt_base_x, self.reg.ppuctrl.nt_base_y);
        // let x_min = (self.reg.ppuscroll.get_x() as usize + if self.reg.ppuctrl.nt_base_x {256} else {0})%size.0;
        // let y_min = (self.reg.ppuscroll.get_y() as usize + if self.reg.ppuctrl.nt_base_y {240} else {0})%size.1;


        for y in 0..size.1 {
            for x in 0..size.0 {

                let (bg_val, bg_palette) = {

                    let chr_id = self.read_nametable(x, y);
                    // let chr_id = self.read_nametable(x, y); // TODO: scrolling/base address
                    // let palette_id = self.read_attribute_table(x, y);

                    // let chr_val = self.lookup_chr_bg(chr_id, y as u8 % 8, x as u8 % 8);
                    let (chr_hi, chr_lo) = self.lookup_chr_bg(chr_id, y as u8 % 8);
                    let bit_num = 7 - (x % 8);
                    let chr_lo = (chr_lo & (1 << bit_num)) >> bit_num;
                    let chr_hi = (chr_hi & (1 << bit_num)) >> bit_num;
                    let chr_val = chr_hi << 1 | chr_lo;

                    let palette_id = self.read_attribute_table(x, y);

                    (chr_val, palette_id)
                };
                let color_id = if bg_val == 0 {
                    self.pallette[0]
                } else {
                    self.pallette[4*bg_palette as usize + bg_val as usize]
                };
                let color = if x % 16 == 0|| y % 16 == 0 {//if x == x_min || y == y_min {
                     (255,0,0)
                // } else if x == x_min + 255 || y == y_min + 239 {
                //     (200,200,0)
                 } else {
                    SYSTEM_PALETTE[(color_id & 0x3f) as usize]
                 }
                ;
                nt_img.put_pixel(x as u32, y as u32, image::Rgb([color.0, color.1, color.2]));
            }
        }

        nt_img.save("imgs/nt.bmp").unwrap();
        nt_img
    }

    pub fn print_nametable(&self) {
        for r in 0..(240/8) {
            for c in 0..(256/8) {
                print!("{:2x},",self.nametable1[r*(256/8) + c]);
            }
            println!();
        }
            println!();
            println!();
        for r in 0..(240/8) {
            for c in 0..(256/8) {
                print!("{:2x},",self.nametable2[r*(256/8) + c]);
            }
            println!();
        }
        println!();
        println!();
        println!("Attr");
        for r in 0..8 {
            for c in 0..8 {
                print!("{:x},",self.nametable1[960 + r*8 + c]);
            }
            println!();
        }
        println!();
        println!();
        println!("Palettes");
        for r in 0..8 {
            for c in 0..4 {
                print!("{:x},",self.pallette[r*4 + c]);
            }
            println!();
        }
        println!();
        println!();
        println!("Sprites");
        for s in 0..64 {
            let s_data = &self.oam[s*4..(s+1)*4];
            println!("{}: ({},{}), t: {:2x}, attr: {:2x}",s, s_data[3], s_data[0], s_data[1], s_data[2]);
        }
    }

    pub fn nmi_requested(&self) -> bool {
        self.reg.ppuctrl.nmi && self.reg.ppustatus.vblank
    }

    pub fn set_cartridge(&mut self, cartridge: Rc<RefCell<Box<dyn Cartridge>>>) {
        self.mirroring = cartridge.borrow().get_nt_mirroring();
        self.cartridge = Some(cartridge);
    }
}