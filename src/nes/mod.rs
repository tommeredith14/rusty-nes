pub mod cpu; // temporarily public
mod memory;
mod cartridge;
mod memory_utils;
mod ppu;
pub mod input;

use std::{cell::RefCell, rc::Rc};

use cpu::Cpu;
use ppu::Ppu;
use memory::MemoryMap;

use self::{cartridge::Cartridge, input::InputBus};

pub struct Nes {
    pub cpu: Rc<RefCell<Cpu>>,
    pub ppu: Rc<RefCell<Ppu>>,
    pub mem: Rc<RefCell<MemoryMap>>,
    pub inputs: Rc<RefCell<InputBus>>,
}

impl Nes {
    pub fn new() -> Self {
        let mem = Rc::new(RefCell::new(MemoryMap::new()));

        let ppu = Rc::new(RefCell::new(Ppu::new(mem.clone())));
        let cpu = Rc::new(RefCell::new(Cpu::new(mem.clone())));
        let inputs = Rc::new(RefCell::new(InputBus::new()));

        mem.borrow_mut().set_refs(ppu.clone(), cpu.clone(), inputs.clone());

        Nes {
            ppu,
            cpu,
            mem,
            inputs
        }

    }

    pub fn step(&mut self) -> Option<image::RgbaImage> {
        let (cpu_cycles, _) = self.cpu.borrow_mut().run_instruction();
        self.ppu.borrow_mut().advance_cycles(cpu_cycles*3);
        None
    }

    pub fn run_frame(&mut self) -> image::RgbImage {
        let mut frame_complete = false;
        while !frame_complete {
            let (cpu_cycles, _) = self.cpu.borrow_mut().run_instruction();
            frame_complete = self.ppu.borrow_mut().advance_cycles(cpu_cycles*3);
        }
        self.ppu.borrow().get_frame()
    }

    // todo move this
    pub fn load_rom(&mut self, path: String) -> Result<(), String> {
        let Ok(()) = self.mem.borrow_mut().load_rom(path) else {
            panic!("Failed to load")
        };
        self.cpu.borrow_mut().initialize();
        Ok(())
    }
}

impl Default for Nes {
    fn default() -> Self { Nes::new() }
}