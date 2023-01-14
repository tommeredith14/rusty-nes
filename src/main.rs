mod cpu;
mod memory;

use crate::cpu::Cpu;

fn main() {
    println!("Welcome to Tom's NES Emulator");
    let mut cpu = Cpu::new();
    cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/01-basics.nes"));
    // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/02-implied.nes"));
    // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/rom_singles/03-immediate.nes"));
    // cpu.load_rom(String::from("nes-test-roms/instr_test-v5/official_only.nes"));
    for _ in 1..1000000 {
        let (_count, done) = cpu.run_instruction();
        if done {
            break;
        }
    }
    // Test result
    println!("Test result: {}, {:x}, {:x},{:x}", cpu.memory.read_byte(0x6000), cpu.memory.read_byte(0x6001), cpu.memory.read_byte(0x6002), cpu.memory.read_byte(0x6003));
    println!("Test output:");
    for i in 0..100 {
        print!("{}", cpu.memory.read_byte(0x6004+i) as char);
    }
}
