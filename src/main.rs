mod cpu;
mod memory;
mod cartridge;
mod memory_utils;

use crate::cpu::Cpu;

fn main() {
    println!("Welcome to Tom's NES Emulator");
    let mut cpu = Cpu::new();
    // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/01-basics.nes"));
    // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/02-implied.nes"));
    // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/03-immediate.nes"));
    // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/14-rti.nes"));
    // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/15-brk.nes"));
    // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/16-special.nes"));
    cpu.load_rom(String::from("nes-test-roms/instr_test-v5/official_only.nes"));
    for _ in 1..20000000 {
        let (_count, done) = cpu.run_instruction();
        if done {
            break;
        }
        // if i % 100 == 0 {
            // println!();
            // println!("******************************");
            // println!("***     PROGRESS UPDATE    ***");
            // println!("Test status: {}, {:x}, {:x},{:x}", cpu.memory.read_byte(0x6000), cpu.memory.read_byte(0x6001), cpu.memory.read_byte(0x6002), cpu.memory.read_byte(0x6003));
            // println!("Test output:");
            // for i in 0..100 {
            //     let c = cpu.memory.read_byte(0x6004+i) as char;
            //     // if c != '\0' {
            //         print!("{}", c);
            //     // } else {
            //         // break;
            //     // }
            // }
            // println!("******************************");
        // }
    }
    // Test result
    println!("Test result: {}, {:x}, {:x},{:x}", cpu.memory.read_byte(0x6000), cpu.memory.read_byte(0x6001), cpu.memory.read_byte(0x6002), cpu.memory.read_byte(0x6003));
    println!("Test output:");
    for i in 0..100 {
        print!("{}", cpu.memory.read_byte(0x6004+i) as char);
    }
}
