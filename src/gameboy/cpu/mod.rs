/*
    to make this truly cycle accurate we must pass in a reference from the ppu to the cpu
    so we can increment the ppu's timer as well

    generally this emulator follows the standard fetch-execute cycle
*/

mod registers;
mod timer;

use crate::gameboy::ppu::PPU;
use crate::gameboy::MMU;

use registers::FLAGS;
use registers::REGISTERS;

use timer::Timer;

// shared ram
use std::cell::RefCell;
use std::rc::Rc;

pub struct CPU {
    //calls the registers
    registers: REGISTERS,

    //checks if the IME is enabled, and if its queued. The EI instruction is only enabled after one cycle
    pub ime: bool,
    ime_queued: bool,

    pub(crate) cycles: u64,

    ram: Rc<RefCell<MMU>>,
    timer: Timer,

    // different cpu states
    pub halted: bool,
    pub stopped: bool,
}
impl CPU {
    pub fn new(ram_: Rc<RefCell<MMU>>) -> Self {
        CPU {
            registers: REGISTERS::new(),
            ime: true,
            ime_queued: false,
            cycles: 0,
            ram: ram_.clone(),
            timer: Timer::new(ram_.clone()),
            halted: false,
            stopped: false,
        }
    }

    // fetches the next instruction
    pub fn fetch(&mut self) -> u8 {
        let pc = self.registers.get_pc();
        let value = self.ram.borrow_mut().read(pc);
        self.registers.set_pc(pc.wrapping_add(1));
        value
    }

    // used to handle system interrupts
    pub fn handle_interrupt(&mut self, ppu: &mut PPU) {
        // Enable IME if queued
        if self.ime_queued {
            self.ime = true;
            self.ime_queued = false;
        }

        // reads and checks if interrupts are enabled
        let interrupt_enable = self.ram.borrow_mut().read(0xFFFF);
        let interrupt_flags = self.ram.borrow_mut().read(0xFF0F);
        let pending = interrupt_enable & interrupt_flags;

        // if there are no pending interrupts return as there is nothing to do
        if pending == 0 {
            return;
        }

        // halt-bug ime = false + pending exit HALT but the interrupt is not serviced
        if !self.ime && self.halted {
            self.halt_bug(ppu);
            return;
        }

        // stops halt as there is an interrupt pending
        self.halted = false;

        // if ime is disabled no interrupts will be serviced
        if self.ime {
            for &(bit, vector) in &[
                (0, 0x0040), // V-Blank
                (1, 0x0048), // LCD STAT
                (2, 0x0050), // Timer
                (3, 0x0058), // Serial
                (4, 0x0060), // Joypad
            ] {
                if (pending & (1 << bit)) != 0 {
                    self.service_interrupt(bit, vector, ppu);
                    break;
                }
            }
        }
    }

    fn service_interrupt(&mut self, bit: u8, vector: u16, ppu: &mut PPU) {
        self.halted = false;
        self.ime = false;

        self.nop(ppu); // Internal delay

        // Push PC to stack
        self.push_pc(ppu);

        self.nop(ppu);

        // Clear interrupt flag
        let mut interrupt_flag = self.ram.borrow_mut().read(0xFF0F);
        interrupt_flag &= !(1 << bit);
        self.ram.borrow_mut().write(0xFF0F, interrupt_flag);

        // Jump to interrupt vector
        self.registers.set_pc(vector);

        self.nop(ppu);
    }

    // the next opcode is retrieved but the cycles are not added
    fn halt_bug(&mut self, ppu: &mut PPU) {
        self.halted = false;
        let pc = self.registers.get_pc();
        let opcode = self.ram.borrow_mut().read(pc);
        self.execute(opcode, ppu);
    }

    fn push_pc(&mut self, ppu: &mut PPU) {
        let value = self.registers.get_pc();

        self.nop(ppu);
        self.registers
            .set_sp(self.registers.get_sp().wrapping_sub(1));
        self.ram
            .borrow_mut()
            .write(self.registers.get_sp(), ((value & 0xFF00) >> 8) as u8);

        self.nop(ppu);
        self.registers
            .set_sp(self.registers.get_sp().wrapping_sub(1));
        self.ram
            .borrow_mut()
            .write(self.registers.get_sp(), (value & 0xFF) as u8);
    }

    pub fn tick(&mut self, ppu: &mut PPU) {
        self.timer.timer(4);
        ppu.step();

        if self.ram.borrow_mut().oam_dma {
            self.ram.borrow_mut().oam_dma_transfer();
            return;
        }
    }
    // the default do nothing instruction
    pub fn nop(&mut self, ppu: &mut PPU) {
        self.cycles += 4;
        self.tick(ppu);
    }
    // returns a 16 bit value from the specified address
    fn load_16(&mut self, address: u16, ppu: &mut PPU) -> u8 {
        let value = self.ram.borrow_mut().read(address);
        self.nop(ppu);
        value
    }
    // returns the value of a
    fn load_a(&mut self, value: u8, ppu: &mut PPU) {
        self.nop(ppu);
        self.registers.set_a(value);
    }
    // returns the value of b
    fn load_b(&mut self, data: u8, ppu: &mut PPU) {
        self.registers.set_b(data);
        self.nop(ppu);
    }
    fn load_c(&mut self, data: u8, ppu: &mut PPU) {
        self.registers.set_c(data);
        self.nop(ppu);
    }
    fn load_d(&mut self, data: u8, ppu: &mut PPU) {
        self.registers.set_d(data);
        self.nop(ppu);
    }
    fn load_e(&mut self, data: u8, ppu: &mut PPU) {
        self.registers.set_e(data);
        self.nop(ppu);
    }
    fn load_h(&mut self, data: u8, ppu: &mut PPU) {
        self.registers.set_h(data);
        self.nop(ppu);
    }
    fn load_l(&mut self, data: u8, ppu: &mut PPU) {
        self.registers.set_l(data);
        self.nop(ppu);
    }
    // writes to ram
    fn store(&mut self, address: u16, data: u8, ppu: &mut PPU) {
        self.ram.borrow_mut().write(address, data);
        self.nop(ppu);
    }

    // Decimal Adjust Accumulator
    fn daa(&mut self, ppu: &mut PPU) {
        let a = self.registers.get_a();
        let mut f = self.registers.get_f();

        // gets the current state of each flag
        let n = (f & FLAGS::N as u8) != 0;
        let c = (f & FLAGS::C as u8) != 0;
        let h = (f & FLAGS::H as u8) != 0;

        let mut result = a;

        if !n {
            if c || a > 0x99 {
                result = result.wrapping_add(0x60);
                f |= FLAGS::C as u8; // Set carry if adjustment applied
            }
            if h || (a & 0x0F) > 0x09 {
                result = result.wrapping_add(0x06);
            }
        } else {
            if c {
                result = result.wrapping_sub(0x60);
            }
            if h {
                result = result.wrapping_sub(0x06);
            }
        }

        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        f &= !(FLAGS::H as u8);

        self.registers.set_a(result);
        self.registers.set_f(f);
        self.nop(ppu);
    }

    fn increment_bc(&mut self, ppu: &mut PPU) {
        let bc = self.registers.get_bc();
        self.nop(ppu);
        self.registers.set_bc(bc.wrapping_add(1));
        self.nop(ppu);
    }
    fn increment_de(&mut self, ppu: &mut PPU) {
        let de = self.registers.get_de();
        self.nop(ppu);
        self.registers.set_de(de.wrapping_add(1));
        self.nop(ppu);
    }
    fn increment_hl(&mut self, ppu: &mut PPU) {
        let hl = self.registers.get_hl();
        self.nop(ppu);
        self.registers.set_hl(hl.wrapping_add(1));
        self.nop(ppu);
    }
    fn increment_sp(&mut self, ppu: &mut PPU) {
        let sp = self.registers.get_sp();
        self.nop(ppu);
        self.registers.set_sp(sp.wrapping_add(1));
        self.nop(ppu);
    }

    // rotate reg a to the left
    fn rlca(&mut self, ppu: &mut PPU) {
        let a = self.registers.get_a();
        let carry = (a & 0x80) != 0;

        let result = a.rotate_left(1);

        self.registers.set_a(result);

        let mut new_flags: u8 = 0;
        if carry {
            new_flags |= FLAGS::C as u8;
        }
        self.registers.set_f(new_flags);

        self.nop(ppu);
    }
    //adds 8 bit numbers
    fn add_8bit(&mut self, b: u8, ppu: &mut PPU) {
        let a = self.registers.get_a();
        let (result, carry) = a.overflowing_add(b); // Separate carry result

        let mut f = self.registers.get_f()
            & !(FLAGS::Z as u8 | FLAGS::N as u8 | FLAGS::H as u8 | FLAGS::C as u8);

        self.registers.set_a(result);
        // Z flag
        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        // H flag
        if (a & 0x0F) + (b & 0x0F) > 0x0F {
            f |= FLAGS::H as u8;
        } else {
            f &= !(FLAGS::H as u8);
        }

        if carry {
            f |= FLAGS::C as u8;
        } else {
            f &= !(FLAGS::C as u8);
        }

        self.registers.set_f(f);
        self.nop(ppu);
    }

    // adds two 16 bit numbers as well as setting their flags
    fn add_16bit(&mut self, value1: u16, value2: u16, ppu: &mut PPU) -> u16 {
        let result = value1.wrapping_add(value2);

        let mut f = self.registers.get_f();

        //reset the N flag
        f &= !(FLAGS::N as u8);

        //setting the h fl
        if (value1 & 0xFFF) + (value2 & 0xFFF) > 0xFFF {
            f |= FLAGS::H as u8
        } else {
            f &= !(FLAGS::H as u8)
        }

        if value1 > 0xFFFF - value2 {
            f |= FLAGS::C as u8; // Carry flag
        } else {
            f &= !(FLAGS::C as u8);
        }

        self.registers.set_f(f);
        self.nop(ppu);
        result
    }

    fn add_with_carry(&mut self, b: u8, ppu: &mut PPU) {
        let a = self.registers.get_a();
        let carry = (self.registers.get_f() & FLAGS::C as u8) >> 4;
        let (result, carry1) = a.overflowing_add(b);
        let (result, carry2) = result.overflowing_add(carry);

        let mut f = self.registers.get_f()
            & !(FLAGS::Z as u8 | FLAGS::N as u8 | FLAGS::H as u8 | FLAGS::C as u8);

        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        if (a & 0x0F) + (b & 0x0F) + carry > 0x0F {
            f |= FLAGS::H as u8;
        } else {
            f &= !(FLAGS::H as u8);
        }

        if carry1 || carry2 {
            f |= FLAGS::C as u8;
        } else {
            f &= !(FLAGS::C as u8);
        }
        self.registers.set_a(result);

        self.registers.set_f(f);
        self.nop(ppu);
    }

    // increments a 8 bit number
    fn increment_8_bit(&mut self, data: u8, ppu: &mut PPU) -> u8 {
        let result = data.wrapping_add(1);

        let mut f = self.registers.get_f();

        //setting Z flag
        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        //setting N flag

        f &= !(FLAGS::N as u8);

        //setting H flag

        if (data & 0x0F) + 1 > 0x0F {
            f |= FLAGS::H as u8;
        } else {
            f &= !(FLAGS::H as u8);
        }

        self.registers.set_f(f);
        self.nop(ppu);
        result
    }

    // decrements an 8 bit number
    fn decrement_8_bit(&mut self, data: u8, ppu: &mut PPU) -> u8 {
        let result = data.wrapping_sub(1);
        let mut f = self.registers.get_f();

        //setting Z flag
        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        //setting N flag

        f |= FLAGS::N as u8;

        //setting H flag

        if (data & 0x0F) == 0 {
            f |= FLAGS::H as u8;
        } else {
            f &= !(FLAGS::H as u8);
        }

        self.registers.set_f(f & 0xF0);
        self.nop(ppu);
        result
    }

    // zeros a bit 1->0
    fn zero_bit_8bit(&mut self, value: u8, bit: u8, ppu: &mut PPU) -> u8 {
        self.nop(ppu);
        value & !(1 << bit)
    }

    // sets a bit 0->1
    fn set_bit_8bit(&mut self, value: u8, bit: u8, ppu: &mut PPU) -> u8 {
        self.nop(ppu);
        value | (1 << bit)
    }

    // subtracts an 8 bit number
    fn sub_8bit(&mut self, b: u8, ppu: &mut PPU) {
        let a = self.registers.get_a();
        let (result, borrow) = a.overflowing_sub(b);

        let mut f = self.registers.get_f() & !(FLAGS::Z as u8 | FLAGS::H as u8 | FLAGS::C as u8);
        f |= FLAGS::N as u8;

        if result == 0 {
            f |= FLAGS::Z as u8;
        }

        if (a & 0x0F).wrapping_sub(b & 0x0F) & 0x10 != 0 {
            f |= FLAGS::H as u8;
        }

        if borrow {
            f |= FLAGS::C as u8;
        }

        self.registers.set_a(result);
        self.registers.set_f(f);
        self.nop(ppu);
    }

    // subtracts with a carry
    fn sub_with_carry(&mut self, b: u8, ppu: &mut PPU) {
        let a = self.registers.get_a();
        let carry = (self.registers.get_f() & FLAGS::C as u8) >> 4; // Extract carry flag

        let (temp_result, borrow1) = a.overflowing_sub(b);
        let (result, borrow2) = temp_result.overflowing_sub(carry); // Subtract carry

        self.registers.set_a(result);

        let mut f = self.registers.get_f() | FLAGS::N as u8;

        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8)
        }

        if (a & 0x0F).wrapping_sub(b & 0x0F).wrapping_sub(carry) > 0x0F {
            f |= FLAGS::H as u8;
        } else {
            f &= !(FLAGS::H as u8)
        }

        if borrow1 || borrow2 {
            f |= FLAGS::C as u8;
        } else {
            f &= !(FLAGS::C as u8)
        }

        self.registers.set_f(f);
        self.nop(ppu);
    }

    // bitwise and
    fn and(&mut self, b: u8, ppu: &mut PPU) {
        let a = self.registers.get_a();
        let result = a & b;
        self.registers.set_a(result);

        let mut f = self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::C as u8);
        f |= FLAGS::H as u8;

        if result == 0 {
            f |= FLAGS::Z as u8
        } else {
            f &= !(FLAGS::Z as u8)
        }

        self.registers.set_f(f);
        self.nop(ppu);
    }

    // bitwise exclusive or
    fn xor(&mut self, b: u8, ppu: &mut PPU) {
        let a = self.registers.get_a();
        let result = a ^ b;
        self.registers.set_a(result);

        let mut f = self.registers.get_f() & !(FLAGS::C as u8 | FLAGS::N as u8 | FLAGS::H as u8);

        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        self.registers.set_f(f);
        self.nop(ppu);
    }

    // bitwise or
    fn or(&mut self, b: u8, ppu: &mut PPU) {
        let a = self.registers.get_a();
        let result = a | b;
        self.registers.set_a(result);

        let mut f = self.registers.get_f() & !(FLAGS::C as u8 | FLAGS::N as u8 | FLAGS::H as u8);

        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        self.registers.set_f(f);
        self.nop(ppu);
    }

    // compares two numbers and sets it accordingly
    fn compare(&mut self, n8: u8, ppu: &mut PPU) {
        let a = self.registers.get_a();
        let result = a.wrapping_sub(n8);

        let mut f = self.registers.get_f() | FLAGS::N as u8;

        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        if (a & 0x0F) < (n8 & 0x0F) {
            f |= FLAGS::H as u8;
        } else {
            f &= !(FLAGS::H as u8);
        }

        if a < n8 {
            f |= FLAGS::C as u8;
        } else {
            f &= !(FLAGS::C as u8);
        }

        self.registers.set_f(f);
        self.nop(ppu);
    }

    //resets pc
    fn reset(&mut self, address: u16, ppu: &mut PPU) {
        self.registers
            .set_sp(self.registers.get_sp().wrapping_sub(1));
        self.nop(ppu);
        self.ram.borrow_mut().write(
            self.registers.get_sp(),
            (self.registers.get_pc() >> 8) as u8,
        );
        self.nop(ppu);
        self.registers
            .set_sp(self.registers.get_sp().wrapping_sub(1));
        self.nop(ppu);
        self.ram.borrow_mut().write(
            self.registers.get_sp(),
            (self.registers.get_pc() & 0xFF) as u8,
        );

        self.registers.set_pc(address);
        self.nop(ppu);
    }

    fn rotate_without_carry(&mut self, mut value: u8, type_: u8, ppu: &mut PPU) -> u8 {
        self.nop(ppu);
        let bool;
        if type_ == 0 {
            bool = (value & 0x80) != 0;
            value = value.rotate_left(1);
        } else {
            bool = (value & 0x01) != 0;
            value = value.rotate_right(1);
        }

        let mut f: u8 = self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8);

        if value == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8)
        }

        if bool {
            f |= FLAGS::C as u8;
        } else {
            f &= !(FLAGS::C as u8)
        }

        self.registers.set_f(f);
        value
    }

    // rotates a value
    fn rotate(&mut self, mut value: u8, type_: u8, ppu: &mut PPU) -> u8 {
        self.nop(ppu);
        let bool;
        let carry = (self.registers.get_f() & FLAGS::C as u8) != 0;
        if type_ == 0 {
            bool = (value & 0x80) != 0;
            value = (value << 1) | (carry as u8);
        } else {
            bool = (value & 0x01) != 0;
            value = (value >> 1) | ((carry as u8) << 7);
        }

        let mut f = self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8);

        if value == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8)
        }

        if bool {
            f |= FLAGS::C as u8;
        } else {
            f &= !(FLAGS::C as u8)
        }

        self.registers.set_f(f);

        value
    }

    // left shift or right shift
    fn shift(&mut self, mut value: u8, type_: u8, ppu: &mut PPU) -> u8 {
        // Read the old flags, but we’re going to build the new flags from scratch.
        self.nop(ppu);
        let mut f: u8 = 0;

        if type_ == 0 {
            let msb = (value & 0x80) != 0;
            value <<= 1;
            if msb {
                f |= FLAGS::C as u8;
            }
        } else {
            let lsb = (value & 0x01) != 0;
            let msb = value & 0x80;
            value = (value >> 1) | msb;
            if lsb {
                f |= FLAGS::C as u8;
            }
        }

        if value == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        self.registers.set_f(f);

        value
    }

    // swaps the upper nibble and the lower nibble
    fn swap(&mut self, value: u8, ppu: &mut PPU) -> u8 {
        self.nop(ppu);
        let result = (value >> 4) | (value << 4);
        let mut f = self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8 | FLAGS::C as u8);

        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }
        self.registers.set_f(f);
        result
    }

    // special right shift
    fn right_shift(&mut self, mut value: u8, ppu: &mut PPU) -> u8 {
        self.nop(ppu);

        let lsb = (value & 0x01) != 0;
        value >>= 1;

        let mut f = self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8);

        if value == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        if lsb {
            f |= FLAGS::C as u8;
        } else {
            f &= !(FLAGS::C as u8);
        }

        self.registers.set_f(f);

        value
    }

    //sets the bit
    fn bit(&mut self, value: u8, bit: u8, ppu: &mut PPU) {
        self.nop(ppu);
        self.nop(ppu);
        let tested_bit = value & (1 << bit);

        let mut f = self.registers.get_f() | FLAGS::H as u8;
        f &= !(FLAGS::N as u8);

        if tested_bit == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        self.registers.set_f(f);
    }

    // the decode -> execute function
    pub fn execute(&mut self, opcode: u8, ppu: &mut PPU) {
        match opcode {
            0x00 => {
                //NOP
                self.nop(ppu);
            }
            0x01 => {
                //Load 2 bytes into register BC
                let mut value = self.registers.get_and_inc_pc();
                let lower_byte = self.load_16(value, ppu);

                value = self.registers.get_and_inc_pc();
                let upper_byte = self.load_16(value, ppu);

                let data = ((upper_byte as u16) << 8) | (lower_byte as u16);
                self.nop(ppu);
                self.registers.set_bc(data);
            }
            0x02 => {
                //store the data in a into the ram address found in bc
                self.store(self.registers.get_bc(), self.registers.get_a(), ppu);
                self.nop(ppu);
            }
            0x03 => {
                //increment bc
                self.increment_bc(ppu);
            }
            0x04 => {
                //increment b
                let data = self.increment_8_bit(self.registers.get_b(), ppu);
                self.registers.set_b(data);
            }
            0x05 => {
                //decrement b
                let data = self.decrement_8_bit(self.registers.get_b(), ppu);
                self.registers.set_b(data);
            }
            0x06 => {
                //load 1 byte into B
                self.nop(ppu);
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.load_b(data, ppu);
            }
            0x07 => {
                self.rlca(ppu);
            }
            0x08 => {
                //load sp into the address from ram
                let mut address = self.registers.get_and_inc_pc();
                let lower_byte = self.load_16(address, ppu);

                address = self.registers.get_and_inc_pc();
                let upper_byte = self.load_16(address, ppu);

                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                let sp = self.registers.get_sp();
                self.nop(ppu);

                self.ram.borrow_mut().write(address, (sp & 0x00FF) as u8);
                self.nop(ppu);

                self.ram.borrow_mut().write(address + 1, (sp >> 8) as u8);
                self.nop(ppu);
            }
            0x09 => {
                self.nop(ppu);
                let hl = self.registers.get_hl();
                let bc = self.registers.get_bc();
                let result = self.add_16bit(hl, bc, ppu);
                self.registers.set_hl(result);
            }
            0x0A => {
                let data = self.ram.borrow_mut().read(self.registers.get_bc());
                self.load_a(data, ppu);
                self.nop(ppu);
            }
            0x0B => {
                //decrement BC
                self.nop(ppu);
                self.registers
                    .set_bc(self.registers.get_bc().wrapping_sub(1));
                self.nop(ppu);
            }
            0x0C => {
                let data = self.increment_8_bit(self.registers.get_c(), ppu);
                self.registers.set_c(data);
            }
            0x0D => {
                let data = self.decrement_8_bit(self.registers.get_c(), ppu);
                self.registers.set_c(data);
            }
            0x0E => {
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                self.load_c(data, ppu);
            }
            0x0F => {
                let a = self.registers.get_a();
                let carry = a & 0b0000_0001;
                let result = (a >> 1) | (carry << 7);
                self.registers.set_a(result);

                //Reset flags Z, N , H
                self.registers.set_f(
                    self.registers.get_f() & !(FLAGS::Z as u8 | FLAGS::N as u8 | FLAGS::H as u8),
                );
                if carry != 0 {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::C as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::C as u8));
                }

                self.nop(ppu);
            }
            0x10 => {
                self.stopped = true;
                self.nop(ppu);
            }
            0x11 => {
                let mut value = self.registers.get_and_inc_pc();
                let lower_byte = self.load_16(value, ppu);

                value = self.registers.get_and_inc_pc();
                let upper_byte = self.load_16(value, ppu);

                let data = ((upper_byte as u16) << 8) | (lower_byte as u16);
                self.nop(ppu);
                self.registers.set_de(data);
            }
            0x12 => {
                self.store(self.registers.get_de(), self.registers.get_a(), ppu);
                self.nop(ppu);
            }
            0x13 => {
                self.increment_de(ppu);
            }
            0x14 => {
                let data = self.increment_8_bit(self.registers.get_d(), ppu);
                self.registers.set_d(data);
            }
            0x15 => {
                let data = self.decrement_8_bit(self.registers.get_d(), ppu);
                self.registers.set_d(data);
            }
            0x16 => {
                self.nop(ppu);
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.load_d(data, ppu);
            }
            0x17 => {
                let a = self.registers.get_a();
                let carry = (self.registers.get_f() & FLAGS::C as u8) >> 4;
                let new_carry = (a & FLAGS::Z as u8) >> 7;
                let result = (a << 1) | carry;

                self.registers.set_a(result);

                self.registers.set_f(
                    self.registers.get_f() & !(FLAGS::Z as u8 | FLAGS::N as u8 | FLAGS::H as u8),
                );

                // Update C flag
                if new_carry != 0 {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::C as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::C as u8));
                }
                self.nop(ppu);
            }
            0x18 => {
                //relative jump
                let jump = self.ram.borrow_mut().read(self.registers.get_and_inc_pc()) as i8;
                self.nop(ppu);
                let new_pc = (self.registers.get_pc() as i16).wrapping_add(jump as i16) as u16;
                self.nop(ppu);
                self.registers.set_pc(new_pc);
                self.nop(ppu);
            }
            0x19 => {
                self.nop(ppu);
                let hl = self.registers.get_hl();
                let de = self.registers.get_de();
                let result = self.add_16bit(hl, de, ppu);
                self.registers.set_hl(result);
            }
            0x1A => {
                let data = self.ram.borrow_mut().read(self.registers.get_de());
                self.load_a(data, ppu);
                self.nop(ppu);
            }
            0x1B => {
                //decrement de
                self.nop(ppu);
                self.registers
                    .set_de(self.registers.get_de().wrapping_sub(1));
                self.nop(ppu);
            }
            0x1C => {
                let data = self.increment_8_bit(self.registers.get_e(), ppu);
                self.registers.set_e(data);
            }
            0x1d => {
                let data = self.decrement_8_bit(self.registers.get_e(), ppu);
                self.registers.set_e(data);
            }
            0x1E => {
                //load the next byte onto register E
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                self.load_e(data, ppu);
            }
            0x1F => {
                let a = self.registers.get_a();
                let carry = (self.registers.get_f() & FLAGS::C as u8) >> 4;
                let new_carry = a & 0b0000_0001;
                let result = (a >> 1) | (carry << 7);

                self.registers.set_a(result);

                // Reset Z, N, H flags
                self.registers.set_f(
                    self.registers.get_f() & !(FLAGS::Z as u8 | FLAGS::N as u8 | FLAGS::H as u8),
                );

                if new_carry != 0 {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::C as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::C as u8));
                }
                self.nop(ppu);
            }
            0x20 => {
                let jump = self.ram.borrow_mut().read(self.registers.get_and_inc_pc()) as i8;
                self.nop(ppu);
                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    self.nop(ppu);
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(jump as i16 as u16));
                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0x21 => {
                let mut value = self.registers.get_and_inc_pc();
                let lower_byte = self.load_16(value, ppu);

                value = self.registers.get_and_inc_pc();
                let upper_byte = self.load_16(value, ppu);

                let data = ((upper_byte as u16) << 8) | (lower_byte as u16);
                self.nop(ppu);
                self.registers.set_hl(data);
            }
            0x22 => {
                //load a into memory with the address found in HL and increment HL by 1
                let hl = self.registers.get_hl();
                self.store(hl, self.registers.get_a(), ppu);
                self.registers.set_hl(hl.wrapping_add(1));
                self.nop(ppu);
            }
            0x23 => {
                self.increment_hl(ppu);
            }
            0x24 => {
                let data = self.increment_8_bit(self.registers.get_h(), ppu);
                self.registers.set_h(data);
            }
            0x25 => {
                let data = self.decrement_8_bit(self.registers.get_h(), ppu);
                self.registers.set_h(data);
            }
            0x26 => {
                self.nop(ppu);
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.load_h(data, ppu);
            }
            0x27 => {
                self.daa(ppu);
            }

            0x28 => {
                let offset = self.ram.borrow_mut().read(self.registers.get_and_inc_pc()) as i8;
                self.nop(ppu);
                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    self.nop(ppu);
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(offset as i16 as u16));
                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0x29 => {
                self.nop(ppu);
                let hl = self.registers.get_hl();
                let result = self.add_16bit(hl, hl, ppu);
                self.registers.set_hl(result);
            }
            0x2A => {
                let address = self.registers.get_hl();
                let data = self.ram.borrow_mut().read(address);
                self.registers.set_hl(address.wrapping_add(1));
                self.load_a(data, ppu);
                self.nop(ppu);
            }
            0x2B => {
                self.nop(ppu);
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_sub(1));
                self.nop(ppu);
            }
            0x2C => {
                let data = self.increment_8_bit(self.registers.get_l(), ppu);
                self.registers.set_l(data);
            }
            0x2D => {
                let data = self.decrement_8_bit(self.registers.get_l(), ppu);
                self.registers.set_l(data);
            }
            0x2E => {
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                self.load_l(data, ppu);
            }
            0x2F => {
                let a = self.registers.get_a();
                self.registers.set_a(!a); // Bitwise complement

                // Set N and H flags
                self.registers
                    .set_f(self.registers.get_f() | FLAGS::N as u8 | FLAGS::H as u8);

                self.nop(ppu);
            }
            0x30 => {
                let offset = self.ram.borrow_mut().read(self.registers.get_and_inc_pc()) as i8;
                self.nop(ppu);
                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.nop(ppu);
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(offset as i16 as u16));
                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0x31 => {
                let mut value = self.registers.get_and_inc_pc();
                let lower_byte = self.load_16(value, ppu);

                value = self.registers.get_and_inc_pc();
                let upper_byte = self.load_16(value, ppu);

                let data = ((upper_byte as u16) << 8) | (lower_byte as u16);
                self.nop(ppu);
                self.registers.set_sp(data);
            }
            0x32 => {
                let hl = self.registers.get_hl();
                self.store(hl, self.registers.get_a(), ppu);
                self.registers.set_hl(hl.wrapping_sub(1));
                self.nop(ppu);
            }
            0x33 => {
                self.increment_sp(ppu);
            }
            0x34 => {
                let address = self.registers.get_hl();
                let value = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                let result = value.wrapping_add(1);
                self.ram.borrow_mut().write(address, result);
                self.nop(ppu);

                if result == 0 {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::Z as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::Z as u8));
                }

                self.registers
                    .set_f(self.registers.get_f() & !(FLAGS::N as u8));

                if (value & 0x0F) + 1 > 0x0F {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::H as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::H as u8));
                }

                self.nop(ppu);
            }
            0x35 => {
                let address = self.registers.get_hl();
                let value = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                let result = value.wrapping_sub(1);
                self.ram.borrow_mut().write(address, result);
                self.nop(ppu);

                //Z
                if result == 0 {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::Z as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::Z as u8));
                }

                //N
                self.registers
                    .set_f(self.registers.get_f() | (FLAGS::N as u8));

                //H
                if (value & 0x0F) == 0 {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::H as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::H as u8));
                }
                self.nop(ppu);
            }
            0x36 => {
                let address = self.registers.get_hl();
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                self.ram.borrow_mut().write(address, data);
                self.nop(ppu);
                self.nop(ppu);
            }
            0x37 => {
                self.registers
                    .set_f(self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8));

                self.registers
                    .set_f(self.registers.get_f() | FLAGS::C as u8);

                self.nop(ppu);
            }
            0x38 => {
                let offset = self.ram.borrow_mut().read(self.registers.get_and_inc_pc()) as i8;
                self.nop(ppu);
                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    self.nop(ppu);
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(offset as i16 as u16));
                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0x39 => {
                let hl = self.registers.get_hl();
                let sp = self.registers.get_sp();
                let result = hl.wrapping_add(sp);
                self.registers.set_hl(result);
                self.nop(ppu);

                // Preserve the current Z flag.
                let z_flag = self.registers.get_f() & (FLAGS::Z as u8);
                // For ADD HL,SP: Clear N; start with Z preserved.
                let mut f = z_flag;

                // Set H flag if a carry occurs from bit 11 (lower 12 bits)
                if (hl & 0x0FFF) + (sp & 0x0FFF) > 0x0FFF {
                    f |= FLAGS::H as u8;
                }

                if hl > 0xFFFF - sp {
                    f |= FLAGS::C as u8;
                }

                self.registers.set_f(f);

                self.nop(ppu);
            }
            0x3A => {
                let data = self.ram.borrow_mut().read(self.registers.get_hl());
                self.nop(ppu);
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_sub(1));

                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x3B => {
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop(ppu);
            }
            0x3C => {
                let data = self.increment_8_bit(self.registers.get_a(), ppu);
                self.registers.set_a(data);
            }
            0x3D => {
                let data = self.decrement_8_bit(self.registers.get_a(), ppu);
                self.registers.set_a(data);
            }
            0x3E => {
                let value = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.load_a(value, ppu);
                self.nop(ppu);
            }
            0x3F => {
                self.registers
                    .set_f(self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8));

                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::C as u8)); // Clear Carry
                } else {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::C as u8); // Set Carry
                }
                self.nop(ppu);
            }
            0x40 => {
                self.nop(ppu);
            }
            0x41 => {
                self.registers.set_b(self.registers.get_c());
                self.nop(ppu);
            }
            0x42 => {
                self.registers.set_b(self.registers.get_d());
                self.nop(ppu);
            }
            0x43 => {
                self.registers.set_b(self.registers.get_e());
                self.nop(ppu);
            }
            0x44 => {
                self.registers.set_b(self.registers.get_h());
                self.nop(ppu);
            }
            0x45 => {
                self.registers.set_b(self.registers.get_l());
                self.nop(ppu);
            }
            0x46 => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.nop(ppu);
                self.registers.set_b(value);
                self.nop(ppu);
            }
            0x47 => {
                self.registers.set_b(self.registers.get_a());
                self.nop(ppu);
            }
            0x48 => {
                self.registers.set_c(self.registers.get_b());
                self.nop(ppu);
            }
            0x49 => {
                self.nop(ppu);
            }
            0x4A => {
                self.registers.set_c(self.registers.get_d());
                self.nop(ppu);
            }
            0x4B => {
                self.registers.set_c(self.registers.get_e());
                self.nop(ppu);
            }
            0x4C => {
                self.registers.set_c(self.registers.get_h());
                self.nop(ppu);
            }
            0x4D => {
                self.registers.set_c(self.registers.get_l());
                self.nop(ppu);
            }
            0x4E => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.nop(ppu);
                self.registers.set_c(value);
                self.nop(ppu);
            }
            0x4F => {
                self.registers.set_c(self.registers.get_a());
                self.nop(ppu);
            }
            0x50 => {
                self.registers.set_d(self.registers.get_b());
                self.nop(ppu);
            }
            0x51 => {
                self.registers.set_d(self.registers.get_c());
                self.nop(ppu);
            }
            0x52 => {
                self.nop(ppu);
            }
            0x53 => {
                self.registers.set_d(self.registers.get_e());
                self.nop(ppu);
            }
            0x54 => {
                self.registers.set_d(self.registers.get_h());
                self.nop(ppu);
            }
            0x55 => {
                self.registers.set_d(self.registers.get_l());
                self.nop(ppu);
            }
            0x56 => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.nop(ppu);
                self.registers.set_d(value);
                self.nop(ppu);
            }
            0x57 => {
                self.registers.set_d(self.registers.get_a());
                self.nop(ppu);
            }
            0x58 => {
                self.registers.set_e(self.registers.get_b());
                self.nop(ppu);
            }
            0x59 => {
                self.registers.set_e(self.registers.get_c());
                self.nop(ppu);
            }
            0x5A => {
                self.registers.set_e(self.registers.get_d());
                self.nop(ppu);
            }
            0x5B => {
                self.nop(ppu);
            }
            0x5C => {
                self.registers.set_e(self.registers.get_h());
                self.nop(ppu);
            }
            0x5D => {
                self.registers.set_e(self.registers.get_l());
                self.nop(ppu);
            }
            0x5E => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.nop(ppu);
                self.registers.set_e(value);
                self.nop(ppu);
            }
            0x5F => {
                self.registers.set_e(self.registers.get_a());
                self.nop(ppu);
            }
            0x60 => {
                self.registers.set_h(self.registers.get_b());
                self.nop(ppu);
            }
            0x61 => {
                self.registers.set_h(self.registers.get_c());
                self.nop(ppu);
            }
            0x62 => {
                self.registers.set_h(self.registers.get_d());
                self.nop(ppu);
            }
            0x63 => {
                self.registers.set_h(self.registers.get_e());
                self.nop(ppu);
            }
            0x64 => {
                self.nop(ppu);
            }
            0x65 => {
                self.registers.set_h(self.registers.get_l());
                self.nop(ppu);
            }
            0x66 => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.nop(ppu);
                self.registers.set_h(value);
                self.nop(ppu);
            }
            0x67 => {
                self.registers.set_h(self.registers.get_a());
                self.nop(ppu);
            }
            0x68 => {
                self.registers.set_l(self.registers.get_b());
                self.nop(ppu);
            }
            0x69 => {
                self.registers.set_l(self.registers.get_c());
                self.nop(ppu);
            }
            0x6A => {
                self.registers.set_l(self.registers.get_d());
                self.nop(ppu);
            }
            0x6B => {
                self.registers.set_l(self.registers.get_e());
                self.nop(ppu);
            }
            0x6C => {
                self.registers.set_l(self.registers.get_h());
                self.nop(ppu);
            }
            0x6D => {
                self.nop(ppu);
            }
            0x6E => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.nop(ppu);
                self.registers.set_l(value);
                self.nop(ppu);
            }
            0x6F => {
                self.registers.set_l(self.registers.get_a());
                self.nop(ppu);
            }
            0x70 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_b());
                self.nop(ppu);
                self.nop(ppu);
            }
            0x71 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_c());
                self.nop(ppu);
                self.nop(ppu);
            }
            0x72 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_d());
                self.nop(ppu);
                self.nop(ppu);
            }
            0x73 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_e());
                self.nop(ppu);
                self.nop(ppu);
            }
            0x74 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_h());
                self.nop(ppu);
                self.nop(ppu);
            }
            0x75 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_l());
                self.nop(ppu);
                self.nop(ppu);
            }
            0x76 => {
                self.halted = true;
                self.nop(ppu);
            }
            0x77 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_a());
                self.nop(ppu);
                self.nop(ppu);
            }
            0x78 => {
                self.registers.set_a(self.registers.get_b());
                self.nop(ppu);
            }
            0x79 => {
                self.registers.set_a(self.registers.get_c());
                self.nop(ppu);
            }
            0x7A => {
                self.registers.set_a(self.registers.get_d());
                self.nop(ppu);
            }
            0x7B => {
                self.registers.set_a(self.registers.get_e());
                self.nop(ppu);
            }
            0x7C => {
                self.registers.set_a(self.registers.get_h());
                self.nop(ppu);
            }
            0x7D => {
                self.registers.set_a(self.registers.get_l());
                self.nop(ppu);
            }
            0x7E => {
                let address = self.registers.get_hl();
                let value = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                self.registers.set_a(value);
                self.nop(ppu);
            }
            0x7F => {
                self.nop(ppu);
            }
            0x80 => {
                self.add_8bit(self.registers.get_b(), ppu);
            }
            0x81 => {
                self.add_8bit(self.registers.get_c(), ppu);
            }
            0x82 => {
                self.add_8bit(self.registers.get_d(), ppu);
            }
            0x83 => {
                self.add_8bit(self.registers.get_e(), ppu);
            }
            0x84 => {
                self.add_8bit(self.registers.get_h(), ppu);
            }
            0x85 => {
                self.add_8bit(self.registers.get_l(), ppu);
            }
            0x86 => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.add_8bit(value, ppu);
                self.nop(ppu);
            }
            0x87 => {
                self.add_8bit(self.registers.get_a(), ppu);
            }
            0x88 => {
                self.add_with_carry(self.registers.get_b(), ppu);
            }
            0x89 => {
                self.add_with_carry(self.registers.get_c(), ppu);
            }
            0x8A => {
                self.add_with_carry(self.registers.get_d(), ppu);
            }
            0x8B => {
                self.add_with_carry(self.registers.get_e(), ppu);
            }
            0x8C => {
                self.add_with_carry(self.registers.get_h(), ppu);
            }
            0x8D => {
                self.add_with_carry(self.registers.get_l(), ppu);
            }
            0x8E => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.add_with_carry(value, ppu);
                self.nop(ppu);
            }
            0x8F => {
                self.add_with_carry(self.registers.get_a(), ppu);
            }
            0x90 => {
                self.sub_8bit(self.registers.get_b(), ppu);
            }
            0x91 => {
                self.sub_8bit(self.registers.get_c(), ppu);
            }
            0x92 => {
                self.sub_8bit(self.registers.get_d(), ppu);
            }
            0x93 => {
                self.sub_8bit(self.registers.get_e(), ppu);
            }
            0x94 => {
                self.sub_8bit(self.registers.get_h(), ppu);
            }
            0x95 => {
                self.sub_8bit(self.registers.get_l(), ppu);
            }
            0x96 => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.sub_8bit(value, ppu);
                self.nop(ppu);
            }
            0x97 => {
                self.sub_8bit(self.registers.get_a(), ppu);
                let mut f = self.registers.get_f() | (FLAGS::Z as u8 | FLAGS::N as u8);
                f &= !(FLAGS::H as u8 | FLAGS::C as u8);
                self.registers.set_f(f);
            }
            0x98 => {
                self.sub_with_carry(self.registers.get_b(), ppu);
            }
            0x99 => {
                self.sub_with_carry(self.registers.get_c(), ppu);
            }
            0x9A => {
                self.sub_with_carry(self.registers.get_d(), ppu);
            }
            0x9B => {
                self.sub_with_carry(self.registers.get_e(), ppu);
            }
            0x9C => {
                self.sub_with_carry(self.registers.get_h(), ppu);
            }
            0x9D => {
                self.sub_with_carry(self.registers.get_l(), ppu);
            }
            0x9E => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.sub_with_carry(value, ppu);
                self.nop(ppu);
            }
            0x9F => {
                let a = self.registers.get_a();
                let carry = (self.registers.get_f() & FLAGS::C as u8) >> 4; // Extract carry flag

                let (temp_result, _) = a.overflowing_sub(a);
                let (result, _) = temp_result.overflowing_sub(carry); // Subtract carry

                self.registers.set_a(result);

                let mut f = self.registers.get_f() | FLAGS::N as u8;

                if result == 0 {
                    f |= FLAGS::Z as u8;
                } else {
                    f &= !(FLAGS::Z as u8)
                }

                if (a & 0x0F).wrapping_sub(a & 0x0F).wrapping_sub(carry) > 0x0F {
                    f |= FLAGS::H as u8;
                } else {
                    f &= !(FLAGS::H as u8)
                }

                self.registers.set_f(f);
                self.nop(ppu);
            }
            0xA0 => {
                self.and(self.registers.get_b(), ppu);
            }
            0xA1 => {
                self.and(self.registers.get_c(), ppu);
            }
            0xA2 => {
                self.and(self.registers.get_d(), ppu);
            }
            0xA3 => {
                self.and(self.registers.get_e(), ppu);
            }
            0xA4 => {
                self.and(self.registers.get_h(), ppu);
            }
            0xA5 => {
                self.and(self.registers.get_l(), ppu);
            }
            0xA6 => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.and(value, ppu);
                self.nop(ppu);
            }
            0xA7 => {
                self.and(self.registers.get_a(), ppu);
            }
            0xA8 => {
                self.xor(self.registers.get_b(), ppu);
            }
            0xA9 => {
                self.xor(self.registers.get_c(), ppu);
            }
            0xAA => {
                self.xor(self.registers.get_d(), ppu);
            }
            0xAB => {
                self.xor(self.registers.get_e(), ppu);
            }
            0xAC => {
                self.xor(self.registers.get_h(), ppu);
            }
            0xAD => {
                self.xor(self.registers.get_l(), ppu);
            }
            0xAE => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.xor(value, ppu);
                self.nop(ppu);
            }
            0xAF => {
                self.xor(self.registers.get_a(), ppu);
                self.registers
                    .set_f(self.registers.get_f() | (FLAGS::Z as u8));
            }
            0xB0 => {
                self.or(self.registers.get_b(), ppu);
            }
            0xB1 => {
                self.or(self.registers.get_c(), ppu);
            }
            0xB2 => {
                self.or(self.registers.get_d(), ppu);
            }
            0xB3 => {
                self.or(self.registers.get_e(), ppu);
            }
            0xB4 => {
                self.or(self.registers.get_h(), ppu);
            }
            0xB5 => {
                self.or(self.registers.get_l(), ppu);
            }
            0xB6 => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.or(value, ppu);
                self.nop(ppu);
            }
            0xB7 => {
                self.or(self.registers.get_a(), ppu);
            }
            0xB8 => {
                self.compare(self.registers.get_b(), ppu);
            }
            0xB9 => {
                self.compare(self.registers.get_c(), ppu);
            }
            0xBA => {
                self.compare(self.registers.get_d(), ppu);
            }
            0xBB => {
                self.compare(self.registers.get_e(), ppu);
            }
            0xBC => {
                self.compare(self.registers.get_h(), ppu);
            }
            0xBD => {
                self.compare(self.registers.get_l(), ppu);
            }
            0xBE => {
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.compare(value, ppu);
                self.nop(ppu);
            }
            0xBF => {
                self.compare(self.registers.get_a(), ppu);
                let mut f = self.registers.get_f() & !(FLAGS::H as u8 | FLAGS::C as u8);
                f |= FLAGS::Z as u8 | FLAGS::N as u8;
                self.registers.set_f(f);
            }
            0xC0 => {
                self.nop(ppu);
                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    let lower_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                    self.nop(ppu);

                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    self.nop(ppu);
                    let upper_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    self.nop(ppu);

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);

                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0xC1 => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop(ppu);

                let value = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_bc(value);

                self.nop(ppu);
            }
            0xC2 => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    self.nop(ppu);
                    self.registers.set_pc(address);

                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0xC3 => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.nop(ppu);
                self.registers.set_pc(address);
                self.nop(ppu);
            }
            0xC4 => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.nop(ppu);
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );
                    self.nop(ppu);

                    self.registers.set_pc(address);

                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0xC5 => {
                let bc = self.registers.get_bc();
                self.nop(ppu);

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop(ppu);
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (bc >> 8) as u8);
                self.nop(ppu);

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (bc & 0xFF) as u8);
                self.nop(ppu);
            }
            0xC6 => {
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.add_8bit(data, ppu);
                self.nop(ppu);
            }
            0xC7 => {
                self.reset(0x00, ppu);
            }
            0xC8 => {
                self.nop(ppu);
                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    let lower_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    self.nop(ppu);
                    let upper_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);

                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0xC9 => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop(ppu);

                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_pc(address);
            }
            0xCA => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    self.nop(ppu);
                    self.registers.set_pc(address);

                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0xCB => {
                let opcode = self.fetch();
                self.cb(opcode, ppu);
            }
            0xCC => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.nop(ppu);
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.nop(ppu);
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );

                    self.registers.set_pc(address);
                } else {
                    self.nop(ppu);
                }
            }
            0xCD => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop(ppu);
                self.ram.borrow_mut().write(
                    self.registers.get_sp(),
                    (self.registers.get_pc() >> 8) as u8,
                );
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop(ppu);
                self.ram.borrow_mut().write(
                    self.registers.get_sp(),
                    (self.registers.get_pc() & 0xFF) as u8,
                );

                self.registers.set_pc(address);

                self.nop(ppu);
            }
            0xCE => {
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.add_with_carry(data, ppu);
                self.nop(ppu);
            }
            0xCF => {
                self.reset(0x08, ppu);
            }
            0xD0 => {
                self.nop(ppu);
                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.nop(ppu);
                    let lower_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    self.nop(ppu);
                    let upper_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);
                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0xD1 => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop(ppu);

                let value = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_de(value);
            }
            0xD2 => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.nop(ppu);
                    self.registers.set_pc(address);
                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0xD3 => {
                self.nop(ppu);
            }
            0xD4 => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.nop(ppu);
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );

                    self.registers.set_pc(address);

                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0xD5 => {
                let de = self.registers.get_de();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop(ppu);
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (de >> 8) as u8);
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop(ppu);
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (de & 0xFF) as u8);
                self.nop(ppu);
            }
            0xD6 => {
                let address = self.registers.get_and_inc_pc();
                let data = self.ram.borrow_mut().read(address);
                self.sub_8bit(data, ppu);
                self.nop(ppu);
            }
            0xD7 => {
                self.reset(0x10, ppu);
            }
            0xD8 => {
                self.nop(ppu);
                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    let lower_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    self.nop(ppu);
                    let upper_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);

                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0xD9 => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));

                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_pc(address);

                self.ime = true;
                self.nop(ppu);
            }
            0xDA => {
                let lower = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let upper = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = ((upper as u16) << 8) | lower as u16;
                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    self.nop(ppu);
                    self.registers.set_pc(address);
                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0xDB => {
                self.nop(ppu);
            }
            0xDC => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.nop(ppu);
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.nop(ppu);
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );

                    self.registers.set_pc(address);

                    self.nop(ppu);
                } else {
                    self.nop(ppu);
                }
            }
            0xDD => {
                self.nop(ppu);
            }
            0xDE => {
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.sub_with_carry(data, ppu);
                self.nop(ppu);
            }
            0xDF => {
                self.reset(0x18, ppu);
            }
            0xE0 => {
                let offset = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                let address = (0xFF00 as u16).wrapping_add(offset as u16);
                self.nop(ppu);

                self.ram.borrow_mut().write(address, self.registers.get_a());
                self.nop(ppu);
                self.nop(ppu);
            }

            0xE1 => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop(ppu);

                let value = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_hl(value);

                self.nop(ppu);
            }
            0xE2 => {
                let address = 0xff00 | self.registers.get_c() as u16;
                self.ram.borrow_mut().write(address, self.registers.get_a());
                self.nop(ppu);
                self.nop(ppu);
            }
            0xE3 => {
                self.nop(ppu);
            }
            0xE4 => {
                self.nop(ppu);
            }
            0xE5 => {
                let hl = self.registers.get_hl();

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop(ppu);
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (hl >> 8) as u8);
                self.nop(ppu);

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop(ppu);
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (hl & 0xFF) as u8);

                self.nop(ppu);
            }
            0xE6 => {
                let address = self.registers.get_and_inc_pc();
                let data = self.ram.borrow_mut().read(address);
                self.and(data, ppu);
                self.nop(ppu);
            }
            0xE7 => {
                self.reset(0x20, ppu);
            }
            0xE8 => {
                let sp = self.registers.get_sp();
                self.nop(ppu);
                let imm = self.ram.borrow_mut().read(self.registers.get_and_inc_pc()) as i8;
                self.nop(ppu);
                let result = (sp as i16).wrapping_add(imm as i16) as u16;
                self.nop(ppu);
                self.registers.set_sp(result);
                let mut f = 0;
                if ((sp & 0x0F) as i16 + ((imm as i16) & 0x0F)) > 0x0F {
                    f |= FLAGS::H as u8;
                }
                if ((sp & 0xFF) as i16 + ((imm as i16) & 0xFF)) > 0xFF {
                    f |= FLAGS::C as u8;
                }
                self.registers.set_f(f);
                self.nop(ppu);
            }
            0xE9 => {
                self.registers.set_pc(self.registers.get_hl());
                self.nop(ppu);
            }
            0xEA => {
                let lower = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                let upper = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = (upper as u16) << 8 | lower as u16;
                self.nop(ppu);
                self.ram.borrow_mut().write(address, self.registers.get_a());
                self.nop(ppu);
                self.nop(ppu);
            }
            0xEB => {
                self.nop(ppu);
            }
            0xEC => {
                self.nop(ppu);
            }
            0xED => {
                self.nop(ppu);
            }
            0xEE => {
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.xor(data, ppu);
                self.nop(ppu);
            }
            0xEF => {
                self.reset(0x28, ppu);
            }
            0xF0 => {
                let offset = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                let address = 0xFF00u16.wrapping_add(offset as u16);
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                self.registers.set_a(value);
                self.nop(ppu);
            }
            0xF1 => {
                let lower_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop(ppu);
                let upper_byte = self.ram.borrow_mut().read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop(ppu);

                let value = ((upper_byte as u16) << 8) | (lower_byte & 0xF0) as u16;
                self.registers.set_af(value);
            }
            0xF2 => {
                let address = 0xFF00 | self.registers.get_c() as u16;
                let data = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xF3 => {
                self.ime = false;
                self.nop(ppu);
            }
            0xF4 => {
                self.nop(ppu);
            }
            0xF5 => {
                let af = self.registers.get_af();
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop(ppu);
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (af >> 8) as u8);
                self.nop(ppu);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (0xFF & af) as u8);

                self.nop(ppu);
            }
            0xF6 => {
                let data = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.or(data, ppu);
                self.nop(ppu);
            }
            0xF7 => {
                self.reset(0x30, ppu);
            }
            0xF8 => {
                let sp = self.registers.get_sp();
                self.nop(ppu);
                let imm = self.ram.borrow_mut().read(self.registers.get_and_inc_pc()) as i8;
                self.nop(ppu);
                let result = (sp as i16).wrapping_add(imm as i16) as u16;
                self.registers.set_hl(result);
                let mut f = 0;
                if ((sp & 0x0F) as i16 + ((imm as i16) & 0x0F)) > 0x0F {
                    f |= FLAGS::H as u8;
                }
                if ((sp & 0xFF) as i16 + ((imm as i16) & 0xFF)) > 0xFF {
                    f |= FLAGS::C as u8;
                }
                self.registers.set_f(f);
                self.nop(ppu);
            }
            0xF9 => {
                let value = self.registers.get_hl();
                self.nop(ppu);
                self.registers.set_sp(value);
                self.nop(ppu);
            }
            0xFA => {
                let lower = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                let upper = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.nop(ppu);
                let address = ((upper as u16) << 8) | lower as u16;
                self.nop(ppu);
                self.registers.set_a(self.ram.borrow_mut().read(address));
                self.nop(ppu);
                self.nop(ppu);
            }
            0xFB => {
                self.ime_queued = true;
                self.nop(ppu);
            }
            0xFC => {
                self.nop(ppu);
            }
            0xFD => {
                self.nop(ppu);
            }
            0xFE => {
                let n8 = self.ram.borrow_mut().read(self.registers.get_and_inc_pc());
                self.compare(n8, ppu);
                self.nop(ppu);
            }
            0xFF => {
                self.reset(0x38, ppu);
            }
        }
    }

    // special instruction jumped to from 0xCB
    fn cb(&mut self, opcode: u8, ppu: &mut PPU) {
        match opcode {
            0x00 => {
                let data = self.rotate_without_carry(self.registers.get_b(), 0, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x01 => {
                let data = self.rotate_without_carry(self.registers.get_c(), 0, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x02 => {
                let data = self.rotate_without_carry(self.registers.get_d(), 0, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x03 => {
                let data = self.rotate_without_carry(self.registers.get_e(), 0, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x04 => {
                let data = self.rotate_without_carry(self.registers.get_h(), 0, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x05 => {
                let data = self.rotate_without_carry(self.registers.get_l(), 0, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x06 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(address);
                let data = self.rotate_without_carry(value, 0, ppu);
                self.ram.borrow_mut().write(address, data);
                self.nop(ppu);
                self.nop(ppu);
            }
            0x07 => {
                let data = self.rotate_without_carry(self.registers.get_a(), 0, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x08 => {
                let data = self.rotate_without_carry(self.registers.get_b(), 1, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x09 => {
                let data = self.rotate_without_carry(self.registers.get_c(), 1, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x0A => {
                let data = self.rotate_without_carry(self.registers.get_d(), 1, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x0B => {
                let data = self.rotate_without_carry(self.registers.get_e(), 1, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x0C => {
                let data = self.rotate_without_carry(self.registers.get_h(), 1, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x0D => {
                let data = self.rotate_without_carry(self.registers.get_l(), 1, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x0E => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(address);
                let data = self.rotate_without_carry(value, 1, ppu);
                self.ram.borrow_mut().write(address, data);
                self.nop(ppu);
                self.nop(ppu);
            }
            0x0F => {
                let data = self.rotate_without_carry(self.registers.get_a(), 1, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x10 => {
                let data = self.rotate(self.registers.get_b(), 0, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x11 => {
                let data = self.rotate(self.registers.get_c(), 0, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x12 => {
                let data = self.rotate(self.registers.get_d(), 0, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x13 => {
                let data = self.rotate(self.registers.get_e(), 0, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x14 => {
                let data = self.rotate(self.registers.get_h(), 0, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x15 => {
                let data = self.rotate(self.registers.get_l(), 0, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x16 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(address);
                let data = self.rotate(value, 0, ppu);
                self.ram.borrow_mut().write(address, data);
                self.nop(ppu);
                self.nop(ppu);
            }
            0x17 => {
                let data = self.rotate(self.registers.get_a(), 0, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x18 => {
                let data = self.rotate(self.registers.get_b(), 1, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x19 => {
                let data = self.rotate(self.registers.get_c(), 1, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x1A => {
                let data = self.rotate(self.registers.get_d(), 1, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x1B => {
                let data = self.rotate(self.registers.get_e(), 1, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x1C => {
                let data = self.rotate(self.registers.get_h(), 1, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x1D => {
                let data = self.rotate(self.registers.get_l(), 1, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x1E => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(address);
                let data = self.rotate(value, 1, ppu);
                self.ram.borrow_mut().write(address, data);
                self.nop(ppu);
                self.nop(ppu);
            }
            0x1F => {
                let data = self.rotate(self.registers.get_a(), 1, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x20 => {
                let data = self.shift(self.registers.get_b(), 0, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x21 => {
                let data = self.shift(self.registers.get_c(), 0, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x22 => {
                let data = self.shift(self.registers.get_d(), 0, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x23 => {
                let data = self.shift(self.registers.get_e(), 0, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x24 => {
                let data = self.shift(self.registers.get_h(), 0, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x25 => {
                let data = self.shift(self.registers.get_l(), 0, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x26 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(address);
                let data = self.shift(value, 0, ppu);
                self.ram.borrow_mut().write(address, data);
                self.nop(ppu);
                self.nop(ppu);
            }
            0x27 => {
                let data = self.shift(self.registers.get_a(), 0, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x28 => {
                let data = self.shift(self.registers.get_b(), 1, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x29 => {
                let data = self.shift(self.registers.get_c(), 1, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x2A => {
                let data = self.shift(self.registers.get_d(), 1, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x2B => {
                let data = self.shift(self.registers.get_e(), 1, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x2C => {
                let data = self.shift(self.registers.get_h(), 1, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x2D => {
                let data = self.shift(self.registers.get_l(), 1, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x2E => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(address);
                let data = self.shift(value, 1, ppu);
                self.ram.borrow_mut().write(address, data);
                self.nop(ppu);
                self.nop(ppu);
            }
            0x2F => {
                let data = self.shift(self.registers.get_a(), 1, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x30 => {
                let data = self.swap(self.registers.get_b(), ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x31 => {
                let data = self.swap(self.registers.get_c(), ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x32 => {
                let data = self.swap(self.registers.get_d(), ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x33 => {
                let data = self.swap(self.registers.get_e(), ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x34 => {
                let data = self.swap(self.registers.get_h(), ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x35 => {
                let data = self.swap(self.registers.get_l(), ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x36 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(address);
                let data = self.swap(value, ppu);
                self.ram.borrow_mut().write(address, data);
                self.nop(ppu);
                self.nop(ppu);
            }
            0x37 => {
                let data = self.swap(self.registers.get_a(), ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x38 => {
                let data = self.right_shift(self.registers.get_b(), ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x39 => {
                let data = self.right_shift(self.registers.get_c(), ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x3A => {
                let data = self.right_shift(self.registers.get_d(), ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x3B => {
                let data = self.right_shift(self.registers.get_e(), ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x3C => {
                let data = self.right_shift(self.registers.get_h(), ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x3D => {
                let data = self.right_shift(self.registers.get_l(), ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x3E => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(address);
                let data = self.right_shift(value, ppu);
                self.ram.borrow_mut().write(address, data);
                self.nop(ppu);
                self.nop(ppu);
            }
            0x3F => {
                let data = self.right_shift(self.registers.get_a(), ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x40 => self.bit(self.registers.get_b(), 0, ppu),
            0x41 => self.bit(self.registers.get_c(), 0, ppu),
            0x42 => self.bit(self.registers.get_d(), 0, ppu),
            0x43 => self.bit(self.registers.get_e(), 0, ppu),
            0x44 => self.bit(self.registers.get_h(), 0, ppu),
            0x45 => self.bit(self.registers.get_l(), 0, ppu),
            0x46 => {
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.bit(value, 0, ppu);
            }
            0x47 => self.bit(self.registers.get_a(), 0, ppu),
            0x48 => self.bit(self.registers.get_b(), 1, ppu),
            0x49 => self.bit(self.registers.get_c(), 1, ppu),
            0x4A => self.bit(self.registers.get_d(), 1, ppu),
            0x4B => self.bit(self.registers.get_e(), 1, ppu),
            0x4C => self.bit(self.registers.get_h(), 1, ppu),
            0x4D => self.bit(self.registers.get_l(), 1, ppu),
            0x4E => {
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.bit(value, 1, ppu);
            }
            0x4F => self.bit(self.registers.get_a(), 1, ppu),
            0x50 => self.bit(self.registers.get_b(), 2, ppu),
            0x51 => self.bit(self.registers.get_c(), 2, ppu),
            0x52 => self.bit(self.registers.get_d(), 2, ppu),
            0x53 => self.bit(self.registers.get_e(), 2, ppu),
            0x54 => self.bit(self.registers.get_h(), 2, ppu),
            0x55 => self.bit(self.registers.get_l(), 2, ppu),
            0x56 => {
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.bit(value, 2, ppu);
            }
            0x57 => self.bit(self.registers.get_a(), 2, ppu),
            0x58 => self.bit(self.registers.get_b(), 3, ppu),
            0x59 => self.bit(self.registers.get_c(), 3, ppu),
            0x5A => self.bit(self.registers.get_d(), 3, ppu),
            0x5B => self.bit(self.registers.get_e(), 3, ppu),
            0x5C => self.bit(self.registers.get_h(), 3, ppu),
            0x5D => self.bit(self.registers.get_l(), 3, ppu),
            0x5E => {
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.bit(value, 3, ppu);
            }
            0x5F => self.bit(self.registers.get_a(), 3, ppu),
            0x60 => self.bit(self.registers.get_b(), 4, ppu),
            0x61 => self.bit(self.registers.get_c(), 4, ppu),
            0x62 => self.bit(self.registers.get_d(), 4, ppu),
            0x63 => self.bit(self.registers.get_e(), 4, ppu),
            0x64 => self.bit(self.registers.get_h(), 4, ppu),
            0x65 => self.bit(self.registers.get_l(), 4, ppu),
            0x66 => {
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.bit(value, 4, ppu);
            }
            0x67 => self.bit(self.registers.get_a(), 4, ppu),
            0x68 => self.bit(self.registers.get_b(), 5, ppu),
            0x69 => self.bit(self.registers.get_c(), 5, ppu),
            0x6A => self.bit(self.registers.get_d(), 5, ppu),
            0x6B => self.bit(self.registers.get_e(), 5, ppu),
            0x6C => self.bit(self.registers.get_h(), 5, ppu),
            0x6D => self.bit(self.registers.get_l(), 5, ppu),
            0x6E => {
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.bit(value, 5, ppu);
            }
            0x6F => self.bit(self.registers.get_a(), 5, ppu),
            0x70 => self.bit(self.registers.get_b(), 6, ppu),
            0x71 => self.bit(self.registers.get_c(), 6, ppu),
            0x72 => self.bit(self.registers.get_d(), 6, ppu),
            0x73 => self.bit(self.registers.get_e(), 6, ppu),
            0x74 => self.bit(self.registers.get_h(), 6, ppu),
            0x75 => self.bit(self.registers.get_l(), 6, ppu),
            0x76 => {
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.bit(value, 6, ppu);
            }
            0x77 => self.bit(self.registers.get_a(), 6, ppu),
            0x78 => self.bit(self.registers.get_b(), 7, ppu),
            0x79 => self.bit(self.registers.get_c(), 7, ppu),
            0x7A => self.bit(self.registers.get_d(), 7, ppu),
            0x7B => self.bit(self.registers.get_e(), 7, ppu),
            0x7C => self.bit(self.registers.get_h(), 7, ppu),
            0x7D => self.bit(self.registers.get_l(), 7, ppu),
            0x7E => {
                self.nop(ppu);
                let value = self.ram.borrow_mut().read(self.registers.get_hl());
                self.bit(value, 7, ppu);
            }
            0x7F => self.bit(self.registers.get_a(), 7, ppu),

            0x80 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 0, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x81 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 0, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x82 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 0, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x83 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 0, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x84 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 0, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x85 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 0, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x86 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let data = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                self.ram.borrow_mut().write(address, data & !(1 << 0));
                self.nop(ppu);
                self.nop(ppu);
            }
            0x87 => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 0, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x88 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 1, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x89 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 1, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x8A => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 1, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x8B => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 1, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x8C => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 1, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x8D => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 1, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x8E => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let data = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                self.ram.borrow_mut().write(address, data & !(1 << 1));
                self.nop(ppu);
                self.nop(ppu);
            }
            0x8F => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 1, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x90 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 2, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x91 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 2, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x92 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 2, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x93 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 2, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x94 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 2, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x95 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 2, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x96 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let data = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                self.ram.borrow_mut().write(address, data & !(1 << 2));
                self.nop(ppu);
                self.nop(ppu);
            }
            0x97 => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 2, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0x98 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 3, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0x99 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 3, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0x9A => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 3, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0x9B => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 3, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0x9C => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 3, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0x9D => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 3, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0x9E => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let data = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                self.ram.borrow_mut().write(address, data & !(1 << 3));
                self.nop(ppu);
                self.nop(ppu);
            }
            0x9F => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 3, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xA0 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 4, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xA1 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 4, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xA2 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 4, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xA3 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 4, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xA4 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 4, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xA5 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 4, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xA6 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let data = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                self.ram.borrow_mut().write(address, data & !(1 << 4));
                self.nop(ppu);
                self.nop(ppu);
            }
            0xA7 => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 4, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xA8 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 5, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xA9 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 5, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xAA => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 5, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xAB => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 5, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xAC => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 5, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xAD => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 5, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xAE => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let data = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                self.ram.borrow_mut().write(address, data & !(1 << 5));
                self.nop(ppu);
                self.nop(ppu);
            }
            0xAF => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 5, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xB0 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 6, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xB1 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 6, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xB2 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 6, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xB3 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 6, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xB4 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 6, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xB5 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 6, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xB6 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let data = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                self.ram.borrow_mut().write(address, data & !(1 << 6));
                self.nop(ppu);
                self.nop(ppu);
            }
            0xB7 => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 6, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xB8 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 7, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xB9 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 7, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xBA => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 7, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xBB => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 7, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xBC => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 7, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xBD => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 7, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xBE => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let data = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                self.ram.borrow_mut().write(address, data & !(1 << 7));
                self.nop(ppu);
                self.nop(ppu);
            }
            0xBF => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 7, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xC0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 0, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xC1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 0, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xC2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 0, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xC3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 0, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xC4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 0, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xC5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 0, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xC6 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let byte = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                let new_byte = byte | (1 << 0);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop(ppu);
                self.nop(ppu);
            }
            0xC7 => {
                let data = self.set_bit_8bit(self.registers.get_a(), 0, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xC8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 1, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xC9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 1, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xCA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 1, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xCB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 1, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xCC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 1, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xCD => {
                let data = self.set_bit_8bit(self.registers.get_l(), 1, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xCE => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let byte = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                let new_byte = byte | (1 << 1);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop(ppu);
                self.nop(ppu);
            }
            0xCF => {
                let data = self.set_bit_8bit(self.registers.get_a(), 1, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xD0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 2, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xD1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 2, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xD2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 2, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xD3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 2, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xD4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 2, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xD5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 2, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xD6 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let byte = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                let new_byte = byte | (1 << 2);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop(ppu);
                self.nop(ppu);
            }
            0xD7 => {
                let data = self.set_bit_8bit(self.registers.get_a(), 2, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xD8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 3, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xD9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 3, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xDA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 3, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xDB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 3, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xDC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 3, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xDD => {
                let data = self.set_bit_8bit(self.registers.get_l(), 3, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xDE => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let byte = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                let new_byte = byte | (1 << 3);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop(ppu);
                self.nop(ppu);
            }
            0xDF => {
                let data = self.set_bit_8bit(self.registers.get_a(), 3, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xE0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 4, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xE1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 4, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xE2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 4, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xE3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 4, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xE4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 4, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xE5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 4, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xE6 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let byte = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                let new_byte = byte | (1 << 4);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop(ppu);
                self.nop(ppu);
            }
            0xE7 => {
                let data = self.set_bit_8bit(self.registers.get_a(), 4, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xE8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 5, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xE9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 5, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xEA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 5, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xEB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 5, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xEC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 5, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xED => {
                let data = self.set_bit_8bit(self.registers.get_l(), 5, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xEE => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let byte = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                let new_byte = byte | (1 << 5);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop(ppu);
                self.nop(ppu);
            }
            0xEF => {
                let data = self.set_bit_8bit(self.registers.get_a(), 5, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xF0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 6, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xF1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 6, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xF2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 6, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xF3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 6, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xF4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 6, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xF5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 6, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xF6 => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let byte = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                let new_byte = byte | (1 << 6);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop(ppu);
                self.nop(ppu);
            }
            0xF7 => {
                let data = self.set_bit_8bit(self.registers.get_a(), 6, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
            0xF8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 7, ppu);
                self.registers.set_b(data);
                self.nop(ppu);
            }
            0xF9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 7, ppu);
                self.registers.set_c(data);
                self.nop(ppu);
            }
            0xFA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 7, ppu);
                self.registers.set_d(data);
                self.nop(ppu);
            }
            0xFB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 7, ppu);
                self.registers.set_e(data);
                self.nop(ppu);
            }
            0xFC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 7, ppu);
                self.registers.set_h(data);
                self.nop(ppu);
            }
            0xFD => {
                let data = self.set_bit_8bit(self.registers.get_l(), 7, ppu);
                self.registers.set_l(data);
                self.nop(ppu);
            }
            0xFE => {
                let address = self.registers.get_hl();
                self.nop(ppu);
                let byte = self.ram.borrow_mut().read(address);
                self.nop(ppu);
                let new_byte = byte | (1 << 7);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop(ppu);
                self.nop(ppu);
            }
            0xFF => {
                let data = self.set_bit_8bit(self.registers.get_a(), 7, ppu);
                self.registers.set_a(data);
                self.nop(ppu);
            }
        }
    }
}
