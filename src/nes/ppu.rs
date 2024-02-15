use std::{cell::RefCell, rc::Rc};

use super::memory::MemoryMap;

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
    nt_base: u16,
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
            nt_base: match value & 0x03 {
                0 => 0x2000,
                1 => 0x2400,
                2 => 0x2800,
                3 => 0x2C00,
                _ => panic!("Invalid nt")
            },
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


struct PpuScroll {
    scroll_x: u8,
    scroll_y: u8,
    next_write_x: bool
}
impl Default for PpuScroll {
    fn default() -> Self {
        Self { scroll_x: 0, scroll_y: 0, next_write_x: true }
    }
}
impl PpuScroll {
    pub fn unlatch(&mut self) {
        self.scroll_x = 0;
        self.scroll_y = 0;
        self.next_write_x = true;
    }
    pub fn get_x(&self) -> u8 {
        self.scroll_x
    }
    pub fn get_y(&self) -> u8 {
        self.scroll_y
    }
    pub fn write(&mut self, val: u8) {
        if self.next_write_x {
            self.scroll_x = val;
        } else {
            self.scroll_y = val;
        }
        self.next_write_x = !self.next_write_x;
    }
}

struct PpuAddr {
    hi: u8,
    lo: u8,
    next_write_hi: bool
}
impl Default for PpuAddr {
    fn default() -> Self {
        Self { hi: 0, lo: 0, next_write_hi: true }
    }
}
impl PpuAddr {
    pub fn unlatch(&mut self) {
        self.lo = 0;
        self.lo = 0;
        self.next_write_hi = true;
    }
    pub fn get(&self) -> u16 {
        let hi = self.hi as u16;
        let lo = self.lo as u16;
        (hi << 8) | lo
    }
    pub fn write(&mut self, val: u8) {
        if self.next_write_hi {
            self.hi = val;
        } else {
            self.lo = val;
        }
        self.next_write_hi = !self.next_write_hi;
        // match self.hi {
        //     None => self.hi = Some(val),
        //     Some(_) => match self.lo {
        //         None => self.lo = Some(val),
        //         Some(_) => {} // TODO: Ignore further writes??
        //     }
        // }
    }
    pub fn inc(&mut self, inc: VramInc) {
        let addr = self.get();
        let addr = match inc {
            VramInc::Inc1 => addr.wrapping_add(1), // TODO: wrap to 0x4000?
            VramInc::Inc32 => addr.wrapping_add(32)
        };
        self.hi = ((addr & 0xFF00) >> 8) as u8;
        self.lo = (addr & 0x00FF) as u8;
    }
}


#[derive(Default)]
struct PpuRegisters {
    ppuctrl: PpuCtrl,
    ppumask: PpuMask,
    ppustatus: PpuStatus,
    oamaddr: u8,
    ppuscroll: PpuScroll,
    ppuaddr: PpuAddr,
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
struct PpuCache {
    nt: u8,
    at: u8,
    pt_lo: u8,
    pt_hi: u8
}

#[derive(Default)]
struct PpuState {
    frame: u32,
    scanline: usize,
    cycle: usize,
    shift1: PpuCache,
    shift2: PpuCache,
    latch: PpuCache,
    num_2oam: usize,
    chr_cache: (u8, u8),
    attr_cache: u8
}

pub struct Ppu {
    reg: PpuRegisters,
    memory: Rc<RefCell<MemoryMap>>,
    nametable1: [u8; 0x400],
    nametable2: [u8; 0x400],
    oam: [u8; 256],
    pallette: [u8; 0x20],
    state: PpuState,
    fb: RgbImage,
    mirroring: Mirroring,
    secondary_oam: [[u8; 4];8]
}

enum Mirroring {
    Vertical,
    Horizontal,
    OneScreen,
    FourScreen
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
    pub fn new(mem: Rc<RefCell<MemoryMap>>) -> Self {
        let mut ppu = Self {
            memory: mem,
            reg: PpuRegisters::default(),
            nametable1: [0;0x400],
            nametable2: [0;0x400],
            oam: [0; 256],
            pallette: [0; 0x20],
            state: PpuState::default(),
            fb: RgbImage::new(256, 240),
            mirroring: Mirroring::Horizontal,
            secondary_oam: [[0;4];8]
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
                self.reg.ppuscroll.unlatch();
                self.reg.ppuaddr.unlatch();
                ret
            },
            0x03 => panic!(),
            0x04 => todo!(), // OAMDATA,
            0x05 => panic!(),
            0x06 => panic!(),
            0x07 => todo!(), // PPUDATA

            _ => panic!("Invalid ppu read from {:x}",addr)
        };
        // println!("R PPU REG 0x20{:x} => {:x}", addr, ret);
        ret
    }

    pub fn write_reg(&mut self, addr: u16, val: u8) {
        // println!("W PPU REG 0x20{:x} => {:x}", addr, val);
        match addr {
            0x00 => self.reg.ppuctrl = val.into(),
            0x01 => self.reg.ppumask = val.into(),
            0x02 => {} // Can't write to status
            0x03 => self.reg.oamaddr = val,
            0x04 => { // OAMDATA,
                self.oam[self.reg.oamaddr as usize] = val;
                self.reg.oamaddr = self.reg.oamaddr.wrapping_add(1)
            },
            0x05 => self.reg.ppuscroll.write(val),
            0x06 => self.reg.ppuaddr.write(val),
            0x07 => { // PPUDATA
                // println!("W {:x} to ppu {:x}",val, self.reg.ppuaddr.get());
                self.write_ppu_byte(self.reg.ppuaddr.get(), val);
                self.reg.ppuaddr.inc(self.reg.ppuctrl.vram_inc);
            },

            _ => panic!("Invalid ppu write to {:x}",addr)
        }

    }


    pub fn write_ppu_byte(&mut self, addr: u16, val: u8) {
        let parsed_addr = map_ppu_addr(addr);
        match parsed_addr {
            PpuAddress::Chr(_) => (),//panic!("PPU writing to chr"),
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

    pub fn read_ppu_byte(&mut self, addr: u16) -> u8 {
        let parsed_addr = map_ppu_addr(addr);
        match parsed_addr {
            PpuAddress::Chr(offset) => self.memory.borrow().get_chr()[offset as usize],
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

    fn read_attribute_table(&self, x: usize, y: usize) -> u8 {
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

        for s_id in 0..64 {
            let addr = s_id * 4;
            let sprite_y = self.oam[addr] as usize + 1;
            // if (sprite_y..sprite_y+8).contains(&y) { // TODO: 8x16 sprites
            if sprite_y <= y && y < sprite_y+8 {
                self.secondary_oam[self.state.num_2oam].copy_from_slice(&self.oam[addr..addr+4]);
                self.state.num_2oam += 1;
                if self.state.num_2oam >= 8 {
                    break;
                }
            }
        }
        // TODO: proper prefetching
    }

    fn lookup_chr_bg(&self, chr_id: u8, row: u8, col: u8) -> (u8, u8) {
        let chr_addr = {
            let mut chr_addr = self.reg.ppuctrl.bg_pt_addr as usize; // Which pattern table
            chr_addr += (chr_id as usize) << 4; // Which sprite
            chr_addr += row as usize; // Which row within the tile
            chr_addr
        };

        let bit_num = 7 - (col%8);
        let mem_binding = self.memory.borrow();
        let chr = mem_binding.get_chr();
        // let chr_lo = (chr[chr_addr] & (1 << bit_num)) >> bit_num;
        // let chr_hi = (chr[chr_addr + 0x08]& (1 << bit_num)) >> bit_num;
        // chr_hi << 1 | chr_lo
        let chr_lo = chr[chr_addr];
        let chr_hi = chr[chr_addr + 0x08];
        (chr_hi, chr_lo)
    }
    fn lookup_chr_sprite(&self, chr_id: u8, row: u8, col: u8) -> u8 {
        let chr_addr = {
            // TODO: 8x16 sprites
            let mut chr_addr = self.reg.ppuctrl.sprite_pt_addr as usize; // Which pattern table
            chr_addr += (chr_id as usize) << 4; // Which sprite
            chr_addr += row as usize; // Which row within the tile
            chr_addr
        };

        let bit_num = 7 - (col%8);
        let mem_binding = self.memory.borrow();
        let chr = mem_binding.get_chr();
        let chr_lo = (chr[chr_addr] & (1 << bit_num)) >> bit_num;
        let chr_hi = (chr[chr_addr + 0x08]& (1 << bit_num)) >> bit_num;
        chr_hi << 1 | chr_lo
    }

    pub fn run_cycle(&mut self) {
        if self.state.scanline >= 240 && self.state.scanline < 261 {
            return;
        }

        let cycle = self.state.cycle;


        if cycle != 0 {
            let x = cycle - 1;
            let y = self.state.scanline;

            if x < 256 && y <240 {
                // println!("Rendering {},{}",x,y);

                let (bg_val, bg_palette) = {
                    if x % 8 == 0 {
                        let chr_id = self.read_nametable(x, y);
                        self.state.chr_cache = self.lookup_chr_bg(chr_id, y as u8 % 8, x as u8 % 8);
                        self.state.attr_cache = self.read_attribute_table(x, y);
                    }

                    // let chr_id = self.read_nametable(x, y); // TODO: scrolling/base address
                    // let palette_id = self.read_attribute_table(x, y);

                    // let chr_val = self.lookup_chr_bg(chr_id, y as u8 % 8, x as u8 % 8);
                    let (chr_hi, chr_lo) = self.state.chr_cache;
                    let bit_num = 7 - (x % 8);
                    let chr_lo = (chr_lo & (1 << bit_num)) >> bit_num;
                    let chr_hi = (chr_hi & (1 << bit_num)) >> bit_num;
                    let chr_val = chr_hi << 1 | chr_lo;

                    let palette_id = self.state.attr_cache;

                    (chr_val, palette_id)
                };

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
                                    true => 7 - (y - sy)
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
                    if (0x3f00..=0x3fff).contains(&self.reg.ppuaddr.get()) {
                        self.pallette[(self.reg.ppuaddr.get() as usize - 0x3f00) % 0x20]
                    } else {
                        self.pallette[0] // TODO: correct?
                    }
                } else if sprite_enable {
                    // Consider sprite
                    let (sprite_id, sprite_val, sprite_attr) = sprite_data;
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
                let color = SYSTEM_PALETTE[color_id as usize];
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
            }
            if self.state.cycle == 341 || (self.state.scanline == 261 && self.state.cycle == 340 && self.state.frame % 2 == 1) {
                self.state.scanline += 1;
                self.state.cycle = 0;
            }

            if self.state.scanline > 261 {
                self.state.frame += 1;
                self.state.scanline = 0;
            }
            self.run_cycle()
        }
        frame_complete
    }

    pub fn get_frame(&self) -> image::RgbImage {
        self.fb.clone()
    }


    pub fn render_chr(&self) -> image::GrayImage {
        let binding = self.memory.borrow();
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

    pub fn print_nametable(&self) {
        for r in 0..(240/8) {
            for c in 0..(256/8) {
                print!("{:x},",self.nametable1[r*(240/8) + c]);
            }
            println!();
        }
            println!();
            println!();
        for r in 0..(240/8) {
            for c in 0..(256/8) {
                print!("{:x},",self.nametable2[r*(240/8) + c]);
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
    }

    pub fn nmi_requested(&self) -> bool {
        self.reg.ppuctrl.nmi && self.reg.ppustatus.vblank
    }
}