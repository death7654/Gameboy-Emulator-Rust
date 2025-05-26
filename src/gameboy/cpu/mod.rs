mod registers;
mod timer;

use crate::gameboy::RAM;

use registers::FLAGS;
use registers::REGISTERS;

use timer::Timer;

use std::cell::RefCell;
use std::rc::Rc;

use std::fs::OpenOptions;
use std::io::Write;

//todo timer error
//todo seperate cpu into different steps
//increment the timer, ppu, etc every 4 cycles
//handle interrupts every time you fetch

pub struct CPU {
    pub registers: REGISTERS,
    pub ime: bool, //interrupt master enable
    ime_queued: bool,
    pub cycles: u64,

    pub ram: Rc<RefCell<RAM>>,
    timer: Timer,
    pub halted: bool,
    pub stopped: bool,
}
impl CPU {
    pub fn new(ram_: Rc<RefCell<RAM>>) -> Self {
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
    pub fn log_cpu_state(&self) {
        // Retrieve register values (assumes you have these getter methods)
        let a = self.registers.get_a();
        let f = self.registers.get_f();
        let b = self.registers.get_b();
        let c = self.registers.get_c();
        let d = self.registers.get_d();
        let e = self.registers.get_e();
        let h = self.registers.get_h();
        let l = self.registers.get_l();
        let sp = self.registers.get_sp();
        let pc = self.registers.get_pc();

        // Read 4 bytes from memory starting at PC for PCMEM output.
        let pc_mem_bytes = [
            self.ram.borrow().read(pc.wrapping_add(0)),
            self.ram.borrow().read(pc.wrapping_add(1)),
            self.ram.borrow().read(pc.wrapping_add(2)),
            self.ram.borrow().read(pc.wrapping_add(3)),
        ];

        // Format the output string.
        let log_line = format!(
            "A:{:02X} F:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} PC:{:04X} PCMEM:{:02X},{:02X},{:02X},{:02X}\n",
            a, f, b, c, d, e, h, l,
            sp,
            pc,
            pc_mem_bytes[0], pc_mem_bytes[1], pc_mem_bytes[2], pc_mem_bytes[3]
        );

        // Open "cpu_log.txt" in append mode (create if it doesn't exist)
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("cpu_log.log")
        {
            // Write the formatted log line into the file.
            if let Err(err) = file.write_all(log_line.as_bytes()) {
                println!("Failed to write to log file: {}", err);
            }
        } else {
            println!("Failed to open log file for writing.");
        }
    }
    pub fn fetch(&mut self) -> u8 {
        let pc = self.registers.get_pc();
        let value = self.ram.borrow().read(pc);
        self.registers.set_pc(pc.wrapping_add(1));
        value
    }

    pub fn handle_interrupt(&mut self) {
        if self.ime_queued {
            self.ime = true;
            self.ime_queued = false;
        }

        let ie = self.ram.borrow().read(0xFFFF);
        let iflag = self.ram.borrow().read(0xFF0F);
        let pending = ie & iflag;

        if self.halted {
            self.nop();
        }

        if pending == 0 {
            return;
        }

        // 3) HALT-bug: IME=0 + pending → exit HALT but do *not* service
        if !self.ime && self.halted {
            self.halt_bug();
            return;
        }

        self.halted = false;

        if self.ime {
            // 4) Service highest-priority pending interrupt
            for &(bit, vector) in &[
                (0, 0x0040), // V-Blank
                (1, 0x0048), // LCD STAT
                (2, 0x0050), // Timer
                (3, 0x0058), // Serial
                (4, 0x0060), // Joypad
            ] {
                if bit == 2 {
                    self.timer.tima_overflowed = false;
                }
                if (pending & (1 << bit)) != 0 {
                    self.service_interrupt(bit, vector);
                    break;
                }
            }
        }
    }

    fn service_interrupt(&mut self, bit: u8, vector: u16) {
        println!(
            "Interrupt serviced: IME: {}, IF: {:08b}, IE: {:08b} Halt: {}",
            self.ime,
            self.ram.borrow().read(0xFF0F),
            self.ram.borrow().read(0xFFFF),
            self.halted
        );
        self.halted = false;
        self.ime = false;

        self.cycles += 8;

        self.push_pc();

        let mut iflag = self.ram.borrow().read(0xFF0F);
        iflag &= !(1 << bit);
        self.ram.borrow_mut().write(0xFF0F, iflag);

        self.registers.set_pc(vector);

        println!(
            "after serviced: IME: {}, IF: {:08b}, IE: {:08b} Halt: {}",
            self.ime,
            self.ram.borrow().read(0xFF0F),
            self.ram.borrow().read(0xFFFF),
            self.halted
        );
    }

    fn halt_bug(&mut self) {
        self.halted = false;
        let pc = self.registers.get_pc();
        let opcode = self.ram.borrow().read(pc);
        self.execute(opcode);
    }

    fn push_pc(&mut self) {
        let value = self.registers.get_pc();
        self.registers
            .set_sp(self.registers.get_sp().wrapping_sub(1));
        self.ram
            .borrow_mut()
            .write(self.registers.get_sp(), ((value & 0xFF00) >> 8) as u8);
        self.registers
            .set_sp(self.registers.get_sp().wrapping_sub(1));
        self.ram
            .borrow_mut()
            .write(self.registers.get_sp(), (value & 0xFF) as u8);
    }

    fn tick(&mut self) {
        self.timer.timer(4);

        if (self.ime_queued) {
            self.ime = true;
        }
    }
    pub fn nop(&mut self) {
        self.tick();
    }
    fn load_16(&mut self, address: u16) -> u8 {
        let value = self.ram.borrow().read(address);
        self.nop();
        value
    }
    fn load_a(&mut self, value: u8) {
        self.nop();
        self.registers.set_a(value);
    }
    fn load_b(&mut self, data: u8) {
        self.registers.set_b(data);
        self.nop();
    }
    fn load_c(&mut self, data: u8) {
        self.registers.set_c(data);
        self.nop();
    }
    fn load_d(&mut self, data: u8) {
        self.registers.set_d(data);
        self.nop();
    }
    fn load_e(&mut self, data: u8) {
        self.registers.set_e(data);
        self.nop();
    }
    fn load_h(&mut self, data: u8) {
        self.registers.set_h(data);
        self.nop();
    }
    fn load_l(&mut self, data: u8) {
        self.registers.set_l(data);
        self.nop();
    }
    fn store(&mut self, address: u16, data: u8) {
        self.ram.borrow_mut().write(address, data);
        self.nop();
    }

    fn daa(&mut self) {
        let a = self.registers.get_a();
        let mut f = self.registers.get_f();

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
        self.nop();
    }

    fn increment_bc(&mut self) {
        let bc = self.registers.get_bc();
        self.nop();
        self.registers.set_bc(bc.wrapping_add(1));
        self.nop();
    }
    fn increment_de(&mut self) {
        let de = self.registers.get_de();
        self.nop();
        self.registers.set_de(de.wrapping_add(1));
        self.nop();
    }
    fn increment_hl(&mut self) {
        let hl = self.registers.get_hl();
        self.nop();
        self.registers.set_hl(hl.wrapping_add(1));
        self.nop();
    }
    fn increment_sp(&mut self) {
        let sp = self.registers.get_sp();
        self.nop();
        self.registers.set_sp(sp.wrapping_add(1));
        self.nop();
    }
    fn rlca(&mut self) {
        let a = self.registers.get_a();
        let carry = (a & 0x80) != 0;

        let result = a.rotate_left(1);

        self.registers.set_a(result);

        let mut new_flags: u8 = 0;
        if carry {
            new_flags |= FLAGS::C as u8;
        }
        self.registers.set_f(new_flags);

        self.nop();
    }

    fn add_16bit(&mut self, value1: u16, value2: u16) -> u16 {
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
        self.nop();
        result
    }
    fn increment_8_bit(&mut self, data: u8) -> u8 {
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
        self.nop();
        result
    }
    fn decrement_8_bit(&mut self, data: u8) -> u8 {
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
        self.nop();
        result
    }
    fn zero_bit_8bit(&mut self, value: u8, bit: u8) -> u8 {
        self.nop();
        value & !(1 << bit)
    }
    fn set_bit_8bit(&mut self, value: u8, bit: u8) -> u8 {
        self.nop();
        value | (1 << bit)
    }
    fn add_8bit(&mut self, b: u8) {
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
        self.nop();
    }
    fn add_with_carry(&mut self, b: u8) {
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
        self.nop();
    }
    fn sub_8bit(&mut self, b: u8) {
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
        self.nop();
    }
    fn sub_with_carry(&mut self, b: u8) {
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
        self.nop();
    }

    fn and(&mut self, b: u8) {
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
        self.nop();
    }
    fn xor(&mut self, b: u8) {
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
        self.nop();
    }
    fn or(&mut self, b: u8) {
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
        self.nop();
    }

    fn compare(&mut self, n8: u8) {
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
        self.nop();
    }

    fn reset(&mut self, address: u16) {
        self.registers
            .set_sp(self.registers.get_sp().wrapping_sub(1));
        self.nop();
        self.ram.borrow_mut().write(
            self.registers.get_sp(),
            (self.registers.get_pc() >> 8) as u8,
        );
        self.nop();
        self.registers
            .set_sp(self.registers.get_sp().wrapping_sub(1));
        self.nop();
        self.ram.borrow_mut().write(
            self.registers.get_sp(),
            (self.registers.get_pc() & 0xFF) as u8,
        );

        self.registers.set_pc(address);
        self.nop();
    }
    fn rotate_without_carry(&mut self, mut value: u8, type_: u8) -> u8 {
        self.nop();
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

    fn rotate(&mut self, mut value: u8, type_: u8) -> u8 {
        self.nop();
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

    fn shift(&mut self, mut value: u8, type_: u8) -> u8 {
        // Read the old flags, but we’re going to build the new flags from scratch.
        self.nop();
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

    fn swap(&mut self, value: u8) -> u8 {
        self.nop();
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
    fn right_shift(&mut self, mut value: u8) -> u8 {
        self.nop();

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
    fn bit(&mut self, value: u8, bit: u8) {
        self.nop();
        self.nop();
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

    pub fn execute(&mut self, opcode: u8) {
        match opcode {
            0x00 => {
                //NOP
                self.nop();
            }
            0x01 => {
                //Load 2 bytes into register BC
                let mut value = self.registers.get_and_inc_pc();
                let lower_byte = self.load_16(value);

                value = self.registers.get_and_inc_pc();
                let upper_byte = self.load_16(value);

                let data = ((upper_byte as u16) << 8) | (lower_byte as u16);
                self.nop();
                self.registers.set_bc(data);
            }
            0x02 => {
                //store the data in a into the ram address found in bc
                self.store(self.registers.get_bc(), self.registers.get_a());
                self.nop();
            }
            0x03 => {
                //increment bc
                self.increment_bc();
            }
            0x04 => {
                //increment b
                let data = self.increment_8_bit(self.registers.get_b());
                self.registers.set_b(data);
            }
            0x05 => {
                //decrement b
                let data = self.decrement_8_bit(self.registers.get_b());
                self.registers.set_b(data);
            }
            0x06 => {
                //load 1 byte into B
                self.nop();
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.load_b(data);
            }
            0x07 => {
                self.rlca();
            }
            0x08 => {
                //load sp into the address from ram
                let mut address = self.registers.get_and_inc_pc();
                let lower_byte = self.load_16(address);

                address = self.registers.get_and_inc_pc();
                let upper_byte = self.load_16(address);

                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                let sp = self.registers.get_sp();
                self.nop();

                self.ram.borrow_mut().write(address, (sp & 0x00FF) as u8);
                self.nop();

                self.ram.borrow_mut().write(address + 1, (sp >> 8) as u8);
                self.nop();
            }
            0x09 => {
                self.nop();
                let hl = self.registers.get_hl();
                let bc = self.registers.get_bc();
                let result = self.add_16bit(hl, bc);
                self.registers.set_hl(result);
            }
            0x0A => {
                let data = self.ram.borrow().read(self.registers.get_bc());
                self.load_a(data);
                self.nop();
            }
            0x0B => {
                //decrement BC
                self.nop();
                self.registers
                    .set_bc(self.registers.get_bc().wrapping_sub(1));
                self.nop();
            }
            0x0C => {
                let data = self.increment_8_bit(self.registers.get_c());
                self.registers.set_c(data);
            }
            0x0D => {
                let data = self.decrement_8_bit(self.registers.get_c());
                self.registers.set_c(data);
            }
            0x0E => {
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                self.load_c(data);
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

                self.nop();
            }
            0x10 => {
                println!("CPU Stopped");
                self.stopped = true;
                self.nop();
            }
            0x11 => {
                let mut value = self.registers.get_and_inc_pc();
                let lower_byte = self.load_16(value);

                value = self.registers.get_and_inc_pc();
                let upper_byte = self.load_16(value);

                let data = ((upper_byte as u16) << 8) | (lower_byte as u16);
                self.nop();
                self.registers.set_de(data);
            }
            0x12 => {
                self.store(self.registers.get_de(), self.registers.get_a());
                self.nop();
            }
            0x13 => {
                self.increment_de();
            }
            0x14 => {
                let data = self.increment_8_bit(self.registers.get_d());
                self.registers.set_d(data);
            }
            0x15 => {
                let data = self.decrement_8_bit(self.registers.get_d());
                self.registers.set_d(data);
            }
            0x16 => {
                self.nop();
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.load_d(data);
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
                self.nop();
            }
            0x18 => {
                //relative jump
                let jump = self.ram.borrow().read(self.registers.get_and_inc_pc()) as i8;
                self.nop();
                let new_pc = (self.registers.get_pc() as i16).wrapping_add(jump as i16) as u16;
                self.nop();
                self.registers.set_pc(new_pc);
                self.nop();
            }
            0x19 => {
                self.nop();
                let hl = self.registers.get_hl();
                let de = self.registers.get_de();
                let result = self.add_16bit(hl, de);
                self.registers.set_hl(result);
            }
            0x1A => {
                let data = self.ram.borrow().read(self.registers.get_de());
                self.load_a(data);
                self.nop();
            }
            0x1B => {
                //decrement de
                self.nop();
                self.registers
                    .set_de(self.registers.get_de().wrapping_sub(1));
                self.nop();
            }
            0x1C => {
                let data = self.increment_8_bit(self.registers.get_e());
                self.registers.set_e(data);
            }
            0x1d => {
                let data = self.decrement_8_bit(self.registers.get_e());
                self.registers.set_e(data);
            }
            0x1E => {
                //load the next byte onto register E
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                self.load_e(data);
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
                self.nop();
            }
            0x20 => {
                let jump = self.ram.borrow().read(self.registers.get_and_inc_pc()) as i8;
                self.nop();
                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    self.nop();
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(jump as i16 as u16));
                    self.nop();
                } else {
                    self.nop();
                }
            }
            0x21 => {
                let mut value = self.registers.get_and_inc_pc();
                let lower_byte = self.load_16(value);

                value = self.registers.get_and_inc_pc();
                let upper_byte = self.load_16(value);

                let data = ((upper_byte as u16) << 8) | (lower_byte as u16);
                self.nop();
                self.registers.set_hl(data);
            }
            0x22 => {
                //load a into memory with the address found in HL and increment HL by 1
                let hl = self.registers.get_hl();
                self.store(hl, self.registers.get_a());
                self.registers.set_hl(hl.wrapping_add(1));
                self.nop();
            }
            0x23 => {
                self.increment_hl();
            }
            0x24 => {
                let data = self.increment_8_bit(self.registers.get_h());
                self.registers.set_h(data);
            }
            0x25 => {
                let data = self.decrement_8_bit(self.registers.get_h());
                self.registers.set_h(data);
            }
            0x26 => {
                self.nop();
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.load_h(data);
            }
            0x27 => {
                self.daa();
            }

            0x28 => {
                let offset = self.ram.borrow().read(self.registers.get_and_inc_pc()) as i8;
                self.nop();
                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    self.nop();
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(offset as i16 as u16));
                    self.nop();
                } else {
                    self.nop();
                }
            }
            0x29 => {
                self.nop();
                let hl = self.registers.get_hl();
                let result = self.add_16bit(hl, hl);
                self.registers.set_hl(result);
            }
            0x2A => {
                let address = self.registers.get_hl();
                let data = self.ram.borrow().read(address);
                self.registers.set_hl(address.wrapping_add(1));
                self.load_a(data);
                self.nop();
            }
            0x2B => {
                self.nop();
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_sub(1));
                self.nop();
            }
            0x2C => {
                let data = self.increment_8_bit(self.registers.get_l());
                self.registers.set_l(data);
            }
            0x2D => {
                let data = self.decrement_8_bit(self.registers.get_l());
                self.registers.set_l(data);
            }
            0x2E => {
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                self.load_l(data);
            }
            0x2F => {
                let a = self.registers.get_a();
                self.registers.set_a(!a); // Bitwise complement

                // Set N and H flags
                self.registers
                    .set_f(self.registers.get_f() | FLAGS::N as u8 | FLAGS::H as u8);

                self.nop();
            }
            0x30 => {
                let offset = self.ram.borrow().read(self.registers.get_and_inc_pc()) as i8;
                self.nop();
                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.nop();
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(offset as i16 as u16));
                    self.nop();
                } else {
                    self.nop();
                }
            }
            0x31 => {
                let mut value = self.registers.get_and_inc_pc();
                let lower_byte = self.load_16(value);

                value = self.registers.get_and_inc_pc();
                let upper_byte = self.load_16(value);

                let data = ((upper_byte as u16) << 8) | (lower_byte as u16);
                self.nop();
                self.registers.set_sp(data);
            }
            0x32 => {
                let hl = self.registers.get_hl();
                self.store(hl, self.registers.get_a());
                self.registers.set_hl(hl.wrapping_sub(1));
                self.nop();
            }
            0x33 => {
                self.increment_sp();
            }
            0x34 => {
                let address = self.registers.get_hl();
                let value = self.ram.borrow().read(address);
                self.nop();
                let result = value.wrapping_add(1);
                self.ram.borrow_mut().write(address, result);
                self.nop();

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

                self.nop();
            }
            0x35 => {
                let address = self.registers.get_hl();
                let value = self.ram.borrow().read(address);
                self.nop();
                let result = value.wrapping_sub(1);
                self.ram.borrow_mut().write(address, result);
                self.nop();

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
                self.nop();
            }
            0x36 => {
                let address = self.registers.get_hl();
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                self.ram.borrow_mut().write(address, data);
                self.nop();
                self.nop();
            }
            0x37 => {
                self.registers
                    .set_f(self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8));

                self.registers
                    .set_f(self.registers.get_f() | FLAGS::C as u8);

                self.nop();
            }
            0x38 => {
                let offset = self.ram.borrow().read(self.registers.get_and_inc_pc()) as i8;
                self.nop();
                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    self.nop();
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(offset as i16 as u16));
                    self.nop();
                } else {
                    self.nop();
                }
            }
            0x39 => {
                let hl = self.registers.get_hl();
                let sp = self.registers.get_sp();
                let result = hl.wrapping_add(sp);
                self.registers.set_hl(result);
                self.nop();

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

                self.nop();
            }
            0x3A => {
                let data = self.ram.borrow().read(self.registers.get_hl());
                self.nop();
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_sub(1));

                self.registers.set_a(data);
                self.nop();
            }
            0x3B => {
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop();
            }
            0x3C => {
                let data = self.increment_8_bit(self.registers.get_a());
                self.registers.set_a(data);
            }
            0x3D => {
                let data = self.decrement_8_bit(self.registers.get_a());
                self.registers.set_a(data);
            }
            0x3E => {
                let value = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.load_a(value);
                self.nop();
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
                self.nop();
            }
            0x40 => {
                self.nop();
            }
            0x41 => {
                self.registers.set_b(self.registers.get_c());
                self.nop();
            }
            0x42 => {
                self.registers.set_b(self.registers.get_d());
                self.nop();
            }
            0x43 => {
                self.registers.set_b(self.registers.get_e());
                self.nop();
            }
            0x44 => {
                self.registers.set_b(self.registers.get_h());
                self.nop();
            }
            0x45 => {
                self.registers.set_b(self.registers.get_l());
                self.nop();
            }
            0x46 => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.nop();
                self.registers.set_b(value);
                self.nop();
            }
            0x47 => {
                self.registers.set_b(self.registers.get_a());
                self.nop();
            }
            0x48 => {
                self.registers.set_c(self.registers.get_b());
                self.nop();
            }
            0x49 => {
                self.nop();
            }
            0x4A => {
                self.registers.set_c(self.registers.get_d());
                self.nop();
            }
            0x4B => {
                self.registers.set_c(self.registers.get_e());
                self.nop();
            }
            0x4C => {
                self.registers.set_c(self.registers.get_h());
                self.nop();
            }
            0x4D => {
                self.registers.set_c(self.registers.get_l());
                self.nop();
            }
            0x4E => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.nop();
                self.registers.set_c(value);
                self.nop();
            }
            0x4F => {
                self.registers.set_c(self.registers.get_a());
                self.nop();
            }
            0x50 => {
                self.registers.set_d(self.registers.get_b());
                self.nop();
            }
            0x51 => {
                self.registers.set_d(self.registers.get_c());
                self.nop();
            }
            0x52 => {
                self.nop();
            }
            0x53 => {
                self.registers.set_d(self.registers.get_e());
                self.nop();
            }
            0x54 => {
                self.registers.set_d(self.registers.get_h());
                self.nop();
            }
            0x55 => {
                self.registers.set_d(self.registers.get_l());
                self.nop();
            }
            0x56 => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.nop();
                self.registers.set_d(value);
                self.nop();
            }
            0x57 => {
                self.registers.set_d(self.registers.get_a());
                self.nop();
            }
            0x58 => {
                self.registers.set_e(self.registers.get_b());
                self.nop();
            }
            0x59 => {
                self.registers.set_e(self.registers.get_c());
                self.nop();
            }
            0x5A => {
                self.registers.set_e(self.registers.get_d());
                self.nop();
            }
            0x5B => {
                self.nop();
            }
            0x5C => {
                self.registers.set_e(self.registers.get_h());
                self.nop();
            }
            0x5D => {
                self.registers.set_e(self.registers.get_l());
                self.nop();
            }
            0x5E => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.nop();
                self.registers.set_e(value);
                self.nop();
            }
            0x5F => {
                self.registers.set_e(self.registers.get_a());
                self.nop();
            }
            0x60 => {
                self.registers.set_h(self.registers.get_b());
                self.nop();
            }
            0x61 => {
                self.registers.set_h(self.registers.get_c());
                self.nop();
            }
            0x62 => {
                self.registers.set_h(self.registers.get_d());
                self.nop();
            }
            0x63 => {
                self.registers.set_h(self.registers.get_e());
                self.nop();
            }
            0x64 => {
                self.nop();
            }
            0x65 => {
                self.registers.set_h(self.registers.get_l());
                self.nop();
            }
            0x66 => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.nop();
                self.registers.set_h(value);
                self.nop();
            }
            0x67 => {
                self.registers.set_h(self.registers.get_a());
                self.nop();
            }
            0x68 => {
                self.registers.set_l(self.registers.get_b());
                self.nop();
            }
            0x69 => {
                self.registers.set_l(self.registers.get_c());
                self.nop();
            }
            0x6A => {
                self.registers.set_l(self.registers.get_d());
                self.nop();
            }
            0x6B => {
                self.registers.set_l(self.registers.get_e());
                self.nop();
            }
            0x6C => {
                self.registers.set_l(self.registers.get_h());
                self.nop();
            }
            0x6D => {
                self.nop();
            }
            0x6E => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.nop();
                self.registers.set_l(value);
                self.nop();
            }
            0x6F => {
                self.registers.set_l(self.registers.get_a());
                self.nop();
            }
            0x70 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_b());
                self.nop();
                self.nop();
            }
            0x71 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_c());
                self.nop();
                self.nop();
            }
            0x72 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_d());
                self.nop();
                self.nop();
            }
            0x73 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_e());
                self.nop();
                self.nop();
            }
            0x74 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_h());
                self.nop();
                self.nop();
            }
            0x75 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_l());
                self.nop();
                self.nop();
            }
            0x76 => {
                self.halted = true;
                self.nop();
            }
            0x77 => {
                let address = self.registers.get_hl();
                self.ram.borrow_mut().write(address, self.registers.get_a());
                self.nop();
                self.nop();
            }
            0x78 => {
                self.registers.set_a(self.registers.get_b());
                self.nop();
            }
            0x79 => {
                self.registers.set_a(self.registers.get_c());
                self.nop();
            }
            0x7A => {
                self.registers.set_a(self.registers.get_d());
                self.nop();
            }
            0x7B => {
                self.registers.set_a(self.registers.get_e());
                self.nop();
            }
            0x7C => {
                self.registers.set_a(self.registers.get_h());
                self.nop();
            }
            0x7D => {
                self.registers.set_a(self.registers.get_l());
                self.nop();
            }
            0x7E => {
                let address = self.registers.get_hl();
                let value = self.ram.borrow().read(address);
                self.nop();
                self.registers.set_a(value);
                self.nop();
            }
            0x7F => {
                self.nop();
            }
            0x80 => {
                self.add_8bit(self.registers.get_b());
            }
            0x81 => {
                self.add_8bit(self.registers.get_c());
            }
            0x82 => {
                self.add_8bit(self.registers.get_d());
            }
            0x83 => {
                self.add_8bit(self.registers.get_e());
            }
            0x84 => {
                self.add_8bit(self.registers.get_h());
            }
            0x85 => {
                self.add_8bit(self.registers.get_l());
            }
            0x86 => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.add_8bit(value);
                self.nop();
            }
            0x87 => {
                self.add_8bit(self.registers.get_a());
            }
            0x88 => {
                self.add_with_carry(self.registers.get_b());
            }
            0x89 => {
                self.add_with_carry(self.registers.get_c());
            }
            0x8A => {
                self.add_with_carry(self.registers.get_d());
            }
            0x8B => {
                self.add_with_carry(self.registers.get_e());
            }
            0x8C => {
                self.add_with_carry(self.registers.get_h());
            }
            0x8D => {
                self.add_with_carry(self.registers.get_l());
            }
            0x8E => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.add_with_carry(value);
                self.nop();
            }
            0x8F => {
                self.add_with_carry(self.registers.get_a());
            }
            0x90 => {
                self.sub_8bit(self.registers.get_b());
            }
            0x91 => {
                self.sub_8bit(self.registers.get_c());
            }
            0x92 => {
                self.sub_8bit(self.registers.get_d());
            }
            0x93 => {
                self.sub_8bit(self.registers.get_e());
            }
            0x94 => {
                self.sub_8bit(self.registers.get_h());
            }
            0x95 => {
                self.sub_8bit(self.registers.get_l());
            }
            0x96 => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.sub_8bit(value);
                self.nop();
            }
            0x97 => {
                self.sub_8bit(self.registers.get_a());
                let mut f = self.registers.get_f() | (FLAGS::Z as u8 | FLAGS::N as u8);
                f &= !(FLAGS::H as u8 | FLAGS::C as u8);
                self.registers.set_f(f);
            }
            0x98 => {
                self.sub_with_carry(self.registers.get_b());
            }
            0x99 => {
                self.sub_with_carry(self.registers.get_c());
            }
            0x9A => {
                self.sub_with_carry(self.registers.get_d());
            }
            0x9B => {
                self.sub_with_carry(self.registers.get_e());
            }
            0x9C => {
                self.sub_with_carry(self.registers.get_h());
            }
            0x9D => {
                self.sub_with_carry(self.registers.get_l());
            }
            0x9E => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.sub_with_carry(value);
                self.nop();
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
                self.nop();
            }
            0xA0 => {
                self.and(self.registers.get_b());
            }
            0xA1 => {
                self.and(self.registers.get_c());
            }
            0xA2 => {
                self.and(self.registers.get_d());
            }
            0xA3 => {
                self.and(self.registers.get_e());
            }
            0xA4 => {
                self.and(self.registers.get_h());
            }
            0xA5 => {
                self.and(self.registers.get_l());
            }
            0xA6 => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.and(value);
                self.nop();
            }
            0xA7 => {
                self.and(self.registers.get_a());
            }
            0xA8 => {
                self.xor(self.registers.get_b());
            }
            0xA9 => {
                self.xor(self.registers.get_c());
            }
            0xAA => {
                self.xor(self.registers.get_d());
            }
            0xAB => {
                self.xor(self.registers.get_e());
            }
            0xAC => {
                self.xor(self.registers.get_h());
            }
            0xAD => {
                self.xor(self.registers.get_l());
            }
            0xAE => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.xor(value);
                self.nop();
            }
            0xAF => {
                self.xor(self.registers.get_a());
                self.registers
                    .set_f(self.registers.get_f() | (FLAGS::Z as u8));
            }
            0xB0 => {
                self.or(self.registers.get_b());
            }
            0xB1 => {
                self.or(self.registers.get_c());
            }
            0xB2 => {
                self.or(self.registers.get_d());
            }
            0xB3 => {
                self.or(self.registers.get_e());
            }
            0xB4 => {
                self.or(self.registers.get_h());
            }
            0xB5 => {
                self.or(self.registers.get_l());
            }
            0xB6 => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.or(value);
                self.nop();
            }
            0xB7 => {
                self.or(self.registers.get_a());
            }
            0xB8 => {
                self.compare(self.registers.get_b());
            }
            0xB9 => {
                self.compare(self.registers.get_c());
            }
            0xBA => {
                self.compare(self.registers.get_d());
            }
            0xBB => {
                self.compare(self.registers.get_e());
            }
            0xBC => {
                self.compare(self.registers.get_h());
            }
            0xBD => {
                self.compare(self.registers.get_l());
            }
            0xBE => {
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.compare(value);
                self.nop();
            }
            0xBF => {
                self.compare(self.registers.get_a());
                let mut f = self.registers.get_f() & !(FLAGS::H as u8 | FLAGS::C as u8);
                f |= FLAGS::Z as u8 | FLAGS::N as u8;
                self.registers.set_f(f);
            }
            0xC0 => {
                self.nop();
                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    let lower_byte = self.ram.borrow().read(self.registers.get_sp());
                    self.nop();

                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    self.nop();
                    let upper_byte = self.ram.borrow().read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    self.nop();

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);

                    self.nop();
                } else {
                    self.nop();
                }
            }
            0xC1 => {
                let lower_byte = self.ram.borrow().read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop();

                let value = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_bc(value);

                self.nop();
            }
            0xC2 => {
                let lower_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    self.nop();
                    self.registers.set_pc(address);

                    self.nop();
                } else {
                    self.nop();
                }
            }
            0xC3 => {
                let lower_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.nop();
                self.registers.set_pc(address);
                self.nop();
            }
            0xC4 => {
                let lower_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.nop();
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );
                    self.nop();

                    self.registers.set_pc(address);

                    self.nop();
                } else {
                    self.nop();
                }
            }
            0xC5 => {
                let bc = self.registers.get_bc();
                self.nop();

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop();
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (bc >> 8) as u8);
                self.nop();

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (bc & 0xFF) as u8);
                self.nop();
            }
            0xC6 => {
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.add_8bit(data);
                self.nop();
            }
            0xC7 => {
                self.reset(0x00);
            }
            0xC8 => {
                self.nop();
                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    let lower_byte = self.ram.borrow().read(self.registers.get_sp());
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    self.nop();
                    let upper_byte = self.ram.borrow().read(self.registers.get_sp());
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);

                    self.nop();
                } else {
                    self.nop();
                }
            }
            0xC9 => {
                let lower_byte = self.ram.borrow().read(self.registers.get_sp());
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_sp());
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop();

                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_pc(address);
            }
            0xCA => {
                let lower_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    self.nop();
                    self.registers.set_pc(address);

                    self.nop();
                } else {
                    self.nop();
                }
            }
            0xCB => {
                let opcode = self.fetch();
                self.cb(opcode);
            }
            0xCC => {
                let lower_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.nop();
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.nop();
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );

                    self.registers.set_pc(address);

                    self.cycles += 24;
                } else {
                    self.nop();
                }
            }
            0xCD => {
                let lower_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop();
                self.ram.borrow_mut().write(
                    self.registers.get_sp(),
                    (self.registers.get_pc() >> 8) as u8,
                );
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop();
                self.ram.borrow_mut().write(
                    self.registers.get_sp(),
                    (self.registers.get_pc() & 0xFF) as u8,
                );

                self.registers.set_pc(address);

                self.nop();
            }
            0xCE => {
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.add_with_carry(data);
                self.nop();
            }
            0xCF => {
                self.reset(0x08);
            }
            0xD0 => {
                self.nop();
                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.nop();
                    let lower_byte = self.ram.borrow().read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    self.nop();
                    let upper_byte = self.ram.borrow().read(self.registers.get_sp());
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);
                    self.nop();
                } else {
                    self.nop();
                }
            }
            0xD1 => {
                let lower_byte = self.ram.borrow().read(self.registers.get_sp());
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop();

                let value = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_de(value);
            }
            0xD2 => {
                let lower_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.nop();
                    self.registers.set_pc(address);
                    self.nop();
                } else {
                    self.nop();
                }
            }
            0xD3 => {
                println!("D3 NOT VALID");
                self.nop();
            }
            0xD4 => {
                let lower_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.nop();
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );

                    self.registers.set_pc(address);

                    self.nop();
                } else {
                    self.nop();
                }
            }
            0xD5 => {
                let de = self.registers.get_de();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop();
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (de >> 8) as u8);
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop();
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (de & 0xFF) as u8);
                self.nop();
            }
            0xD6 => {
                let address = self.registers.get_and_inc_pc();
                let data = self.ram.borrow().read(address);
                self.sub_8bit(data);
                self.nop();
            }
            0xD7 => {
                self.reset(0x10);
            }
            0xD8 => {
                self.nop();
                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    let lower_byte = self.ram.borrow().read(self.registers.get_sp());
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    self.nop();
                    let upper_byte = self.ram.borrow().read(self.registers.get_sp());
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);

                    self.nop();
                } else {
                    self.nop();
                }
            }
            0xD9 => {
                let lower_byte = self.ram.borrow().read(self.registers.get_sp());
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_sp());
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));

                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_pc(address);

                self.ime = true;
                self.nop();
            }
            0xDA => {
                let lower = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let upper = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = ((upper as u16) << 8) | lower as u16;
                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    self.nop();
                    self.registers.set_pc(address);
                    self.nop();
                } else {
                    self.nop();
                }
            }
            0xDB => {
                println!("DB ILLEGAL");
                self.nop();
            }
            0xDC => {
                let lower_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.nop();
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.nop();
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.borrow_mut().write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );

                    self.registers.set_pc(address);

                    self.nop();
                } else {
                    self.nop();
                }
            }
            0xDD => {
                println!("ILLEGAL_DD");
                self.nop();
            }
            0xDE => {
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.sub_with_carry(data);
                self.nop();
            }
            0xDF => {
                self.reset(0x18);
            }
            0xE0 => {
                let offset = self.ram.borrow().read(self.registers.get_and_inc_pc());
                let address = (0xFF00 as u16).wrapping_add(offset as u16);
                self.nop();

                self.ram.borrow_mut().write(address, self.registers.get_a());
                self.nop();
                self.nop();
            }

            0xE1 => {
                let lower_byte = self.ram.borrow().read(self.registers.get_sp());
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                let upper_byte = self.ram.borrow().read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop();

                let value = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_hl(value);

                self.nop();
            }
            0xE2 => {
                let address = 0xff00 | self.registers.get_c() as u16;
                self.ram.borrow_mut().write(address, self.registers.get_a());
                self.nop();
                self.nop();
            }
            0xE3 => {
                print!("E3 is Illegal");
                self.nop();
            }
            0xE4 => {
                print!("E4 is Illegal");
                self.nop();
            }
            0xE5 => {
                let hl = self.registers.get_hl();

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop();
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (hl >> 8) as u8);
                self.nop();

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop();
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (hl & 0xFF) as u8);

                self.nop();
            }
            0xE6 => {
                let address = self.registers.get_and_inc_pc();
                let data = self.ram.borrow().read(address);
                self.and(data);
                self.nop();
            }
            0xE7 => {
                self.reset(0x20);
            }
            0xE8 => {
                let sp = self.registers.get_sp();
                self.nop();
                let imm = self.ram.borrow().read(self.registers.get_and_inc_pc()) as i8;
                self.nop();
                let result = (sp as i16).wrapping_add(imm as i16) as u16;
                self.nop();
                self.registers.set_sp(result);
                let mut f = 0;
                if ((sp & 0x0F) as i16 + ((imm as i16) & 0x0F)) > 0x0F {
                    f |= FLAGS::H as u8;
                }
                if ((sp & 0xFF) as i16 + ((imm as i16) & 0xFF)) > 0xFF {
                    f |= FLAGS::C as u8;
                }
                self.registers.set_f(f);
                self.nop();
            }
            0xE9 => {
                self.registers.set_pc(self.registers.get_hl());
                self.nop();
            }
            0xEA => {
                let lower = self.ram.borrow().read(self.registers.get_and_inc_pc());
                let upper = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = (upper as u16) << 8 | lower as u16;
                self.nop();
                self.ram.borrow_mut().write(address, self.registers.get_a());
                self.nop();
                self.nop();
            }
            0xEB => {
                println!("EB ILLEGAL");
                self.nop();
            }
            0xEC => {
                println!("EC ILLEGAL");
                self.nop();
            }
            0xED => {
                println!("ED ILLEGAL");
                self.nop();
            }
            0xEE => {
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.xor(data);
                self.nop();
            }
            0xEF => {
                self.reset(0x28);
            }
            0xF0 => {
                let offset = self.ram.borrow().read(self.registers.get_and_inc_pc());
                let address = 0xFF00u16.wrapping_add(offset as u16);
                self.nop();
                let value = self.ram.borrow().read(address);
                self.nop();
                self.registers.set_a(value);
                self.nop();
            }
            0xF1 => {
                let lower_byte = self.ram.borrow().read(self.registers.get_sp());
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop();
                let upper_byte = self.ram.borrow().read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.nop();

                let value = ((upper_byte as u16) << 8) | (lower_byte & 0xF0) as u16;
                self.registers.set_af(value);
            }
            0xF2 => {
                let address = 0xFF00 | self.registers.get_c() as u16;
                let data = self.ram.borrow().read(address);
                self.nop();
                self.registers.set_a(data);
                self.nop();
            }
            0xF3 => {
                self.ime = false;
                self.nop();
            }
            0xF4 => {
                println!("ILLEGAL F4");
                self.nop();
            }
            0xF5 => {
                let af = self.registers.get_af();
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.nop();
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (af >> 8) as u8);
                self.nop();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram
                    .borrow_mut()
                    .write(self.registers.get_sp(), (0xFF & af) as u8);

                self.nop();
            }
            0xF6 => {
                let data = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.or(data);
                self.nop();
            }
            0xF7 => {
                self.reset(0x30);
            }
            0xF8 => {
                let sp = self.registers.get_sp();
                self.nop();
                let imm = self.ram.borrow().read(self.registers.get_and_inc_pc()) as i8;
                self.nop();
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
                self.nop();
            }
            0xF9 => {
                let value = self.registers.get_hl();
                self.nop();
                self.registers.set_sp(value);
                self.nop();
            }
            0xFA => {
                let lower = self.ram.borrow().read(self.registers.get_and_inc_pc());
                let upper = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.nop();
                let address = ((upper as u16) << 8) | lower as u16;
                self.nop();
                self.registers.set_a(self.ram.borrow().read(address));
                self.nop();
                self.nop();
            }
            0xFB => {
                self.ime_queued = true;
                self.nop();
            }
            0xFC => {
                println!("FC ILLEGAL");
                self.nop();
            }
            0xFD => {
                println!("FD ILLEGAL");
                self.nop();
            }
            0xFE => {
                let n8 = self.ram.borrow().read(self.registers.get_and_inc_pc());
                self.compare(n8);
                self.nop();
            }
            0xFF => {
                self.reset(0x38);
            }
        }
    }
    fn cb(&mut self, opcode: u8) {
        match opcode {
            0x00 => {
                let data = self.rotate_without_carry(self.registers.get_b(), 0);
                self.registers.set_b(data);
                self.nop();
            }
            0x01 => {
                let data = self.rotate_without_carry(self.registers.get_c(), 0);
                self.registers.set_c(data);
                self.nop();
            }
            0x02 => {
                let data = self.rotate_without_carry(self.registers.get_d(), 0);
                self.registers.set_d(data);
                self.nop();
            }
            0x03 => {
                let data = self.rotate_without_carry(self.registers.get_e(), 0);
                self.registers.set_e(data);
                self.nop();
            }
            0x04 => {
                let data = self.rotate_without_carry(self.registers.get_h(), 0);
                self.registers.set_h(data);
                self.nop();
            }
            0x05 => {
                let data = self.rotate_without_carry(self.registers.get_l(), 0);
                self.registers.set_l(data);
                self.nop();
            }
            0x06 => {
                let address = self.registers.get_hl();
                self.nop();
                let value = self.ram.borrow().read(address);
                let data = self.rotate_without_carry(value, 0);
                self.ram.borrow_mut().write(address, data);
                self.nop();
                self.nop();
            }
            0x07 => {
                let data = self.rotate_without_carry(self.registers.get_a(), 0);
                self.registers.set_a(data);
                self.nop();
            }
            0x08 => {
                let data = self.rotate_without_carry(self.registers.get_b(), 1);
                self.registers.set_b(data);
                self.nop();
            }
            0x09 => {
                let data = self.rotate_without_carry(self.registers.get_c(), 1);
                self.registers.set_c(data);
                self.nop();
            }
            0x0A => {
                let data = self.rotate_without_carry(self.registers.get_d(), 1);
                self.registers.set_d(data);
                self.nop();
            }
            0x0B => {
                let data = self.rotate_without_carry(self.registers.get_e(), 1);
                self.registers.set_e(data);
                self.nop();
            }
            0x0C => {
                let data = self.rotate_without_carry(self.registers.get_h(), 1);
                self.registers.set_h(data);
                self.nop();
            }
            0x0D => {
                let data = self.rotate_without_carry(self.registers.get_l(), 1);
                self.registers.set_l(data);
                self.nop();
            }
            0x0E => {
                let address = self.registers.get_hl();
                self.nop();
                let value = self.ram.borrow().read(address);
                let data = self.rotate_without_carry(value, 1);
                self.ram.borrow_mut().write(address, data);
                self.nop();
                self.nop();
            }
            0x0F => {
                let data = self.rotate_without_carry(self.registers.get_a(), 1);
                self.registers.set_a(data);
                self.nop();
            }
            0x10 => {
                let data = self.rotate(self.registers.get_b(), 0);
                self.registers.set_b(data);
                self.nop();
            }
            0x11 => {
                let data = self.rotate(self.registers.get_c(), 0);
                self.registers.set_c(data);
                self.nop();
            }
            0x12 => {
                let data = self.rotate(self.registers.get_d(), 0);
                self.registers.set_d(data);
                self.nop();
            }
            0x13 => {
                let data = self.rotate(self.registers.get_e(), 0);
                self.registers.set_e(data);
                self.nop();
            }
            0x14 => {
                let data = self.rotate(self.registers.get_h(), 0);
                self.registers.set_h(data);
                self.nop();
            }
            0x15 => {
                let data = self.rotate(self.registers.get_l(), 0);
                self.registers.set_l(data);
                self.nop();
            }
            0x16 => {
                let address = self.registers.get_hl();
                self.nop();
                let value = self.ram.borrow().read(address);
                let data = self.rotate(value, 0);
                self.ram.borrow_mut().write(address, data);
                self.nop();
                self.nop();
            }
            0x17 => {
                let data = self.rotate(self.registers.get_a(), 0);
                self.registers.set_a(data);
                self.nop();
            }
            0x18 => {
                let data = self.rotate(self.registers.get_b(), 1);
                self.registers.set_b(data);
                self.nop();
            }
            0x19 => {
                let data = self.rotate(self.registers.get_c(), 1);
                self.registers.set_c(data);
                self.nop();
            }
            0x1A => {
                let data = self.rotate(self.registers.get_d(), 1);
                self.registers.set_d(data);
                self.nop();
            }
            0x1B => {
                let data = self.rotate(self.registers.get_e(), 1);
                self.registers.set_e(data);
                self.nop();
            }
            0x1C => {
                let data = self.rotate(self.registers.get_h(), 1);
                self.registers.set_h(data);
                self.nop();
            }
            0x1D => {
                let data = self.rotate(self.registers.get_l(), 1);
                self.registers.set_l(data);
                self.nop();
            }
            0x1E => {
                let address = self.registers.get_hl();
                self.nop();
                let value = self.ram.borrow().read(address);
                let data = self.rotate(value, 1);
                self.ram.borrow_mut().write(address, data);
                self.nop();
                self.nop();
            }
            0x1F => {
                let data = self.rotate(self.registers.get_a(), 1);
                self.registers.set_a(data);
                self.nop();
            }
            0x20 => {
                let data = self.shift(self.registers.get_b(), 0);
                self.registers.set_b(data);
                self.nop();
            }
            0x21 => {
                let data = self.shift(self.registers.get_c(), 0);
                self.registers.set_c(data);
                self.nop();
            }
            0x22 => {
                let data = self.shift(self.registers.get_d(), 0);
                self.registers.set_d(data);
                self.nop();
            }
            0x23 => {
                let data = self.shift(self.registers.get_e(), 0);
                self.registers.set_e(data);
                self.nop();
            }
            0x24 => {
                let data = self.shift(self.registers.get_h(), 0);
                self.registers.set_h(data);
                self.nop();
            }
            0x25 => {
                let data = self.shift(self.registers.get_l(), 0);
                self.registers.set_l(data);
                self.nop();
            }
            0x26 => {
                let address = self.registers.get_hl();
                self.nop();
                let value = self.ram.borrow().read(address);
                let data = self.shift(value, 0);
                self.ram.borrow_mut().write(address, data);
                self.nop();
                self.nop();
            }
            0x27 => {
                let data = self.shift(self.registers.get_a(), 0);
                self.registers.set_a(data);
                self.nop();
            }
            0x28 => {
                let data = self.shift(self.registers.get_b(), 1);
                self.registers.set_b(data);
                self.nop();
            }
            0x29 => {
                let data = self.shift(self.registers.get_c(), 1);
                self.registers.set_c(data);
                self.nop();
            }
            0x2A => {
                let data = self.shift(self.registers.get_d(), 1);
                self.registers.set_d(data);
                self.nop();
            }
            0x2B => {
                let data = self.shift(self.registers.get_e(), 1);
                self.registers.set_e(data);
                self.nop();
            }
            0x2C => {
                let data = self.shift(self.registers.get_h(), 1);
                self.registers.set_h(data);
                self.nop();
            }
            0x2D => {
                let data = self.shift(self.registers.get_l(), 1);
                self.registers.set_l(data);
                self.nop();
            }
            0x2E => {
                let address = self.registers.get_hl();
                self.nop();
                let value = self.ram.borrow().read(address);
                let data = self.shift(value, 1);
                self.ram.borrow_mut().write(address, data);
                self.nop();
                self.nop();
            }
            0x2F => {
                let data = self.shift(self.registers.get_a(), 1);
                self.registers.set_a(data);
                self.nop();
            }
            0x30 => {
                let data = self.swap(self.registers.get_b());
                self.registers.set_b(data);
                self.nop();
            }
            0x31 => {
                let data = self.swap(self.registers.get_c());
                self.registers.set_c(data);
                self.nop();
            }
            0x32 => {
                let data = self.swap(self.registers.get_d());
                self.registers.set_d(data);
                self.nop();
            }
            0x33 => {
                let data = self.swap(self.registers.get_e());
                self.registers.set_e(data);
                self.nop();
            }
            0x34 => {
                let data = self.swap(self.registers.get_h());
                self.registers.set_h(data);
                self.nop();
            }
            0x35 => {
                let data = self.swap(self.registers.get_l());
                self.registers.set_l(data);
                self.nop();
            }
            0x36 => {
                let address = self.registers.get_hl();
                self.nop();
                let value = self.ram.borrow().read(address);
                let data = self.swap(value);
                self.ram.borrow_mut().write(address, data);
                self.nop();
                self.nop();
            }
            0x37 => {
                let data = self.swap(self.registers.get_a());
                self.registers.set_a(data);
                self.nop();
            }
            0x38 => {
                let data = self.right_shift(self.registers.get_b());
                self.registers.set_b(data);
                self.nop();
            }
            0x39 => {
                let data = self.right_shift(self.registers.get_c());
                self.registers.set_c(data);
                self.nop();
            }
            0x3A => {
                let data = self.right_shift(self.registers.get_d());
                self.registers.set_d(data);
                self.nop();
            }
            0x3B => {
                let data = self.right_shift(self.registers.get_e());
                self.registers.set_e(data);
                self.nop();
            }
            0x3C => {
                let data = self.right_shift(self.registers.get_h());
                self.registers.set_h(data);
                self.nop();
            }
            0x3D => {
                let data = self.right_shift(self.registers.get_l());
                self.registers.set_l(data);
                self.nop();
            }
            0x3E => {
                let address = self.registers.get_hl();
                self.nop();
                let value = self.ram.borrow().read(address);
                let data = self.right_shift(value);
                self.ram.borrow_mut().write(address, data);
                self.nop();
                self.nop();
            }
            0x3F => {
                let data = self.right_shift(self.registers.get_a());
                self.registers.set_a(data);
                self.nop();
            }
            0x40 => self.bit(self.registers.get_b(), 0),
            0x41 => self.bit(self.registers.get_c(), 0),
            0x42 => self.bit(self.registers.get_d(), 0),
            0x43 => self.bit(self.registers.get_e(), 0),
            0x44 => self.bit(self.registers.get_h(), 0),
            0x45 => self.bit(self.registers.get_l(), 0),
            0x46 => {
                self.nop();
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.bit(value, 0);
            }
            0x47 => self.bit(self.registers.get_a(), 0),
            0x48 => self.bit(self.registers.get_b(), 1),
            0x49 => self.bit(self.registers.get_c(), 1),
            0x4A => self.bit(self.registers.get_d(), 1),
            0x4B => self.bit(self.registers.get_e(), 1),
            0x4C => self.bit(self.registers.get_h(), 1),
            0x4D => self.bit(self.registers.get_l(), 1),
            0x4E => {
                self.nop();
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.bit(value, 1);
            }
            0x4F => self.bit(self.registers.get_a(), 1),
            0x50 => self.bit(self.registers.get_b(), 2),
            0x51 => self.bit(self.registers.get_c(), 2),
            0x52 => self.bit(self.registers.get_d(), 2),
            0x53 => self.bit(self.registers.get_e(), 2),
            0x54 => self.bit(self.registers.get_h(), 2),
            0x55 => self.bit(self.registers.get_l(), 2),
            0x56 => {
                self.nop();
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.bit(value, 2);
            }
            0x57 => self.bit(self.registers.get_a(), 2),
            0x58 => self.bit(self.registers.get_b(), 3),
            0x59 => self.bit(self.registers.get_c(), 3),
            0x5A => self.bit(self.registers.get_d(), 3),
            0x5B => self.bit(self.registers.get_e(), 3),
            0x5C => self.bit(self.registers.get_h(), 3),
            0x5D => self.bit(self.registers.get_l(), 3),
            0x5E => {
                self.nop();
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.bit(value, 3);
            }
            0x5F => self.bit(self.registers.get_a(), 3),
            0x60 => self.bit(self.registers.get_b(), 4),
            0x61 => self.bit(self.registers.get_c(), 4),
            0x62 => self.bit(self.registers.get_d(), 4),
            0x63 => self.bit(self.registers.get_e(), 4),
            0x64 => self.bit(self.registers.get_h(), 4),
            0x65 => self.bit(self.registers.get_l(), 4),
            0x66 => {
                self.nop();
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.bit(value, 4);
            }
            0x67 => self.bit(self.registers.get_a(), 4),
            0x68 => self.bit(self.registers.get_b(), 5),
            0x69 => self.bit(self.registers.get_c(), 5),
            0x6A => self.bit(self.registers.get_d(), 5),
            0x6B => self.bit(self.registers.get_e(), 5),
            0x6C => self.bit(self.registers.get_h(), 5),
            0x6D => self.bit(self.registers.get_l(), 5),
            0x6E => {
                self.nop();
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.bit(value, 5);
            }
            0x6F => self.bit(self.registers.get_a(), 5),
            0x70 => self.bit(self.registers.get_b(), 6),
            0x71 => self.bit(self.registers.get_c(), 6),
            0x72 => self.bit(self.registers.get_d(), 6),
            0x73 => self.bit(self.registers.get_e(), 6),
            0x74 => self.bit(self.registers.get_h(), 6),
            0x75 => self.bit(self.registers.get_l(), 6),
            0x76 => {
                self.nop();
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.bit(value, 6);
            }
            0x77 => self.bit(self.registers.get_a(), 6),
            0x78 => self.bit(self.registers.get_b(), 7),
            0x79 => self.bit(self.registers.get_c(), 7),
            0x7A => self.bit(self.registers.get_d(), 7),
            0x7B => self.bit(self.registers.get_e(), 7),
            0x7C => self.bit(self.registers.get_h(), 7),
            0x7D => self.bit(self.registers.get_l(), 7),
            0x7E => {
                self.nop();
                let value = self.ram.borrow().read(self.registers.get_hl());
                self.bit(value, 7);
            }
            0x7F => self.bit(self.registers.get_a(), 7),

            0x80 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 0);
                self.registers.set_b(data);
                self.nop();
            }
            0x81 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 0);
                self.registers.set_c(data);
                self.nop();
            }
            0x82 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 0);
                self.registers.set_d(data);
                self.nop();
            }
            0x83 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 0);
                self.registers.set_e(data);
                self.nop();
            }
            0x84 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 0);
                self.registers.set_h(data);
                self.nop();
            }
            0x85 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 0);
                self.registers.set_l(data);
                self.nop();
            }
            0x86 => {
                let address = self.registers.get_hl();
                self.nop();
                let data = self.ram.borrow().read(address);
                self.nop();
                self.ram.borrow_mut().write(address, data & !(1 << 0));
                self.nop();
                self.nop();
            }
            0x87 => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 0);
                self.registers.set_a(data);
                self.nop();
            }
            0x88 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 1);
                self.registers.set_b(data);
                self.nop();
            }
            0x89 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 1);
                self.registers.set_c(data);
                self.nop();
            }
            0x8A => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 1);
                self.registers.set_d(data);
                self.nop();
            }
            0x8B => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 1);
                self.registers.set_e(data);
                self.nop();
            }
            0x8C => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 1);
                self.registers.set_h(data);
                self.nop();
            }
            0x8D => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 1);
                self.registers.set_l(data);
                self.nop();
            }
            0x8E => {
                let address = self.registers.get_hl();
                self.nop();
                let data = self.ram.borrow().read(address);
                self.nop();
                self.ram.borrow_mut().write(address, data & !(1 << 1));
                self.nop();
                self.nop();
            }
            0x8F => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 1);
                self.registers.set_a(data);
                self.nop();
            }
            0x90 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 2);
                self.registers.set_b(data);
                self.nop();
            }
            0x91 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 2);
                self.registers.set_c(data);
                self.nop();
            }
            0x92 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 2);
                self.registers.set_d(data);
                self.nop();
            }
            0x93 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 2);
                self.registers.set_e(data);
                self.nop();
            }
            0x94 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 2);
                self.registers.set_h(data);
                self.nop();
            }
            0x95 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 2);
                self.registers.set_l(data);
                self.nop();
            }
            0x96 => {
                let address = self.registers.get_hl();
                self.nop();
                let data = self.ram.borrow().read(address);
                self.nop();
                self.ram.borrow_mut().write(address, data & !(1 << 2));
                self.nop();
                self.nop();
            }
            0x97 => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 2);
                self.registers.set_a(data);
                self.nop();
            }
            0x98 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 3);
                self.registers.set_b(data);
                self.nop();
            }
            0x99 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 3);
                self.registers.set_c(data);
                self.nop();
            }
            0x9A => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 3);
                self.registers.set_d(data);
                self.nop();
            }
            0x9B => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 3);
                self.registers.set_e(data);
                self.nop();
            }
            0x9C => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 3);
                self.registers.set_h(data);
                self.nop();
            }
            0x9D => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 3);
                self.registers.set_l(data);
                self.nop();
            }
            0x9E => {
                let address = self.registers.get_hl();
                self.nop();
                let data = self.ram.borrow().read(address);
                self.nop();
                self.ram.borrow_mut().write(address, data & !(1 << 3));
                self.nop();
                self.nop();
            }
            0x9F => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 3);
                self.registers.set_a(data);
                self.nop();
            }
            0xA0 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 4);
                self.registers.set_b(data);
                self.nop();
            }
            0xA1 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 4);
                self.registers.set_c(data);
                self.nop();
            }
            0xA2 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 4);
                self.registers.set_d(data);
                self.nop();
            }
            0xA3 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 4);
                self.registers.set_e(data);
                self.nop();
            }
            0xA4 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 4);
                self.registers.set_h(data);
                self.nop();
            }
            0xA5 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 4);
                self.registers.set_l(data);
                self.nop();
            }
            0xA6 => {
                let address = self.registers.get_hl();
                self.nop();
                let data = self.ram.borrow().read(address);
                self.nop();
                self.ram.borrow_mut().write(address, data & !(1 << 4));
                self.nop();
                self.nop();
            }
            0xA7 => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 4);
                self.registers.set_a(data);
                self.nop();
            }
            0xA8 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 5);
                self.registers.set_b(data);
                self.nop();
            }
            0xA9 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 5);
                self.registers.set_c(data);
                self.nop();
            }
            0xAA => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 5);
                self.registers.set_d(data);
                self.nop();
            }
            0xAB => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 5);
                self.registers.set_e(data);
                self.nop();
            }
            0xAC => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 5);
                self.registers.set_h(data);
                self.nop();
            }
            0xAD => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 5);
                self.registers.set_l(data);
                self.nop();
            }
            0xAE => {
                let address = self.registers.get_hl();
                self.nop();
                let data = self.ram.borrow().read(address);
                self.nop();
                self.ram.borrow_mut().write(address, data & !(1 << 5));
                self.nop();
                self.nop();
            }
            0xAF => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 5);
                self.registers.set_a(data);
                self.nop();
            }
            0xB0 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 6);
                self.registers.set_b(data);
                self.nop();
            }
            0xB1 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 6);
                self.registers.set_c(data);
                self.nop();
            }
            0xB2 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 6);
                self.registers.set_d(data);
                self.nop();
            }
            0xB3 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 6);
                self.registers.set_e(data);
                self.nop();
            }
            0xB4 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 6);
                self.registers.set_h(data);
                self.nop();
            }
            0xB5 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 6);
                self.registers.set_l(data);
                self.nop();
            }
            0xB6 => {
                let address = self.registers.get_hl();
                self.nop();
                let data = self.ram.borrow().read(address);
                self.nop();
                self.ram.borrow_mut().write(address, data & !(1 << 6));
                self.nop();
                self.nop();
            }
            0xB7 => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 6);
                self.registers.set_a(data);
                self.nop();
            }
            0xB8 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 7);
                self.registers.set_b(data);
                self.nop();
            }
            0xB9 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 7);
                self.registers.set_c(data);
                self.nop();
            }
            0xBA => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 7);
                self.registers.set_d(data);
                self.nop();
            }
            0xBB => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 7);
                self.registers.set_e(data);
                self.nop();
            }
            0xBC => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 7);
                self.registers.set_h(data);
                self.nop();
            }
            0xBD => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 7);
                self.registers.set_l(data);
                self.nop();
            }
            0xBE => {
                let address = self.registers.get_hl();
                self.nop();
                let data = self.ram.borrow().read(address);
                self.nop();
                self.ram.borrow_mut().write(address, data & !(1 << 7));
                self.nop();
                self.nop();
            }
            0xBF => {
                let data = self.zero_bit_8bit(self.registers.get_a(), 7);
                self.registers.set_a(data);
                self.nop();
            }
            0xC0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 0);
                self.registers.set_b(data);
                self.nop();
            }
            0xC1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 0);
                self.registers.set_c(data);
                self.nop();
            }
            0xC2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 0);
                self.registers.set_d(data);
                self.nop();
            }
            0xC3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 0);
                self.registers.set_e(data);
                self.nop();
            }
            0xC4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 0);
                self.registers.set_h(data);
                self.nop();
            }
            0xC5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 0);
                self.registers.set_l(data);
                self.nop();
            }
            0xC6 => {
                let address = self.registers.get_hl();
                self.nop();
                let byte = self.ram.borrow().read(address);
                self.nop();
                let new_byte = byte | (1 << 0);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop();
                self.nop();
            }
            0xC7 => {
                let data = self.set_bit_8bit(self.registers.get_a(), 0);
                self.registers.set_a(data);
                self.nop();
            }
            0xC8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 1);
                self.registers.set_b(data);
                self.nop();
            }
            0xC9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 1);
                self.registers.set_c(data);
                self.nop();
            }
            0xCA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 1);
                self.registers.set_d(data);
                self.nop();
            }
            0xCB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 1);
                self.registers.set_e(data);
                self.nop();
            }
            0xCC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 1);
                self.registers.set_h(data);
                self.nop();
            }
            0xCD => {
                let data = self.set_bit_8bit(self.registers.get_l(), 1);
                self.registers.set_l(data);
                self.nop();
            }
            0xCE => {
                let address = self.registers.get_hl();
                self.nop();
                let byte = self.ram.borrow().read(address);
                self.nop();
                let new_byte = byte | (1 << 1);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop();
                self.nop();
            }
            0xCF => {
                let data = self.set_bit_8bit(self.registers.get_a(), 1);
                self.registers.set_a(data);
                self.nop();
            }
            0xD0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 2);
                self.registers.set_b(data);
                self.nop();
            }
            0xD1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 2);
                self.registers.set_c(data);
                self.nop();
            }
            0xD2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 2);
                self.registers.set_d(data);
                self.nop();
            }
            0xD3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 2);
                self.registers.set_e(data);
                self.nop();
            }
            0xD4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 2);
                self.registers.set_h(data);
                self.nop();
            }
            0xD5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 2);
                self.registers.set_l(data);
                self.nop();
            }
            0xD6 => {
                let address = self.registers.get_hl();
                self.nop();
                let byte = self.ram.borrow().read(address);
                self.nop();
                let new_byte = byte | (1 << 2);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop();
                self.nop();
            }
            0xD7 => {
                let data = self.set_bit_8bit(self.registers.get_a(), 2);
                self.registers.set_a(data);
                self.nop();
            }
            0xD8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 3);
                self.registers.set_b(data);
                self.nop();
            }
            0xD9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 3);
                self.registers.set_c(data);
                self.nop();
            }
            0xDA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 3);
                self.registers.set_d(data);
                self.nop();
            }
            0xDB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 3);
                self.registers.set_e(data);
                self.nop();
            }
            0xDC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 3);
                self.registers.set_h(data);
                self.nop();
            }
            0xDD => {
                let data = self.set_bit_8bit(self.registers.get_l(), 3);
                self.registers.set_l(data);
                self.nop();
            }
            0xDE => {
                let address = self.registers.get_hl();
                self.nop();
                let byte = self.ram.borrow().read(address);
                self.nop();
                let new_byte = byte | (1 << 3);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop();
                self.nop();
            }
            0xDF => {
                let data = self.set_bit_8bit(self.registers.get_a(), 3);
                self.registers.set_a(data);
                self.nop();
            }
            0xE0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 4);
                self.registers.set_b(data);
                self.nop();
            }
            0xE1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 4);
                self.registers.set_c(data);
                self.nop();
            }
            0xE2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 4);
                self.registers.set_d(data);
                self.nop();
            }
            0xE3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 4);
                self.registers.set_e(data);
                self.nop();
            }
            0xE4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 4);
                self.registers.set_h(data);
                self.nop();
            }
            0xE5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 4);
                self.registers.set_l(data);
                self.nop();
            }
            0xE6 => {
                let address = self.registers.get_hl();
                self.nop();
                let byte = self.ram.borrow().read(address);
                self.nop();
                let new_byte = byte | (1 << 4);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop();
                self.nop();
            }
            0xE7 => {
                let data = self.set_bit_8bit(self.registers.get_a(), 4);
                self.registers.set_a(data);
                self.nop();
            }
            0xE8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 5);
                self.registers.set_b(data);
                self.nop();
            }
            0xE9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 5);
                self.registers.set_c(data);
                self.nop();
            }
            0xEA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 5);
                self.registers.set_d(data);
                self.nop();
            }
            0xEB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 5);
                self.registers.set_e(data);
                self.nop();
            }
            0xEC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 5);
                self.registers.set_h(data);
                self.nop();
            }
            0xED => {
                let data = self.set_bit_8bit(self.registers.get_l(), 5);
                self.registers.set_l(data);
                self.nop();
            }
            0xEE => {
                let address = self.registers.get_hl();
                self.nop();
                let byte = self.ram.borrow().read(address);
                self.nop();
                let new_byte = byte | (1 << 5);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop();
                self.nop();
            }
            0xEF => {
                let data = self.set_bit_8bit(self.registers.get_a(), 5);
                self.registers.set_a(data);
                self.nop();
            }
            0xF0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 6);
                self.registers.set_b(data);
                self.nop();
            }
            0xF1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 6);
                self.registers.set_c(data);
                self.nop();
            }
            0xF2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 6);
                self.registers.set_d(data);
                self.nop();
            }
            0xF3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 6);
                self.registers.set_e(data);
                self.nop();
            }
            0xF4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 6);
                self.registers.set_h(data);
                self.nop();
            }
            0xF5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 6);
                self.registers.set_l(data);
                self.nop();
            }
            0xF6 => {
                let address = self.registers.get_hl();
                self.nop();
                let byte = self.ram.borrow().read(address);
                self.nop();
                let new_byte = byte | (1 << 6);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop();
                self.nop();
            }
            0xF7 => {
                let data = self.set_bit_8bit(self.registers.get_a(), 6);
                self.registers.set_a(data);
                self.nop();
            }
            0xF8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 7);
                self.registers.set_b(data);
                self.nop();
            }
            0xF9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 7);
                self.registers.set_c(data);
                self.nop();
            }
            0xFA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 7);
                self.registers.set_d(data);
                self.nop();
            }
            0xFB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 7);
                self.registers.set_e(data);
                self.nop();
            }
            0xFC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 7);
                self.registers.set_h(data);
                self.nop();
            }
            0xFD => {
                let data = self.set_bit_8bit(self.registers.get_l(), 7);
                self.registers.set_l(data);
                self.nop();
            }
            0xFE => {
                let address = self.registers.get_hl();
                self.nop();
                let byte = self.ram.borrow().read(address);
                self.nop();
                let new_byte = byte | (1 << 7);
                self.ram.borrow_mut().write(address, new_byte);
                self.nop();
                self.nop();
            }
            0xFF => {
                let data = self.set_bit_8bit(self.registers.get_a(), 7);
                self.registers.set_a(data);
                self.nop();
            }
        }
    }
}
