
#[allow(unused)]
use crate::memory::MemoryMap;

use bitflags::bitflags;

bitflags! {
    struct Status: u8 {
        const CARRY = 0b0000_0001;
        const ZERO  = 0b0000_0010;
        const IT_DISABLE = 0b0000_0100;
        const DECIMAL = 0b0000_1000;
        const BREAK = 0b0001_0000;
        const IGNORED = 0b0010_0000;
        // const _ = 0b0011_0000;
        const OVERFLOW = 0b0100_0000;
        const NEGATIVE = 0b1000_0000;
    }
}
struct CpuRegisters {
    a: u8,
    x: u8,
    y: u8,
    pc: u16,   // Program counter
    p: Status, // Status
    s: u8,     // Stack pointer
}

mod cpu_helpers {

    use crate::memory::MemoryMap;

    use super::CpuRegisters;

    pub(super) fn push_stack(reg: &mut CpuRegisters, mem: &mut MemoryMap, val: u8) {
        mem.write_byte(reg.s as u16 + 0x100, val);
        // println!("   PUSHED {:x} to stack at {:x}", val, reg.s as u16 + 0x100);
        reg.s = reg.s.wrapping_sub(1);
    }
    pub(super) fn pop_stack(reg: &mut CpuRegisters, mem: &mut MemoryMap) -> u8 {
        reg.s = reg.s.wrapping_add(1);
        // println!("   POPPED {:x} from stack at {:x}", mem.read_byte(reg.s as u16 + 0x100), reg.s as u16 + 0x100);
        mem.read_byte(reg.s as u16 + 0x100)
    }
}

mod control_instructions {
    use crate::{memory::MemoryMap, cpu::cpu_helpers::pop_stack};

    use super::{CpuRegisters, Status, Operand, cpu_helpers::push_stack};

    pub(super) fn run_bit(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("BIT only takes addresses");
        };
        let val = mem.read_byte(addr);
        reg.p.set(Status::ZERO, (val & reg.a) == 0); // Z from A & mem
        reg.p.set(Status::NEGATIVE, (val & 0b1000_0000) != 0); // Copy bit 7 to N
        reg.p.set(Status::OVERFLOW, (val & 0b0100_0000) != 0); // Copy bit 6 to V
    }
    pub(super) fn run_bcc(reg: &mut CpuRegisters, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("BCC only takes addresses");
        };
        // print!("   {:x} -> {}",reg.p.bits, reg.p.contains(Status::CARRY));
        if !reg.p.contains(Status::CARRY) {
            reg.pc = addr;
            // print!("  to 0x{:x}", reg.pc);
        }
        // No flags
    }
    pub(super) fn run_bcs(reg: &mut CpuRegisters, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("BCS only takes addresses");
        };
        // print!("   {:x} -> {}",reg.p.bits, reg.p.contains(Status::CARRY));
        if reg.p.contains(Status::CARRY) {
            reg.pc = addr;
            // print!("  to 0x{:x}", reg.pc);
        }
        // No flags
    }
    pub(super) fn run_beq(reg: &mut CpuRegisters, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("BEQ only takes addresses");
        };
        if reg.p.contains(Status::ZERO) {
            reg.pc = addr;
            // print!("  to 0x{:x}", reg.pc);
        }
        // No flags
    }
    pub(super) fn run_bmi(reg: &mut CpuRegisters, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("BMI only takes addresses");
        };
        if reg.p.contains(Status::NEGATIVE) {
            reg.pc = addr;
            // print!("  to 0x{:x}", reg.pc);
        }
        // No flags
    }
    pub(super) fn run_bne(reg: &mut CpuRegisters, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("BNE only takes addresses");
        };
        if !reg.p.contains(Status::ZERO) {
            reg.pc = addr;
            // print!("  to 0x{:x}", reg.pc);
        }
        // No flags
    }
    pub(super) fn run_bpl(reg: &mut CpuRegisters, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("BPL only takes addresses");
        };
        if !reg.p.contains(Status::NEGATIVE) {
            reg.pc = addr;
            // print!("  to 0x{:x}", reg.pc);
        }
        // No flags
    }
    pub(super) fn run_brk(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        assert_eq!(operand, Operand::None);

        // Push PC + 2 to stack
        let return_addr = reg.pc +2-1; // Should point to the next instruction
        push_stack(reg, mem, ((return_addr & 0xFF00) >> 8) as u8);
        push_stack(reg, mem, (return_addr & 0x00FF) as u8);
        

        // println!("Status: {:x}", reg.p.bits());

        push_stack(reg, mem, (reg.p | Status::BREAK | Status::IGNORED).bits);

        // Initiate interrupt

        let irq_addr = mem.read_word(0xfffe);
        reg.p.set(Status::IT_DISABLE, true);
        reg.pc = irq_addr
    }
    pub(super) fn run_rti(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        assert_eq!(operand, Operand::None);

        // Pop flags fom register
        let flags = Status::from_bits(pop_stack(reg, mem)).unwrap();
        reg.p.set(Status::NEGATIVE, flags.contains(Status::NEGATIVE));
        reg.p.set(Status::OVERFLOW, flags.contains(Status::OVERFLOW));
        reg.p.set(Status::DECIMAL, flags.contains(Status::DECIMAL));
        reg.p.set(Status::IT_DISABLE, flags.contains(Status::IT_DISABLE));
        reg.p.set(Status::ZERO, flags.contains(Status::ZERO));
        reg.p.set(Status::CARRY, flags.contains(Status::CARRY));
        reg.p.set(Status::IGNORED, true); // TODO: needed?

        // println!("Status: {:x}", reg.p.bits());

        // Pop the return address
        let pc = pop_stack(reg, mem) as u16;
        let pc = pc | ((pop_stack(reg, mem) as u16) << 8);
        reg.pc = pc;

    }
    pub(super) fn run_bvc(reg: &mut CpuRegisters, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("BVC only takes addresses");
        };
        // print!("   {:x} -> {}",reg.p.bits, reg.p.contains(Status::CARRY));
        if !reg.p.contains(Status::OVERFLOW) {
            reg.pc = addr;
            // print!("  to 0x{:x}", reg.pc);
        }
        // No flags
    }
    pub(super) fn run_bvs(reg: &mut CpuRegisters, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("BVS only takes addresses");
        };
        // print!("   {:x} -> {}",reg.p.bits, reg.p.contains(Status::CARRY));
        if reg.p.contains(Status::OVERFLOW) {
            reg.pc = addr;
            // print!("  to 0x{:x}", reg.pc);
        }
        // No flags
    }
    pub(super) fn run_clc(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.p.remove(Status::CARRY);
        // No flags
    }
    pub(super) fn run_cld(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.p.remove(Status::DECIMAL);
        // No flags
    }
    pub(super) fn run_cli(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.p.remove(Status::IT_DISABLE);
        // No flags
    }
    pub(super) fn run_clv(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.p.remove(Status::OVERFLOW);
        // No flags
    }
    pub(super) fn run_cpx(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(val) => val,
            Operand::None => panic!("CPX requires an operand") 
        };
        let result = reg.x.wrapping_sub(val);

        // flags: https://www.pagetable.com/c64ref/6502/?tab=2#CMP
        reg.p.set(Status::ZERO, result == 0);
        reg.p.set(Status::NEGATIVE, result & 0x80 != 0);
        reg.p.set(Status::CARRY, val <= reg.x);

    }    
    pub(super) fn run_cpy(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(val) => val,
            Operand::None => panic!("CPY requires an operand") 
        };
        let result = reg.y.wrapping_sub(val);

        // flags: https://www.pagetable.com/c64ref/6502/?tab=2#CMP
        reg.p.set(Status::ZERO, result == 0);
        reg.p.set(Status::NEGATIVE, result & 0x80 != 0);
        reg.p.set(Status::CARRY, val <= reg.y);

    }    
    pub(super) fn run_dey(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        let val = match reg.y {
            1..=0xFF => reg.y - 1,
            0 => 0xFF
        };
        // print!(" DEY {:x} -> {:x}", reg.y, val);
        reg.y = val;
        // flags:
        reg.p.set(Status::ZERO, val == 0);
        reg.p.set(Status::NEGATIVE, val & 0x80 != 0);
    }
    pub(super) fn run_inx(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        let val = match reg.x {
            0..=0xFE => reg.x + 1,
            0xFF => 0
        };
        reg.x = val;
        // flags:
        reg.p.set(Status::ZERO, val == 0);
        reg.p.set(Status::NEGATIVE, val & 0x80 != 0);
    }
    pub(super) fn run_iny(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        let val = match reg.y {
            0..=0xFE => reg.y + 1,
            0xFF => 0
        };
        reg.y = val;
        // flags:
        reg.p.set(Status::ZERO, val == 0);
        reg.p.set(Status::NEGATIVE, val & 0x80 != 0);
    }
    pub(super) fn run_jmp(reg: &mut CpuRegisters, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("JMP only takes addresses");
        };
        reg.pc = addr;
        // println!("Jump to 0x{:x}", reg.pc);
        // TODO: flags??
    }
    pub(super) fn run_jsr(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("JSR only takes addresses");
        };
        // Push PC to stack
        let return_addr = reg.pc - 1; // Should point to the last read byte
        push_stack(reg, mem, ((return_addr & 0xFF00) >> 8) as u8);
        push_stack(reg, mem, (return_addr & 0x00FF) as u8);

        // Jump to addr
        reg.pc = addr;
        // print!(" to 0x{:x}", reg.pc);

        // No flags
    }
    pub(super) fn run_ldy(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(val) => val,
            Operand::None => panic!("LDY requires an operand") 
        };
        reg.y = val;

        // flags:
        reg.p.set(Status::ZERO, val == 0);
        reg.p.set(Status::NEGATIVE, val & 0x80 != 0);
    }
    pub(super) fn run_pha(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        assert_eq!(operand, Operand::None);
        push_stack(reg, mem, reg.a);
        // No flags
    }
    pub(super) fn run_php(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        assert_eq!(operand, Operand::None);
        push_stack(reg, mem, reg.p.bits | Status::BREAK.bits | Status::IGNORED.bits);
        // No flags
    }
    pub(super) fn run_pla(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.a = pop_stack(reg, mem);
        // flags:
        reg.p.set(Status::ZERO, reg.a == 0);
        reg.p.set(Status::NEGATIVE, reg.a & 0x80 != 0);
    }
    pub(super) fn run_plp(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        assert_eq!(operand, Operand::None);
        let val = pop_stack(reg, mem);
        reg.p.bits = val;
        // No flags:
    }
    pub(super) fn run_rts(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        assert_eq!(operand, Operand::None);
        let bl = pop_stack(reg, mem) as u16;//mem.read_byte(reg.s.into()) as u16;
        let bh = pop_stack(reg, mem) as u16;// mem.read_byte(reg.s.into()) as u16;

        let return_addr = (bh << 8) | bl;
        // print!("  {:x} & {:x} -> {:x}", bh, bl, return_addr);
        reg.pc = return_addr + 1;
        // No flags
    }
    pub(super) fn run_sec(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.p.insert(Status::CARRY);
        // No flags
    }
    pub(super) fn run_sed(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.p.insert(Status::DECIMAL);
        // No flags
    }
    pub(super) fn run_sei(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.p.insert(Status::IT_DISABLE);
        // No flags
    }
    pub(super) fn run_sty(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("STY only takes addresses");
        };
        mem.write_byte(addr, reg.y);
        // No flags
    }
    pub(super) fn run_tay(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.y = reg.a;
        // flags:
        reg.p.set(Status::ZERO, reg.y == 0);
        reg.p.set(Status::NEGATIVE, reg.y & 0x80 != 0);        
    }
    pub(super) fn run_tya(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.a = reg.y;
        // print!("  TYA: A -> {:x}", reg.a);
        // flags:
        reg.p.set(Status::ZERO, reg.a == 0);
        reg.p.set(Status::NEGATIVE, reg.a & 0x80 != 0);        
    }
}

mod alu_instructions {
    use crate::memory::MemoryMap;

    use super::{Operand, CpuRegisters, Status};

    pub(super) fn run_cmp(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(val) => val,
            Operand::None => panic!("CMP requires an operand") 
        };
        let result = reg.a.wrapping_sub(val);

        // flags: https://www.pagetable.com/c64ref/6502/?tab=2#CMP
        reg.p.set(Status::ZERO, result == 0);
        reg.p.set(Status::NEGATIVE, result & 0x80 != 0);
        reg.p.set(Status::CARRY, val <= reg.a);

    }    
    pub(super) fn run_sta(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("STA only takes addresses");
        };
        mem.write_byte(addr, reg.a);
        // println!("Stored A {:x} to mem {:x}", reg.a, addr);
    }

    pub(super) fn run_lda(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(val) => val,
            Operand::None => panic!("LDA requires an operand") 
        };
        // println!("  loaded {}", val);
        reg.a = val;

        // flags:
        reg.p.set(Status::ZERO, reg.a == 0);
        reg.p.set(Status::NEGATIVE, reg.a & 0x80 != 0);
    }
    pub(super) fn run_ora(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(val) => val,
            Operand::None => panic!("ORA requires an operand") 
        };
        reg.a |= val;

        // flags:
        reg.p.set(Status::ZERO, reg.a == 0);
        reg.p.set(Status::NEGATIVE, reg.a & 0x80 != 0);

        // print!("   ORA result {:x}", reg.a);
    }
    pub(super) fn run_eor(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(val) => val,
            Operand::None => panic!("EOR requires an operand") 
        };
        reg.a ^= val;

        // flags:
        reg.p.set(Status::ZERO, reg.a == 0);
        reg.p.set(Status::NEGATIVE, reg.a & 0x80 != 0);

        // print!("   EOR result {:x}", reg.a);
    }    
    pub(super) fn run_and(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(val) => val,
            Operand::None => panic!("AND requires an operand") 
        };
        // print!("  {:b} & {:b} = ", reg.a, val);
        reg.a &= val;
        // print!("{:b}", reg.a);

        // flags:
        reg.p.set(Status::ZERO, reg.a == 0);
        reg.p.set(Status::NEGATIVE, reg.a & 0x80 != 0);

        // print!("   AND result {:x}", reg.a);
    }    
    pub(super) fn run_adc(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(val) => val,
            Operand::None => panic!("SBC requires an operand") 
        };
        // print!(" {} + {}, {}", reg.a, val, reg.p.contains(Status::CARRY));
        let mut result = reg.a as u16 + val as u16;
        if reg.p.contains(Status::CARRY) {
            result += 1;
        }

        let truncated_result = (result & 0x00FF) as u8;
        let overflow: bool = (!(reg.a ^ val))&(reg.a ^ truncated_result)&0x80 != 0;
        reg.a = truncated_result;

        // flags: https://www.pagetable.com/c64ref/6502/?tab=2#SBC
        reg.p.set(Status::ZERO, truncated_result == 0);
        reg.p.set(Status::NEGATIVE, truncated_result & 0x80 != 0);
        reg.p.set(Status::CARRY, result > 0xFF);
        reg.p.set(Status::OVERFLOW, overflow);
        // print!("= {} -> {}, {:b}", result, truncated_result, reg.p.bits)

    }    
    pub(super) fn run_sbc(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(val) => val,
            Operand::None => panic!("SBC requires an operand") 
        };

        run_adc(reg, mem, Operand::Value(!val));
        // print!(" {} - {}, {}", reg.a, val, reg.p.contains(Status::CARRY));
        // print!("  ({} - {})  ",reg.a as i8 as i16, val as i8 as i16);
        // let mut result = reg.a as i8 as i16 - val as i8 as i16;
        // if !reg.p.contains(Status::CARRY) {
        //     result -= 1;
        // }

        // let truncated_result = result as i8 as u8;
        // reg.a = truncated_result;

        // // flags: https://www.pagetable.com/c64ref/6502/?tab=2#SBC
        // reg.p.set(Status::ZERO, truncated_result == 0);
        // reg.p.set(Status::NEGATIVE, truncated_result & 0x80 != 0);
        // reg.p.set(Status::CARRY, result >= 0);
        // reg.p.set(Status::OVERFLOW, result > i8::MAX as i16 || result < i8::MIN as i16);
        // print!("= {} -> {}, {:b}", result, truncated_result, reg.p.bits)

    }    
}

mod rmw_instructions {
    use crate::memory::MemoryMap;
    use super::{Operand, CpuRegisters, Status};
    pub(super) fn run_asl(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(_) => panic!("ASL Operates on A or memory"),
            Operand::None => reg.a
        };
        let carry = (val & 0x80) != 0;
        let result = val << 1;
        match operand {
            Operand::Address(addr) => mem.write_byte(addr,result),
            Operand::Value(_) => panic!("ASL Operates on A or memory"),
            Operand::None => reg.a = result
        };

        // flags: https://www.pagetable.com/c64ref/6502/?tab=2#LSR
        reg.p.set(Status::ZERO, result == 0);
        reg.p.set(Status::CARRY, carry);
        reg.p.set(Status::NEGATIVE, result & 0x80 != 0);
    }
    pub(super) fn run_rol(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(_) => panic!("ROL Operates on A or memory"),
            Operand::None => reg.a
        };
        let carry = (val & 0x80) != 0;
        let result = val << 1 | reg.p.contains(Status::CARRY) as u8;
        match operand {
            Operand::Address(addr) => mem.write_byte(addr,result),
            Operand::Value(_) => panic!("ROL Operates on A or memory"),
            Operand::None => reg.a = result
        };

        // flags: https://www.pagetable.com/c64ref/6502/?tab=2#LSR
        reg.p.set(Status::ZERO, result == 0);
        reg.p.set(Status::CARRY, carry);
        reg.p.set(Status::NEGATIVE, result & 0x80 != 0);
    }
    pub(super) fn run_ror(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(_) => panic!("ROR Operates on A or memory"),
            Operand::None => reg.a
        };
        let carry = (val & 0x01) == 1;
        let result = val >> 1 | if reg.p.contains(Status::CARRY) {0x80} else {0x0};
        match operand {
            Operand::Address(addr) => mem.write_byte(addr,result),
            Operand::Value(_) => panic!("ROR Operates on A or memory"),
            Operand::None => reg.a = result
        };

        // flags: https://www.pagetable.com/c64ref/6502/?tab=2#LSR
        reg.p.set(Status::ZERO, result == 0);
        reg.p.set(Status::CARRY, carry);
        reg.p.set(Status::NEGATIVE, result & 0x80 != 0);
    }
    pub(super) fn run_dec(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let addr = match operand {
            Operand::Address(addr) => addr,
            _ => panic!("Dec Operates on memory"),
        };
        let val = mem.read_byte(addr);
        let val = match val {
            1..=0xFF => val - 1,
            0 => 0xFF
        };
        mem.write_byte(addr,val);
        // flags:
        reg.p.set(Status::ZERO, val == 0);
        reg.p.set(Status::NEGATIVE, val & 0x80 != 0);
    }
    pub(super) fn run_dex(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        let val = match reg.x {
            1..=0xFF => reg.x - 1,
            0 => 0xFF
        };
        // print!(" DEX {:x} -> {:x}", reg.x, val);
        reg.x = val;
        // flags:
        reg.p.set(Status::ZERO, val == 0);
        reg.p.set(Status::NEGATIVE, val & 0x80 != 0);
    }
    pub(super) fn run_inc(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("INC only takes addresses");
        };
        let val = mem.read_byte(addr);
        let val = match val {
            0..=0xFE => val + 1,
            0xFF => 0
        };
        mem.write_byte(addr, val);
        // flags:
        reg.p.set(Status::ZERO, val == 0);
        reg.p.set(Status::NEGATIVE, val & 0x80 != 0);
    }
    pub(super) fn run_ldx(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let val = match operand {
            Operand::Address(addr) => mem.read_byte(addr),
            Operand::Value(val) => val,
            Operand::None => panic!("LDx requires an operand") 
        };
        reg.x = val;

        // flags:
        reg.p.set(Status::ZERO, val == 0);
        reg.p.set(Status::NEGATIVE, val & 0x80 != 0);
    }
    pub(super) fn run_lsr(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let (z, c) = match operand {
            Operand::Address(addr) => {
                let val = mem.read_byte(addr);
                let carry = (val & 0x01) == 1;
                let val = val >> 1;
                mem.write_byte(addr, val);
                (val == 0, carry)
            },
            Operand::Value(_) => panic!("LSR Operates on A or memory"),
            Operand::None =>  {
                let carry = (reg.a & 0x01) == 1;
                reg.a >>= 1;
                (reg.a == 0, carry)
            }
        };

        // flags: https://www.pagetable.com/c64ref/6502/?tab=2#LSR
        reg.p.set(Status::ZERO, z);
        reg.p.set(Status::CARRY, c);
        reg.p.set(Status::NEGATIVE, false);
    }
    pub(super) fn run_stx(reg: &mut CpuRegisters, mem: &mut MemoryMap, operand: Operand) {
        let Operand::Address(addr) = operand else {
            panic!("STX only takes addresses");
        };
        mem.write_byte(addr, reg.x);
        // No flags
    }
    pub(super) fn run_tax(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.x = reg.a;
        // flags:
        reg.p.set(Status::ZERO, reg.x == 0);
        reg.p.set(Status::NEGATIVE, reg.x & 0x80 != 0);        
    }
    pub(super) fn run_tsx(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.x = reg.s;
        // flags:
        reg.p.set(Status::ZERO, reg.x == 0);
        reg.p.set(Status::NEGATIVE, reg.x & 0x80 != 0);        
    }
    pub(super) fn run_txa(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        reg.a = reg.x;
        // print!("  TXA: A -> {:x}", reg.a);
        // flags:
        reg.p.set(Status::ZERO, reg.a == 0);
        reg.p.set(Status::NEGATIVE, reg.a & 0x80 != 0);        
    }
    pub(super) fn run_txs(reg: &mut CpuRegisters, operand: Operand) {
        assert_eq!(operand, Operand::None);
        // println!("  TXS: OVERRITING STACK TO -> {:x}", reg.x);
        reg.s = reg.x;
    }

}

struct LoopDetection {
    last_pc: u16,
    repeats: usize
}

pub struct Cpu {
    registers: CpuRegisters,
    // cycle_count: u64,
    pub memory: MemoryMap,
    loop_detection: LoopDetection
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            registers: CpuRegisters {
                a: 0,
                x: 0,
                y: 0,
                pc: 0,
                p: Status { bits: Status::IGNORED.bits },
                s: 0xff,
            },
            // cycle_count: 0,
            memory: MemoryMap::new(),
            loop_detection: LoopDetection { last_pc: 0, repeats: 0 }
        }
    }

    fn read_byte_pc(&mut self) -> u8 {
        // println!("PC at {:x}", self.registers.pc);
        let ret = self.memory.read_byte(self.registers.pc);
        self.registers.pc += 1;
        ret
    }

    fn read_word_pc(&mut self) -> u16 {
        let ret = self.memory.read_word(self.registers.pc);
        self.registers.pc += 2;
        ret
    }

    pub fn run_instruction(&mut self) -> (u64, bool) {
        let instruction = self.read_byte_pc();
        // print!(
        //     "Instr {:x} at pc {:x} -> ",
        //     instruction, self.registers.pc-1
        // );
        let instruction = InstructionType::from(instruction);
        // println!("{:?}",instruction);
        match instruction {
            InstructionType::Control(inst, mode) => self.run_control_instruction(inst, mode),
            InstructionType::Alu(inst, mode) => self.run_alu_instruction(inst, mode),
            InstructionType::Rmw(inst, mode) => self.run_rmw_instruction(inst, mode),
            InstructionType::Nop(mode) => self.run_nop_instruction(mode),
            // InstructionType::Unofficial => todo!(),
        };
        if self.registers.pc == self.loop_detection.last_pc {
            self.loop_detection.repeats += 1;
        } else {
            self.loop_detection.repeats = 0;
        }
        let loop_detected = self.loop_detection.repeats >= 3;
        self.loop_detection.last_pc = self.registers.pc;
        (1, loop_detected)
    }

    fn parse_operand(&mut self, mode: AddressMode) -> Operand {
        match mode {
            AddressMode::Implied => Operand::None,
            AddressMode::Acc => Operand::None,
            AddressMode::Abs => Operand::Address(self.read_word_pc()),
            AddressMode::AbsX => Operand::Address(self.read_word_pc() + self.registers.x as u16),
            AddressMode::AbsY => Operand::Address(self.read_word_pc() + self.registers.y as u16),
            AddressMode::Imm => Operand::Value(self.read_byte_pc()),
            AddressMode::Ind => {
                let addr_location = self.read_word_pc();
                // Indir wraps on page boundaries!
                let lo_byte = self.memory.read_byte(addr_location);
                let hi_byte_loc = (addr_location & 0xFF00) | (((addr_location & 0xFF) as u8).wrapping_add(1) as u16);
                let hi_byte = self.memory.read_byte(hi_byte_loc);

                let addr = ((hi_byte as u16) << 8) | (lo_byte as u16);
                Operand::Address(addr)
            },
            AddressMode::IndX => {
                let ll_addr = self.read_byte_pc();
                let addr_location = ll_addr.wrapping_add(self.registers.x);
                let addr = (self.memory.read_byte(addr_location as u16) as u16) |
                    ((self.memory.read_byte(addr_location.wrapping_add(1) as u16) as u16) << 8);
                Operand::Address(addr)
            },
            AddressMode::IndY => {
                let ll_addr = self.read_byte_pc();
                let addr = self.memory.read_byte(ll_addr as u16) as u16 | ((self.memory.read_byte(ll_addr.wrapping_add(1) as u16) as u16) << 8);
                Operand::Address(addr.wrapping_add(self.registers.y as u16))
            }, // TODO: carry
            AddressMode::Rel => {
                // let orig_pc = self.registers.pc - 1;
                // let opu8 = self.read_byte_pc();
                // let opi8 = opu8 as i8;
                // let res = orig_pc.wrapping_add((opi8) as u16);
                // println!("{:x} + {:x}({}) = {:x}",orig_pc, opu8, opi8, res);
                let opi8 = self.read_byte_pc() as i8;
                let res = self.registers.pc.wrapping_add((opi8) as u16); // My guess: acts in incremented pc
                Operand::Address(res)
            },
            AddressMode::Zpg => Operand::Address(self.read_byte_pc() as u16), // addr 00BB
            AddressMode::ZpgX => Operand::Address(self.read_byte_pc().wrapping_add(self.registers.x) as u16),
            AddressMode::ZpgY => Operand::Address(self.read_byte_pc().wrapping_add(self.registers.y) as u16),
        }
    }

    fn run_control_instruction(&mut self, inst: ControlInstruction, mode: AddressMode) {
        let operand = self.parse_operand(mode);
        match inst {
            ControlInstruction::Bcc => control_instructions::run_bcc(&mut self.registers, operand),
            ControlInstruction::Bcs => control_instructions::run_bcs(&mut self.registers, operand),
            ControlInstruction::Beq => control_instructions::run_beq(&mut self.registers, operand),
            ControlInstruction::Bit => control_instructions::run_bit(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Bmi => control_instructions::run_bmi(&mut self.registers, operand),
            ControlInstruction::Bne => control_instructions::run_bne(&mut self.registers, operand),
            ControlInstruction::Bpl => control_instructions::run_bpl(&mut self.registers, operand),
            ControlInstruction::Brk => control_instructions::run_brk(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Bvc => control_instructions::run_bvc(&mut self.registers, operand),
            ControlInstruction::Bvs => control_instructions::run_bvs(&mut self.registers, operand),
            ControlInstruction::Clc => control_instructions::run_clc(&mut self.registers, operand),
            ControlInstruction::Cld => control_instructions::run_cld(&mut self.registers, operand),
            ControlInstruction::Cli => control_instructions::run_cli(&mut self.registers, operand),
            ControlInstruction::Clv => control_instructions::run_clv(&mut self.registers, operand),
            ControlInstruction::Cpx => control_instructions::run_cpx(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Cpy => control_instructions::run_cpy(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Dey => control_instructions::run_dey(&mut self.registers, operand),
            ControlInstruction::Inx => control_instructions::run_inx(&mut self.registers, operand),
            ControlInstruction::Iny => control_instructions::run_iny(&mut self.registers, operand),
            ControlInstruction::Jmp => control_instructions::run_jmp(&mut self.registers, operand),
            ControlInstruction::Jsr => control_instructions::run_jsr(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Ldy => control_instructions::run_ldy(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Pha => control_instructions::run_pha(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Php => control_instructions::run_php(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Pla => control_instructions::run_pla(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Plp => control_instructions::run_plp(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Rti => control_instructions::run_rti(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Rts => control_instructions::run_rts(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Sec => control_instructions::run_sec(&mut self.registers, operand),
            ControlInstruction::Sed => control_instructions::run_sed(&mut self.registers, operand),
            ControlInstruction::Sei => control_instructions::run_sei(&mut self.registers, operand),
            ControlInstruction::Sty => control_instructions::run_sty(&mut self.registers, &mut self.memory, operand),
            ControlInstruction::Tay => control_instructions::run_tay(&mut self.registers, operand),
            ControlInstruction::Tya => control_instructions::run_tya(&mut self.registers, operand),
        }
    }

    fn run_alu_instruction(&mut self, inst: AluInstruction, mode: AddressMode) {
        use alu_instructions::*;
        let operand = self.parse_operand(mode);
        match inst {
            AluInstruction::Adc => run_adc(&mut self.registers, &mut self.memory, operand),
            AluInstruction::And => run_and(&mut self.registers, &mut self.memory, operand),
            AluInstruction::Cmp => run_cmp(&mut self.registers, &mut self.memory, operand),
            AluInstruction::Eor => run_eor(&mut self.registers, &mut self.memory, operand),
            AluInstruction::Lda => run_lda(&mut self.registers, &mut self.memory, operand),
            AluInstruction::Ora => run_ora(&mut self.registers, &mut self.memory, operand),
            AluInstruction::Sbc => run_sbc(&mut self.registers, &mut self.memory, operand),
            AluInstruction::Sta => run_sta(&mut self.registers, &mut self.memory, operand),
        }
    }

    fn run_rmw_instruction(&mut self, inst: RmwInstruction, mode: AddressMode) {
        use rmw_instructions::*;
        let operand = self.parse_operand(mode);
        match inst {
            RmwInstruction::Asl => run_asl(&mut self.registers, &mut self.memory, operand),
            RmwInstruction::Dec => run_dec(&mut self.registers, &mut self.memory, operand),
            RmwInstruction::Dex => run_dex(&mut self.registers, operand),
            RmwInstruction::Inc => run_inc(&mut self.registers, &mut self.memory, operand),
            RmwInstruction::Ldx => run_ldx(&mut self.registers, &mut self.memory, operand),
            RmwInstruction::Lsr => run_lsr(&mut self.registers, &mut self.memory, operand),
            RmwInstruction::Rol => run_rol(&mut self.registers, &mut self.memory, operand),
            RmwInstruction::Ror => run_ror(&mut self.registers, &mut self.memory, operand),
            RmwInstruction::Stx => run_stx(&mut self.registers, &mut self.memory, operand),
            RmwInstruction::Tax => run_tax(&mut self.registers, operand),
            RmwInstruction::Tsx => run_tsx(&mut self.registers, operand),
            RmwInstruction::Txa => run_txa(&mut self.registers, operand),
            RmwInstruction::Txs => run_txs(&mut self.registers, operand),
        }
    }
    fn run_nop_instruction(&mut self, mode: AddressMode) {
        let _ = self.parse_operand(mode);
    }

    // fn instruction_type(instruction: u8) -> InstructionType {
    //     match instruction & 0x03 {
    //         0x00 => InstructionType::Control,
    //         0x01 => InstructionType::Alu,
    //         0x02 => InstructionType::Rmw,
    //         0x03 => InstructionType::Unofficial,
    //         _ => unreachable!()
    //     }
    // }

    // todo move this
    pub fn load_rom(&mut self, path: String) {
        let Ok(()) = self.memory.load_rom(path) else {
            panic!("Failed to load")
        };
        self.registers.pc = self.memory.read_word(0xFFFC); // Reset
        // TODO: need better way to fake ppu
        self.memory.write_byte(0x2002, 0x80);// Fake malfunctioning PPUSTATUS register
        println!(
            "Pc now at {:x}-> {}",
            self.registers.pc,
            self.memory.read_byte(self.registers.pc)
        );
    }
}

#[derive(Debug)]
enum ControlInstruction {
    Bcc,
    Bcs,
    Beq,
    Bit,
    Bmi,
    Bne,
    Bpl,
    Brk,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Cpx,
    Cpy,
    Dey,
    Inx,
    Iny,
    Jmp,
    Jsr,
    Ldy,
    Pha,
    Php,
    Pla,
    Plp,
    Rti,
    Rts,
    Sec,
    Sed,
    Sei,
    Sty,
    Tay,
    Tya,
}

#[derive(Debug)]
enum AluInstruction {
    Ora,
    And,
    Eor,
    Adc,
    Sta,
    Lda,
    Cmp,
    Sbc,
}

#[derive(Debug)]
enum RmwInstruction {
    Asl,
    Dec,
    Dex,
    Inc,
    Ldx,
    Lsr,
    Rol,
    Ror,
    Stx,
    Tax,
    Tsx,
    Txs,
    Txa,
}

#[derive(Debug)]
enum AddressMode {
    Implied,
    Acc,
    Abs,
    AbsX,
    AbsY,
    Imm,
    Ind,
    IndX,
    IndY,
    Rel,
    Zpg,
    ZpgX,
    ZpgY,
}

#[derive(PartialEq)]
#[derive(Debug)]
enum Operand {
    Address(u16),
    Value(u8),
    None, // Implied or Accumulator (no overlap)
}

#[derive(Debug)]
enum InstructionType {
    Control(ControlInstruction, AddressMode),
    Alu(AluInstruction, AddressMode),
    Rmw(RmwInstruction, AddressMode),
    Nop(AddressMode)
    // Unofficial,
}

impl From<u8> for InstructionType {
    fn from(instruction: u8) -> Self {
        match instruction {
            0x00 => InstructionType::Control(ControlInstruction::Brk, AddressMode::Implied),
            0x01 => InstructionType::Alu(AluInstruction::Ora, AddressMode::IndX),
            0x05 => InstructionType::Alu(AluInstruction::Ora, AddressMode::Zpg),
            0x06 => InstructionType::Rmw(RmwInstruction::Asl, AddressMode::Zpg),
            0x08 => InstructionType::Control(ControlInstruction::Php, AddressMode::Implied),
            0x09 => InstructionType::Alu(AluInstruction::Ora, AddressMode::Imm),
            0x0A => InstructionType::Rmw(RmwInstruction::Asl, AddressMode::Acc),
            0x0D => InstructionType::Alu(AluInstruction::Ora, AddressMode::Abs),
            0x0E => InstructionType::Rmw(RmwInstruction::Asl, AddressMode::Abs),

            0x10 => InstructionType::Control(ControlInstruction::Bpl, AddressMode::Rel),
            0x11 => InstructionType::Alu(AluInstruction::Ora, AddressMode::IndY),
            0x15 => InstructionType::Alu(AluInstruction::Ora, AddressMode::ZpgX),
            0x16 => InstructionType::Rmw(RmwInstruction::Asl, AddressMode::ZpgX),
            0x18 => InstructionType::Control(ControlInstruction::Clc, AddressMode::Implied),
            0x19 => InstructionType::Alu(AluInstruction::Ora, AddressMode::AbsY),
            0x1A => InstructionType::Nop(AddressMode::Implied), // Unofficial
            0x1D => InstructionType::Alu(AluInstruction::Ora, AddressMode::AbsX),
            0x1E => InstructionType::Rmw(RmwInstruction::Asl, AddressMode::AbsX),

            0x20 => InstructionType::Control(ControlInstruction::Jsr, AddressMode::Abs),
            0x21 => InstructionType::Alu(AluInstruction::And, AddressMode::IndX),
            0x24 => InstructionType::Control(ControlInstruction::Bit, AddressMode::Zpg),
            0x25 => InstructionType::Alu(AluInstruction::And, AddressMode::Zpg),
            0x26 => InstructionType::Rmw(RmwInstruction::Rol, AddressMode::Zpg),
            0x28 => InstructionType::Control(ControlInstruction::Plp, AddressMode::Implied),
            0x29 => InstructionType::Alu(AluInstruction::And, AddressMode::Imm),
            0x2A => InstructionType::Rmw(RmwInstruction::Rol, AddressMode::Acc),
            0x2C => InstructionType::Control(ControlInstruction::Bit, AddressMode::Abs),
            0x2D => InstructionType::Alu(AluInstruction::And, AddressMode::Abs),
            0x2E => InstructionType::Rmw(RmwInstruction::Rol, AddressMode::Abs),

            0x30 => InstructionType::Control(ControlInstruction::Bmi, AddressMode::Rel),
            0x31 => InstructionType::Alu(AluInstruction::And, AddressMode::IndY),
            0x35 => InstructionType::Alu(AluInstruction::And, AddressMode::ZpgX),
            0x36 => InstructionType::Rmw(RmwInstruction::Rol, AddressMode::ZpgX),
            0x38 => InstructionType::Control(ControlInstruction::Sec, AddressMode::Implied),
            0x39 => InstructionType::Alu(AluInstruction::And, AddressMode::AbsY),
            0x3A => InstructionType::Nop(AddressMode::Implied), // Unofficial
            0x3D => InstructionType::Alu(AluInstruction::And, AddressMode::AbsX),
            0x3E => InstructionType::Rmw(RmwInstruction::Rol, AddressMode::AbsX),

            0x40 => InstructionType::Control(ControlInstruction::Rti, AddressMode::Implied),
            0x41 => InstructionType::Alu(AluInstruction::Eor, AddressMode::IndX),
            0x45 => InstructionType::Alu(AluInstruction::Eor, AddressMode::Zpg),
            0x46 => InstructionType::Rmw(RmwInstruction::Lsr, AddressMode::Zpg),
            0x48 => InstructionType::Control(ControlInstruction::Pha, AddressMode::Implied),
            0x49 => InstructionType::Alu(AluInstruction::Eor, AddressMode::Imm),
            0x4A => InstructionType::Rmw(RmwInstruction::Lsr, AddressMode::Acc),
            0x4C => InstructionType::Control(ControlInstruction::Jmp, AddressMode::Abs),
            0x4D => InstructionType::Alu(AluInstruction::Eor, AddressMode::Abs),
            0x4E => InstructionType::Rmw(RmwInstruction::Lsr, AddressMode::Abs),

            0x50 => InstructionType::Control(ControlInstruction::Bvc, AddressMode::Rel),
            0x51 => InstructionType::Alu(AluInstruction::Eor, AddressMode::IndY),
            0x55 => InstructionType::Alu(AluInstruction::Eor, AddressMode::ZpgX),
            0x56 => InstructionType::Rmw(RmwInstruction::Lsr, AddressMode::ZpgX),
            0x58 => InstructionType::Control(ControlInstruction::Cli, AddressMode::Implied),
            0x59 => InstructionType::Alu(AluInstruction::Eor, AddressMode::AbsY),
            0x5A => InstructionType::Nop(AddressMode::Implied), // Unofficial
            0x5D => InstructionType::Alu(AluInstruction::Eor, AddressMode::AbsX),
            0x5E => InstructionType::Rmw(RmwInstruction::Lsr, AddressMode::AbsX),

            0x60 => InstructionType::Control(ControlInstruction::Rts, AddressMode::Implied),
            0x61 => InstructionType::Alu(AluInstruction::Adc, AddressMode::IndX),
            0x65 => InstructionType::Alu(AluInstruction::Adc, AddressMode::Zpg),
            0x66 => InstructionType::Rmw(RmwInstruction::Ror, AddressMode::Zpg),
            0x68 => InstructionType::Control(ControlInstruction::Pla, AddressMode::Implied),
            0x69 => InstructionType::Alu(AluInstruction::Adc, AddressMode::Imm),
            0x6A => InstructionType::Rmw(RmwInstruction::Ror, AddressMode::Acc),
            0x6C => InstructionType::Control(ControlInstruction::Jmp, AddressMode::Ind),
            0x6D => InstructionType::Alu(AluInstruction::Adc, AddressMode::Abs),
            0x6E => InstructionType::Rmw(RmwInstruction::Ror, AddressMode::Abs),

            0x70 => InstructionType::Control(ControlInstruction::Bvs, AddressMode::Rel),
            0x71 => InstructionType::Alu(AluInstruction::Adc, AddressMode::IndY),
            0x75 => InstructionType::Alu(AluInstruction::Adc, AddressMode::ZpgX),
            0x76 => InstructionType::Rmw(RmwInstruction::Ror, AddressMode::ZpgX),
            0x78 => InstructionType::Control(ControlInstruction::Sei, AddressMode::Implied),
            0x79 => InstructionType::Alu(AluInstruction::Adc, AddressMode::AbsY),
            0x7A => InstructionType::Nop(AddressMode::Implied), // Unofficial
            0x7D => InstructionType::Alu(AluInstruction::Adc, AddressMode::AbsX),
            0x7E => InstructionType::Rmw(RmwInstruction::Ror, AddressMode::AbsX),

            0x81 => InstructionType::Alu(AluInstruction::Sta, AddressMode::IndX),
            0x84 => InstructionType::Control(ControlInstruction::Sty, AddressMode::Zpg),
            0x85 => InstructionType::Alu(AluInstruction::Sta, AddressMode::Zpg),
            0x86 => InstructionType::Rmw(RmwInstruction::Stx, AddressMode::Zpg),
            0x88 => InstructionType::Control(ControlInstruction::Dey, AddressMode::Implied),
            0x8A => InstructionType::Rmw(RmwInstruction::Txa, AddressMode::Implied),
            0x8C => InstructionType::Control(ControlInstruction::Sty, AddressMode::Abs),
            0x8D => InstructionType::Alu(AluInstruction::Sta, AddressMode::Abs),
            0x8E => InstructionType::Rmw(RmwInstruction::Stx, AddressMode::Abs),

            0x90 => InstructionType::Control(ControlInstruction::Bcc, AddressMode::Rel),
            0x91 => InstructionType::Alu(AluInstruction::Sta, AddressMode::IndY),
            0x94 => InstructionType::Control(ControlInstruction::Sty, AddressMode::ZpgX),
            0x95 => InstructionType::Alu(AluInstruction::Sta, AddressMode::ZpgX),
            0x96 => InstructionType::Rmw(RmwInstruction::Stx, AddressMode::ZpgY),
            0x98 => InstructionType::Control(ControlInstruction::Tya, AddressMode::Implied),
            0x99 => InstructionType::Alu(AluInstruction::Sta, AddressMode::AbsY),
            0x9A => InstructionType::Rmw(RmwInstruction::Txs, AddressMode::Implied),
            0x9D => InstructionType::Alu(AluInstruction::Sta, AddressMode::AbsX),

            0xA0 => InstructionType::Control(ControlInstruction::Ldy, AddressMode::Imm),
            0xA1 => InstructionType::Alu(AluInstruction::Lda, AddressMode::IndX),
            0xA2 => InstructionType::Rmw(RmwInstruction::Ldx, AddressMode::Imm),
            0xA4 => InstructionType::Control(ControlInstruction::Ldy, AddressMode::Zpg),
            0xA5 => InstructionType::Alu(AluInstruction::Lda, AddressMode::Zpg),
            0xA6 => InstructionType::Rmw(RmwInstruction::Ldx, AddressMode::Zpg),
            0xA8 => InstructionType::Control(ControlInstruction::Tay, AddressMode::Implied),
            0xA9 => InstructionType::Alu(AluInstruction::Lda, AddressMode::Imm),
            0xAA => InstructionType::Rmw(RmwInstruction::Tax, AddressMode::Implied),
            0xAC => InstructionType::Control(ControlInstruction::Ldy, AddressMode::Abs),
            0xAD => InstructionType::Alu(AluInstruction::Lda, AddressMode::Abs),
            0xAE => InstructionType::Rmw(RmwInstruction::Ldx, AddressMode::Abs),

            0xB0 => InstructionType::Control(ControlInstruction::Bcs, AddressMode::Rel),
            0xB1 => InstructionType::Alu(AluInstruction::Lda, AddressMode::IndY),
            0xB4 => InstructionType::Control(ControlInstruction::Ldy, AddressMode::ZpgX),
            0xB5 => InstructionType::Alu(AluInstruction::Lda, AddressMode::ZpgX),
            0xB6 => InstructionType::Rmw(RmwInstruction::Ldx, AddressMode::ZpgY),
            0xB8 => InstructionType::Control(ControlInstruction::Clv, AddressMode::Implied),
            0xB9 => InstructionType::Alu(AluInstruction::Lda, AddressMode::AbsY),
            0xBA => InstructionType::Rmw(RmwInstruction::Tsx, AddressMode::Implied),
            0xBC => InstructionType::Control(ControlInstruction::Ldy, AddressMode::AbsX),
            0xBD => InstructionType::Alu(AluInstruction::Lda, AddressMode::AbsX),
            0xBE => InstructionType::Rmw(RmwInstruction::Ldx, AddressMode::AbsY),

            0xC0 => InstructionType::Control(ControlInstruction::Cpy, AddressMode::Imm),
            0xC1 => InstructionType::Alu(AluInstruction::Cmp, AddressMode::IndX),
            0xC4 => InstructionType::Control(ControlInstruction::Cpy, AddressMode::Zpg),
            0xC5 => InstructionType::Alu(AluInstruction::Cmp, AddressMode::Zpg),
            0xC6 => InstructionType::Rmw(RmwInstruction::Dec, AddressMode::Zpg),
            0xC8 => InstructionType::Control(ControlInstruction::Iny, AddressMode::Implied),
            0xC9 => InstructionType::Alu(AluInstruction::Cmp, AddressMode::Imm),
            0xCA => InstructionType::Rmw(RmwInstruction::Dex, AddressMode::Implied),
            0xCC => InstructionType::Control(ControlInstruction::Cpy, AddressMode::Abs),
            0xCD => InstructionType::Alu(AluInstruction::Cmp, AddressMode::Abs),
            0xCE => InstructionType::Rmw(RmwInstruction::Dec, AddressMode::Abs),

            0xD0 => InstructionType::Control(ControlInstruction::Bne, AddressMode::Rel),
            0xD1 => InstructionType::Alu(AluInstruction::Cmp, AddressMode::IndY),
            0xD5 => InstructionType::Alu(AluInstruction::Cmp, AddressMode::ZpgX),
            0xD6 => InstructionType::Rmw(RmwInstruction::Dec, AddressMode::ZpgX),
            0xD8 => InstructionType::Control(ControlInstruction::Cld, AddressMode::Implied),
            0xD9 => InstructionType::Alu(AluInstruction::Cmp, AddressMode::AbsY),
            0xDA => InstructionType::Nop(AddressMode::Implied), // Unofficial
            0xDD => InstructionType::Alu(AluInstruction::Cmp, AddressMode::AbsX),
            0xDE => InstructionType::Rmw(RmwInstruction::Dec, AddressMode::AbsX),

            0xE0 => InstructionType::Control(ControlInstruction::Cpx, AddressMode::Imm),
            0xE1 => InstructionType::Alu(AluInstruction::Sbc, AddressMode::IndX),
            0xE4 => InstructionType::Control(ControlInstruction::Cpx, AddressMode::Zpg),
            0xE5 => InstructionType::Alu(AluInstruction::Sbc, AddressMode::Zpg),
            0xE6 => InstructionType::Rmw(RmwInstruction::Inc, AddressMode::Zpg),
            0xE8 => InstructionType::Control(ControlInstruction::Inx, AddressMode::Implied),
            0xE9 => InstructionType::Alu(AluInstruction::Sbc, AddressMode::Imm),
            0xEA => InstructionType::Nop(AddressMode::Implied),
            0xEC => InstructionType::Control(ControlInstruction::Cpx, AddressMode::Abs),
            0xED => InstructionType::Alu(AluInstruction::Sbc, AddressMode::Abs),
            0xEE => InstructionType::Rmw(RmwInstruction::Inc, AddressMode::Abs),

            0xF0 => InstructionType::Control(ControlInstruction::Beq, AddressMode::Rel),
            0xF1 => InstructionType::Alu(AluInstruction::Sbc, AddressMode::IndY),
            0xF5 => InstructionType::Alu(AluInstruction::Sbc, AddressMode::ZpgX),
            0xF6 => InstructionType::Rmw(RmwInstruction::Inc, AddressMode::ZpgX),
            0xF8 => InstructionType::Control(ControlInstruction::Sed, AddressMode::Implied),
            0xF9 => InstructionType::Alu(AluInstruction::Sbc, AddressMode::AbsY),
            0xFA => InstructionType::Nop(AddressMode::Implied), // Unofficial
            0xFD => InstructionType::Alu(AluInstruction::Sbc, AddressMode::AbsX),
            0xFE => InstructionType::Rmw(RmwInstruction::Inc, AddressMode::AbsX),

            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cpu::Operand;

    use super::{control_instructions::run_sei, CpuRegisters, Status};

    #[test]
    fn test_sei() {
        let mut reg = CpuRegisters {
            a: 0,
            x: 0,
            y: 0,
            p: Status { bits: 0 },
            s: 0,
            pc: 0,
        };
        reg.p.insert(Status::CARRY);
        run_sei(&mut reg, Operand::None);
        assert!(reg.p.contains(Status::IT_DISABLE));
        assert!(reg.p.contains(Status::CARRY));
        assert!(!reg.p.contains(Status::ZERO));
    }
}
