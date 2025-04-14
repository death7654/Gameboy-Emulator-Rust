const DISPLAY_HEIGHT: usize = 144;
const DISPLAY_WIDTH: usize = 160;

struct REGISTERS {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    pc: u16,
    sp: u16,
}
impl REGISTERS {
    pub fn new() -> REGISTERS {
        REGISTERS {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }
    fn get_a(&self) -> u8 {
        self.a
    }

    fn set_a(&mut self, value: u8) {
        self.a = value;
    }

    fn get_b(&self) -> u8 {
        self.b
    }

    fn set_b(&mut self, value: u8) {
        self.b = value;
    }

    fn get_c(&self) -> u8 {
        self.c
    }

    fn set_c(&mut self, value: u8) {
        self.c = value
    }
    fn get_d(&self) -> u8 {
        self.d
    }

    fn set_d(&mut self, value: u8) {
        self.d = value;
    }

    fn get_e(&self) -> u8 {
        self.e
    }

    fn set_e(&mut self, value: u8) {
        self.e = value;
    }

    fn get_f(&self) -> u8 {
        self.f
    }

    fn set_f(&mut self, value: u8) {
        self.f = value & 0xF0
    }
    fn get_h(&self) -> u8 {
        self.h
    }

    fn set_h(&mut self, value: u8) {
        self.h = value
    }
    fn get_l(&self) -> u8 {
        self.l
    }

    fn set_l(&mut self, value: u8) {
        self.l = value
    }
    fn get_pc(&self) -> u16 {
        self.pc
    }

    fn set_pc(&mut self, value: u16) {
        self.pc = value
    }
    fn get_sp(&self) -> u16 {
        self.sp
    }

    fn set_sp(&mut self, value: u16) {
        self.sp = value
    }

    //since b, c, d, e, h, l can act as one 16 bit instuction we need to add code for that
    fn get_bc(&self) -> u16 {
        let mut result: u16;
        let reg1 = self.b as u16;
        let reg2 = self.c as u16;
        result = reg1 << 8;
        result = result | reg2;
        result
    }
    fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }
    fn get_de(&self) -> u16 {
        let mut result: u16;
        let reg1 = self.d as u16;
        let reg2 = self.e as u16;
        result = reg1 << 8;
        result = result | reg2;
        result
    }
    fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }
    fn get_hl(&self) -> u16 {
        let mut result: u16;
        let reg1 = self.h as u16;
        let reg2 = self.l as u16;
        result = reg1 << 8;
        result = result | reg2;
        result
    }
    fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }
    fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | self.f as u16
    }
    fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0x00F0) as u8;
    }
    fn get_and_inc_pc(&mut self) -> u16 {
        let ret_pc = self.pc;
        self.pc += 1;
        ret_pc
    }

    fn inc_pc(&mut self) -> u16 {
        self.pc += 1;
        self.pc
    }
}
enum FLAGS {
    Z = 0b1000_0000,
    N = 0b0100_0000,
    H = 0b0010_0000,
    C = 0b0001_0000,
}

struct CPU<'a> {
    registers: REGISTERS,
    ime: bool, //interrupt master enable
    opcode: u8,
    cycles: u64,
    ram: &'a mut RAM,
}
impl<'a> CPU<'a> {
    fn new(ram: &'a mut RAM) -> Self {
        CPU {
            registers: REGISTERS::new(),
            ime: true,
            opcode: 0,
            cycles: 0,
            ram,
        }
    }
    fn fetch(&mut self) -> u8 {
        let opcode = self.ram.read(self.registers.pc);
        self.registers.pc += 1;
        opcode
    }
    fn execute(&mut self, opcode: u8) {
        match opcode {
            0x00 => {
                //NOP
                self.cycles += 4;
                return;
            }
            0x01 => {
                //Load 2 bytes into register BC
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let data = ((upper_byte as u16) << 8) | (lower_byte as u16);
                self.registers.set_bc(data);
                self.cycles += 12;
            }
            0x02 => {
                //load the data in a into the ram address found in bc
                let address = self.registers.get_bc();
                let data = self.registers.get_a();
                self.ram.write(address, data);
                self.cycles += 8;
            }
            0x03 => {
                //increment bc
                let bc = self.registers.get_bc();
                self.registers.set_bc((bc.wrapping_add(1)));
                self.cycles += 8;
            }
            0x04 => {
                let data  = self.increment_8_bit(self.registers.get_b());
                self.registers.set_b(data);
            }
            0x05 => {
                //decrement b
                let data  = self.decrement_8_bit(self.registers.get_b());
                self.registers.set_b(data);            
            }
            0x06 => {
                //load 1 byte into B
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_b(data);
                self.cycles += 8;
            }
            0x07 => {
                let a = self.registers.get_a();
                let msb = (a & 0b1000_0000) >> 7;
                let result = (a<<1) | msb;

                self.registers.set_a(result);

                if result == 0 {
                    self.registers.set_f(self.registers.get_f() | FLAGS::Z as u8);
                } else {
                    self.registers.set_f(self.registers.get_f() & !(FLAGS::Z as u8));
                }
            
                self.registers.set_f(self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8));
            
                if msb != 0 {
                    self.registers.set_f(self.registers.get_f() | FLAGS::C as u8);
                } else {
                    self.registers.set_f(self.registers.get_f() & !(FLAGS::C as u8));
                }                
                self.cycles +=8;
            }
            0x08 => {
                //load load sp into the address from ram
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                let sp = self.registers.get_sp();
                self.ram.write(address, (sp & 0x00FF) as u8);
                self.ram.write(address + 1, (sp >> 8) as u8);

                self.cycles += 20;
            }
            0x09 => {
                let hl = self.registers.get_hl();
                let bc = self.registers.get_bc();
                let result = hl.wrapping_add(bc);

                //Reset the N flag
                self.registers
                    .set_f(self.registers.get_f() & !(FLAGS::N as u8));

                //detect half carry
                if (hl & 0xFFF) + (bc & 0xFFF) > 0xFFF {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::H as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::H as u8));
                }

                //detect carry
                if hl > 0xFFFF - bc {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::C as u8); // Carry flag
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::C as u8));
                }

                self.registers.set_hl(result);
                self.cycles += 8;
            }
            0x0A => {
                let data = self.ram.read(self.registers.get_bc());
                self.registers.set_a(data);
                self.cycles += 8;
            }
            0x0B => {
                //decrement BC
                self.registers.set_bc(self.registers.get_bc().wrapping_sub(1));
                self.cycles += 8;
            }
            0x0C => {
                let data  = self.increment_8_bit(self.registers.get_c());
                self.registers.set_c(data);            }
            0x0D => {
                let data  = self.decrement_8_bit(self.registers.get_c());
                self.registers.set_c(data);
            }
            0x0E => {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_c(data);
                self.cycles += 8;
            }
            0x0F => {
                let mut a = self.registers.get_a();
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

                self.cycles += 4;
            }
            0x10 => {
                //not implemented, stop

                println!("CPU Stopped");
                self.cycles += 4;
            }
            0x11 => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());

                let data = ((upper_byte as u16) << 8) | lower_byte as u16;

                self.registers.set_de(data);
                self.cycles += 12;
            }
            0x12 => {
                let address = self.registers.get_de();
                let data = self.registers.get_a();

                self.ram.write(address, data);
                self.cycles += 8;
            }
            0x13 => {
                let value = self.registers.get_de();
                self.registers.set_de(value.wrapping_add(1));
                self.cycles += 8;
            }
            0x14 => {
                let data  = self.increment_8_bit(self.registers.get_d());
                self.registers.set_d(data);
            }
            0x15 => {
                let data  = self.decrement_8_bit(self.registers.get_d());
                self.registers.set_d(data);
            }
            0x16 => {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_d(data);
                self.cycles += 8;
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
                self.cycles += 4;
            }
            0x18 => {
                //relative jump
                let jump = self.ram.read(self.registers.get_and_inc_pc()) as i8;
                self.registers
                    .set_pc(self.registers.get_pc().wrapping_add(jump as u16));
                self.cycles += 12;
            }
            0x19 => {
                let hl = self.registers.get_hl();
                let de = self.registers.get_de();
                let result = hl.wrapping_add(de);

                self.registers.set_hl(result);

                //N
                self.registers
                    .set_f(self.registers.get_f() & !(FLAGS::N as u8));

                if (hl & 0xFFF) + (de & 0xFFF) > 0xFFF {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::H as u8); // Half-carry flag
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::H as u8));
                }

                if hl > 0xFFFF - de {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::C as u8); // Carry flag
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::C as u8));
                }

                self.cycles += 8
            }
            0x1A => {
                let data = self.ram.read(self.registers.get_de());
                self.registers.set_a(data);
                self.cycles += 8;
            }
            0x1B => {
                //decrement de
                self.registers
                    .set_de(self.registers.get_de().wrapping_sub(1));
                self.cycles += 8;
            }
            0x1C => {
                let data  = self.increment_8_bit(self.registers.get_e());
                self.registers.set_e(data);
            }
            0x1d =>
            {
                let data  = self.decrement_8_bit(self.registers.get_e());
                self.registers.set_e(data);
            }
            0x1E => {
                //load the next byte onto register E
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_e(data);
                self.cycles += 8;
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

                self.cycles += 4;
            }
            0x20 => {
                let jump = self.ram.read(self.registers.get_and_inc_pc()) as i8;
                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(jump as i16 as u16));
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
            }
            0x21 => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());

                let data = ((upper_byte as u16) << 8) | lower_byte as u16;

                self.registers.set_hl(data);
                self.cycles += 12;
            }
            0x22 => {
                //load a into memory with the address found in HL and increment HL by 1
                let data = self.registers.get_a();
                let address = self.registers.get_hl();
                self.ram.write(address, data);

                self.cycles += 8;
            }
            0x23 => {
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_add(1));
                self.cycles += 8;
            }
            0x24 => {
                let data  = self.increment_8_bit(self.registers.get_h());
                self.registers.set_h(data);
            }
            0x25 => {
                let data  = self.decrement_8_bit(self.registers.get_h());
                self.registers.set_h(data);
            }
            0x26 => {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_h(data);
                self.cycles += 8;
            }
            0x27 => {
                let mut a = self.registers.get_a();
                let mut adjust = 0;
                let carry_flag = self.registers.get_f() & FLAGS::C as u8 != 0;
                let half_carry_flag = self.registers.get_f() & FLAGS::H as u8 != 0;
                let subtract_flag = self.registers.get_f() & FLAGS::N as u8 != 0;

                if !subtract_flag {
                    if a > 0x99 || carry_flag {
                        adjust |= 0x60;
                        self.registers
                            .set_f(self.registers.get_f() | FLAGS::C as u8);
                    }
                    if (a & 0x0F) > 0x09 || half_carry_flag {
                        adjust |= 0x06;
                    }
                } else {
                    if carry_flag {
                        adjust |= 0x60;
                    }
                    if half_carry_flag {
                        adjust |= 0x06;
                    }
                }

                a = a.wrapping_add(adjust);
                self.registers.set_a(a);

                // Update Z flag (if result is zero)
                if a == 0 {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::Z as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::Z as u8));
                }

                // Clear H flag
                self.registers
                    .set_f(self.registers.get_f() & !(FLAGS::H as u8));

                self.cycles += 4;
            }
            0x28 => {
                let offset = self.ram.read(self.registers.get_and_inc_pc()) as i8;
                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(offset as i16 as u16));
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
            }
            0x29 => {
                let hl = self.registers.get_hl();
                let result = hl.wrapping_add(hl);

                self.registers.set_hl(result);

                self.registers
                    .set_f(self.registers.get_f() & !(FLAGS::N as u8));

                if (hl & 0xFFF) + (hl & 0xFFF) > 0xFFF {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::H as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::H as u8));
                }

                // C flag: Set if a carry occurs from bit 15 to bit 16
                if hl > 0xFFFF - hl {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::C as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::C as u8));
                }

                self.registers.set_hl(result);
                self.cycles += 8;
            }
            0x2A => {
                let data = self.ram.read(self.registers.get_hl());
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_add(1));
                self.registers.set_a(data);
                self.cycles += 8;
            }
            0x2B => {
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_sub(1));
                self.cycles += 8;
            }
            0x2C => {
                let data  = self.increment_8_bit(self.registers.get_l());
                self.registers.set_l(data);
            }
            0x2D => {
                let data  = self.decrement_8_bit(self.registers.get_l());
                self.registers.set_l(data);
            }
            0x2E => {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_l(data);
                self.cycles += 8;
            }
            0x2F => {
                let a = self.registers.get_a();
                self.registers.set_a(!a); // Bitwise complement
            
                // Set N and H flags
                self.registers.set_f(self.registers.get_f() | FLAGS::N as u8 | FLAGS::H as u8);
            
                self.cycles += 4;
            }
            0x30 => {
                let offset = self.ram.read(self.registers.get_and_inc_pc()) as i8;
                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.registers.set_pc(self.registers.get_pc().wrapping_add(offset as i16 as u16));
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
            }
            0x31 => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let data = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_sp(data);
                self.cycles += 12;
            }
            0x32 => {
                let data = self.registers.get_a();
                self.ram.write(self.registers.get_hl(), data);
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_sub(1));

                self.cycles += 8;
            }
            0x33 => {
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                self.cycles += 8;
            }
            0x34 => {
                let address = self.registers.get_hl();
                let value = self.ram.read(address);
                let result = value.wrapping_add(1);
                self.ram.write(address, result);
            
                if result == 0 {
                    self.registers.set_f(self.registers.get_f() | FLAGS::Z as u8);
                } else {
                    self.registers.set_f(self.registers.get_f() & !(FLAGS::Z as u8));
                }
            
                self.registers.set_f(self.registers.get_f() & !(FLAGS::N as u8));
            
                if (value & 0x0F) + 1 > 0x0F {
                    self.registers.set_f(self.registers.get_f() | FLAGS::H as u8);
                } else {
                    self.registers.set_f(self.registers.get_f() & !(FLAGS::H as u8));
                }
            
                self.cycles += 12;
            }
            0x35 => {
                let address = self.registers.get_hl();
                let value = self.ram.read(address);
                let result = value.wrapping_sub(1);
                self.ram.write(address, result);

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
                self.cycles += 12;
            }
            0x36 => {
                let address = self.registers.get_hl();
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.ram.write(address, data);
                self.cycles += 12;
            }
            0x37 => {

                self.registers.set_f(self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8));

                self.registers.set_f(self.registers.get_f() | FLAGS::C as u8);

                self.cycles += 4;
            }
            0x38 => {
                let offset = self.ram.read(self.registers.get_and_inc_pc()) as i8;
                if self.registers.get_f() & FLAGS::C as u8 != 0
                {
                    self.registers.set_pc(self.registers.get_pc().wrapping_add(offset as i16 as u16));
                    self.cycles +=12;
                }
                else {
                    self.cycles +=8;
                }
            }
            0x39 => {
                let hl = self.registers.get_hl();
                let sp = self.registers.get_sp();
                let result = hl.wrapping_add(sp);
                self.registers.set_hl(result);

                if (hl & 0xFFF) + (sp & 0xFFF) > 0xFFF {
                    self.registers.set_f(self.registers.get_f() | FLAGS::H as u8);
                } else {
                    self.registers.set_f(self.registers.get_f() & !(FLAGS::H as u8));
                }
            
                // Set C flag if carry occurs from bit 15 to bit 16
                if hl > 0xFFFF - sp {
                    self.registers.set_f(self.registers.get_f() | FLAGS::C as u8);
                } else {
                    self.registers.set_f(self.registers.get_f() & !(FLAGS::C as u8));
                }

                self.cycles += 8;
            }
            0x3A => {
                let data = self.ram.read(self.registers.get_hl());
                self.registers
                    .set_hl(self.registers.get_hl().wrapping_sub(1));

                self.registers.set_a(data);
                self.cycles += 8;
            }
            0x3B => {
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.cycles += 8;
            }
            0x3C => {
                let data  = self.increment_8_bit(self.registers.get_a());
                self.registers.set_a(data);
            }
            0x3D => {
                let data  = self.decrement_8_bit(self.registers.get_a());
                self.registers.set_a(data);
            }
            0x3E => {
                let value = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_a(value);
                self.cycles += 8;
            }
            0x3F => {
                self.registers.set_f(self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8));

                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    self.registers.set_f(self.registers.get_f() & !(FLAGS::C as u8)); // Clear Carry
                } else {
                    self.registers.set_f(self.registers.get_f() | FLAGS::C as u8); // Set Carry
                }
                self.cycles +=4;
            }
            0x40 => {
                self.cycles += 4;
            }
            0x41 => {
                self.registers.set_b(self.registers.get_c());
                self.cycles += 4;
            }
            0x42 => {
                self.registers.set_b(self.registers.get_d());
                self.cycles += 4;
            }
            0x43 => {
                self.registers.set_b(self.registers.get_e());
                self.cycles += 4;
            }
            0x44 => {
                self.registers.set_b(self.registers.get_h());
                self.cycles += 4;
            }
            0x45 => {
                self.registers.set_b(self.registers.get_l());
                self.cycles += 4;
            }
            0x46 => {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_b(value);
                self.cycles += 4;
            }
            0x47 => {
                self.registers.set_b(self.registers.get_a());
                self.cycles += 4;
            }
            0x48 => {
                self.registers.set_c(self.registers.get_b());
                self.cycles += 4;
            }
            0x49 => {
                self.cycles += 4;
            }
            0x4A => {
                self.registers.set_c(self.registers.get_d());
                self.cycles += 4;
            }
            0x4B => {
                self.registers.set_c(self.registers.get_e());
                self.cycles += 4;
            }
            0x4C => {
                self.registers.set_c(self.registers.get_h());
                self.cycles += 4;
            }
            0x4D => {
                self.registers.set_c(self.registers.get_l());
                self.cycles += 4;
            }
            0x4E => {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_c(value);
                self.cycles += 8;
            }
            0x4F => {
                self.registers.set_c(self.registers.get_a());
                self.cycles += 4;
            }
            0x50 => {
                self.registers.set_d(self.registers.get_b());
                self.cycles += 4;
            }
            0x51 => {
                self.registers.set_d(self.registers.get_c());
                self.cycles += 4;
            }
            0x52 => {
                self.cycles += 4;
            }
            0x53 => {
                self.registers.set_d(self.registers.get_e());
                self.cycles += 4;
            }
            0x54 => {
                self.registers.set_d(self.registers.get_h());
                self.cycles += 4;
            }
            0x55 => {
                self.registers.set_d(self.registers.get_l());
                self.cycles += 4;
            }
            0x56 => {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_d(value);
                self.cycles += 8;
            }
            0x57 => {
                self.registers.set_d(self.registers.get_a());
                self.cycles += 4;
            }
            0x58 => {
                self.registers.set_e(self.registers.get_b());
                self.cycles += 4;
            }
            0x59 => {
                self.registers.set_e(self.registers.get_c());
                self.cycles += 4;
            }
            0x5A => {
                self.registers.set_e(self.registers.get_d());
                self.cycles += 4;
            }
            0x5B => {
                self.cycles += 4;
            }
            0x5C => {
                self.registers.set_e(self.registers.get_h());
                self.cycles += 4;
            }
            0x5D => {
                self.registers.set_e(self.registers.get_l());
                self.cycles += 4;
            }
            0x5E => {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_e(value);
                self.cycles += 8;
            }
            0x5F => {
                self.registers.set_e(self.registers.get_a());
                self.cycles += 4;
            }
            0x60 => {
                self.registers.set_h(self.registers.get_b());
                self.cycles += 4;
            }
            0x61 => {
                self.registers.set_h(self.registers.get_c());
                self.cycles += 4;
            }
            0x62 => {
                self.registers.set_h(self.registers.get_d());
                self.cycles += 4;
            }
            0x63 => {
                self.registers.set_h(self.registers.get_e());
                self.cycles += 4;
            }
            0x64 => {
                self.cycles += 4;
            }
            0x65 => {
                self.registers.set_h(self.registers.get_l());
                self.cycles += 4;
            }
            0x66 => {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_h(value);
                self.cycles += 8;
            }
            0x67 => {
                self.registers.set_h(self.registers.get_a());
                self.cycles += 4;
            }
            0x68 => {
                self.registers.set_l(self.registers.get_b());
                self.cycles += 4;
            }
            0x69 => {
                self.registers.set_l(self.registers.get_c());
                self.cycles += 4;
            }
            0x6A => {
                self.registers.set_l(self.registers.get_d());
                self.cycles += 4;
            }
            0x6B => {
                self.registers.set_l(self.registers.get_e());
                self.cycles += 4;
            }
            0x6C => {
                self.registers.set_l(self.registers.get_h());
                self.cycles += 4;
            }
            0x6D => {
                self.cycles += 4;
            }
            0x6E => {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_l(value);
                self.cycles += 8;
            }
            0x6F => {
                self.registers.set_l(self.registers.get_a());
                self.cycles += 4;
            }
            0x70 => {
                self.ram
                    .write(self.registers.get_hl(), self.registers.get_b());
                self.cycles += 8;
            }
            0x71 => {
                self.ram
                    .write(self.registers.get_hl(), self.registers.get_c());
                self.cycles += 8;
            }
            0x72 => {
                self.ram
                    .write(self.registers.get_hl(), self.registers.get_d());
                self.cycles += 8;
            }
            0x73 => {
                self.ram
                    .write(self.registers.get_hl(), self.registers.get_e());
                self.cycles += 8;
            }
            0x74 => {
                self.ram
                    .write(self.registers.get_hl(), self.registers.get_h());
                self.cycles += 8;
            }
            0x75 => {
                self.ram
                    .write(self.registers.get_hl(), self.registers.get_l());
                self.cycles += 8;
            }
            0x76 => {
                //halt, not implemented
            }
            0x77 => {
                self.ram
                    .write(self.registers.get_hl(), self.registers.get_a());
                self.cycles += 8;
            }
            0x78 => {
                self.registers.set_a(self.registers.get_b());
                self.cycles += 4;
            }
            0x79 => {
                self.registers.set_a(self.registers.get_c());
                self.cycles += 4;
            }
            0x7A => {
                self.registers.set_a(self.registers.get_d());
                self.cycles += 4;
            }
            0x7B => {
                self.registers.set_a(self.registers.get_e());
                self.cycles += 4;
            }
            0x7C => {
                self.registers.set_a(self.registers.get_h());
                self.cycles += 4;
            }
            0x7D => {
                self.registers.set_a(self.registers.get_l());
                self.cycles += 4;
            }
            0x7E => {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_a(value);
                self.cycles += 8;
            }
            0x7F => {
                self.cycles += 4;
            }
            0x80 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 0);
                self.registers.set_b(data);
            }
            0x81 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 0);
                self.registers.set_c(data);
            }
            0x82 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 0);
                self.registers.set_d(data);
            }
            0x83 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 0);
                self.registers.set_e(data);
            }
            0x84 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_h(), 0);
                self.registers.set_h(data);
            }
            0x85 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_l(), 0);
                self.registers.set_l(data);
            }
            0x86 =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & !(1 << 0)); 
                self.cycles += 16;
            }
            0x87 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_a(), 0);
                self.registers.set_a(data);
            }
            0x88 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 1);
                self.registers.set_b(data);
            }
            0x89 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 1);
                self.registers.set_c(data);
            }
            0x8A => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 1);
                self.registers.set_d(data);
            }
            0x8B => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 1);
                self.registers.set_e(data);
            }
            0x8C =>
            {
                let data = self.zero_bit_8bit(self.registers.get_h(), 1);
                self.registers.set_h(data);
            }
            0x8D =>
            {
                let data = self.zero_bit_8bit(self.registers.get_l(), 1);
                self.registers.set_l(data);
            }
            0x8E =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & !(1 << 1)); 
                self.cycles += 16;
            }
            0x8F =>
            {
                let data = self.zero_bit_8bit(self.registers.get_a(), 1);
                self.registers.set_a(data);
            }
            0x90 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 2);
                self.registers.set_b(data);
            }
            0x91 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 2);
                self.registers.set_c(data);
            }
            0x92 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 2);
                self.registers.set_d(data);
            }
            0x93 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 2);
                self.registers.set_e(data);
            }
            0x94 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_h(), 2);
                self.registers.set_h(data);
            }
            0x95 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_l(), 2);
                self.registers.set_l(data);
            }
            0x96 =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & !(1 << 2)); 
                self.cycles += 16;
            }
            0x97 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_a(), 2);
                self.registers.set_a(data);
            }
            0x98 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 3);
                self.registers.set_b(data);
            }
            0x99 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 3);
                self.registers.set_c(data);
            }
            0x9A => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 3);
                self.registers.set_d(data);
            }
            0x9B => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 3);
                self.registers.set_e(data);
            }
            0x9C =>
            {
                let data = self.zero_bit_8bit(self.registers.get_h(), 3);
                self.registers.set_h(data);
            }
            0x9D =>
            {
                let data = self.zero_bit_8bit(self.registers.get_l(), 3);
                self.registers.set_l(data);
            }
            0x9E =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & !(1 << 3)); 
                self.cycles += 16;
            }
            0x9F =>
            {
                let data = self.zero_bit_8bit(self.registers.get_a(), 3);
                self.registers.set_a(data);
            }
            0xA0 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 4);
                self.registers.set_b(data);
            }
            0xA1 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 4);
                self.registers.set_c(data);
            }
            0xA2 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 4);
                self.registers.set_d(data);
            }
            0xA3 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 4);
                self.registers.set_e(data);
            }
            0xA4 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_h(), 4);
                self.registers.set_h(data);
            }
            0xA5 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_l(), 4);
                self.registers.set_l(data);
            }
            0xA6 =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & !(1 << 4)); 
                self.cycles += 16;
            }
            0xA7 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_a(), 4);
                self.registers.set_a(data);
            }
            0xA8 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 5);
                self.registers.set_b(data);
            }
            0xA9 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 5);
                self.registers.set_c(data);
            }
            0xAA => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 5);
                self.registers.set_d(data);
            }
            0xAB => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 5);
                self.registers.set_e(data);
            }
            0xAC =>
            {
                let data = self.zero_bit_8bit(self.registers.get_h(), 5);
                self.registers.set_h(data);
            }
            0xAD =>
            {
                let data = self.zero_bit_8bit(self.registers.get_l(), 5);
                self.registers.set_l(data);
            }
            0xAE =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & !(1 << 5)); 
                self.cycles += 16;
            }
            0xAF =>
            {
                let data = self.zero_bit_8bit(self.registers.get_a(), 5);
                self.registers.set_a(data);
            }
            0xB0 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 6);
                self.registers.set_b(data);
            }
            0xB1 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 6);
                self.registers.set_c(data);
            }
            0xB2 => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 6);
                self.registers.set_d(data);
            }
            0xB3 => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 6);
                self.registers.set_e(data);
            }
            0xB4 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_h(), 6);
                self.registers.set_h(data);
            }
            0xB5 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_l(), 6);
                self.registers.set_l(data);
            }
            0xB6 =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & !(1 << 6)); 
                self.cycles += 16;
            }
            0xB7 =>
            {
                let data = self.zero_bit_8bit(self.registers.get_a(), 6);
                self.registers.set_a(data);        
            }
            0xB8 => {
                let data = self.zero_bit_8bit(self.registers.get_b(), 7);
                self.registers.set_b(data);
            }
            0xB9 => {
                let data = self.zero_bit_8bit(self.registers.get_c(), 7);
                self.registers.set_c(data);
            }
            0xBA => {
                let data = self.zero_bit_8bit(self.registers.get_d(), 7);
                self.registers.set_d(data);
            }
            0xBB => {
                let data = self.zero_bit_8bit(self.registers.get_e(), 7);
                self.registers.set_e(data);
            }
            0xBC =>
            {
                let data = self.zero_bit_8bit(self.registers.get_h(), 7);
                self.registers.set_h(data);;
            }
            0xBD =>
            {
                let data = self.zero_bit_8bit(self.registers.get_l(), 7);
                self.registers.set_l(data);
            }
            0xBE =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & !(1 << 7)); 
                self.cycles += 16;
            }
            0xBF =>
            {
                let data = self.zero_bit_8bit(self.registers.get_a(), 7);
                self.registers.set_a(data);         
            }
            0xC0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 0);
                self.registers.set_b(data);
            }
            0xC1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 0);
                self.registers.set_c(data);
            }
            0xC2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 0);
                self.registers.set_d(data);
            }
            0xC3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 0);
                self.registers.set_e(data);
            }
            0xC4 =>
            {
                let data = self.set_bit_8bit(self.registers.get_h(), 0);
                self.registers.set_h(data);
            }
            0xC5 =>
            {
                let data = self.set_bit_8bit(self.registers.get_l(), 0);
                self.registers.set_l(data);
            }
            0xC6 =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & (1 << 0)); 
                self.cycles += 16;
            }
            0xC7 =>
            {
                let data = self.set_bit_8bit(self.registers.get_a(), 0);
                self.registers.set_a(data);       
            }
            0xC8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 1);
                self.registers.set_b(data);
            }
            0xC9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 1);
                self.registers.set_c(data);
            }
            0xCA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 1);
                self.registers.set_d(data);
            }
            0xCB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 1);
                self.registers.set_e(data);
            }
            0xCC =>
            {
                let data = self.set_bit_8bit(self.registers.get_h(), 1);
                self.registers.set_h(data);
            }
            0xCD =>
            {
                let data = self.set_bit_8bit(self.registers.get_l(), 1);
                self.registers.set_l(data);
            }
            0xCE =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & (1 << 1)); 
                self.cycles += 16;
            }
            0xCF =>
            {
                let data = self.set_bit_8bit(self.registers.get_a(), 1);
                self.registers.set_a(data);        
            }
            0xD0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 2);
                self.registers.set_b(data);
            }
            0xD1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 2);
                self.registers.set_c(data);
            }
            0xD2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 2);
                self.registers.set_d(data);
            }
            0xD3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 2);
                self.registers.set_e(data);
            }
            0xD4 =>
            {
                let data = self.set_bit_8bit(self.registers.get_h(), 2);
                self.registers.set_h(data);
            }
            0xD5 =>
            {
                let data = self.set_bit_8bit(self.registers.get_l(), 2);
                self.registers.set_l(data);
            }
            0xD6 =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & (1 << 2)); 
                self.cycles += 16;
            }
            0xD7 =>
            {
                let data = self.set_bit_8bit(self.registers.get_a(), 2);
                self.registers.set_a(data);       
            }
            0xD8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 3);
                self.registers.set_b(data);
            }
            0xD9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 3);
                self.registers.set_c(data);
            }
            0xDA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 3);
                self.registers.set_d(data);
            }
            0xDB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 3);
                self.registers.set_e(data);
            }
            0xDC =>
            {
                let data = self.set_bit_8bit(self.registers.get_h(), 3);
                self.registers.set_h(data);
            }
            0xDD =>
            {
                let data = self.set_bit_8bit(self.registers.get_l(), 3);
                self.registers.set_l(data);
            }
            0xDE =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & (1 << 3)); 
                self.cycles += 16;
            }
            0xDF =>
            {
                let data = self.set_bit_8bit(self.registers.get_a(), 3);
                self.registers.set_a(data);         
            }
            0xE0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 4);
                self.registers.set_b(data);
            }
            0xE1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 4);
                self.registers.set_c(data);
            }
            0xE2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 4);
                self.registers.set_d(data);
            }
            0xE3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 4);
                self.registers.set_e(data);
            }
            0xE4 =>
            {
                let data = self.set_bit_8bit(self.registers.get_h(), 4);
                self.registers.set_h(data);
            }
            0xE5 =>
            {
                let data = self.set_bit_8bit(self.registers.get_l(), 4);
                self.registers.set_l(data);
            }
            0xE6 =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & (1 << 4)); 
                self.cycles += 16;
            }
            0xE7 =>
            {
                let data = self.set_bit_8bit(self.registers.get_a(), 4);
                self.registers.set_a(data);         
            }
            0xE8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 5);
                self.registers.set_b(data); 
            }
            0xE9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 5);
                self.registers.set_c(data);
            }
            0xEA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 5);
                self.registers.set_d(data);
            }
            0xEB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 5);
                self.registers.set_e(data);
            }
            0xEC =>
            {
                let data = self.set_bit_8bit(self.registers.get_h(), 5);
                self.registers.set_h(data);
            }
            0xED =>
            {
                let data = self.set_bit_8bit(self.registers.get_l(), 5);
                self.registers.set_l(data);
            }
            0xEE =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & (1 <<5)); 
                self.cycles += 16;
            }
            0xEF =>
            {
                let data = self.set_bit_8bit(self.registers.get_a(), 5);
                self.registers.set_a(data);         
            }
            0xF0 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 6);
                self.registers.set_b(data);
            }
            0xF1 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 6);
                self.registers.set_c(data);
            }
            0xF2 => {
                let data = self.set_bit_8bit(self.registers.get_d(), 6);
                self.registers.set_d(data);
            }
            0xF3 => {
                let data = self.set_bit_8bit(self.registers.get_e(), 6);
                self.registers.set_e(data);
            }
            0xF4 =>
            {
                let data = self.set_bit_8bit(self.registers.get_h(), 6);
                self.registers.set_h(data);
            }
            0xF5 =>
            {
                let data = self.set_bit_8bit(self.registers.get_l(), 6);
                self.registers.set_l(data);
            }
            0xF6 =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & (1 << 6)); 
                self.cycles += 16;
            }
            0xF7 =>
            {
                let data = self.set_bit_8bit(self.registers.get_a(), 6);
                self.registers.set_a(data);         
            }
            0xF8 => {
                let data = self.set_bit_8bit(self.registers.get_b(), 7);
                self.registers.set_b(data);
            }
            0xF9 => {
                let data = self.set_bit_8bit(self.registers.get_c(), 7);
                self.registers.set_c(data);
            }
            0xFA => {
                let data = self.set_bit_8bit(self.registers.get_d(), 7);
                self.registers.set_d(data);
            }
            0xFB => {
                let data = self.set_bit_8bit(self.registers.get_e(), 7);
                self.registers.set_e(data);
            }
            0xFC =>
            {
                let data = self.set_bit_8bit(self.registers.get_h(), 7);
                self.registers.set_h(data);
            }
            0xFD =>
            {
                let data = self.set_bit_8bit(self.registers.get_l(), 7);
                self.registers.set_l(data);
            }
            0xFE =>
            {
                let data = self.registers.get_hl();
                self.registers.set_hl(data & (1 << 7)); 
                self.cycles += 16;
            }
            0xFF =>
            {
                let data = self.set_bit_8bit(self.registers.get_a(), 7);
                self.registers.set_a(data);         
            }

            _ => println!("{} Not Implemented", opcode),
        }

    }

    fn increment_8_bit(&mut self, data: u8) -> u8
    {
        let result = data.wrapping_add(1);

        
        let mut f = self.registers.get_f();

        //setting Z flag
        if result == 0
        {
            f |= FLAGS::Z as u8;
        }
        else {
            f&= !(FLAGS::Z as u8);
        }

        //setting N flag

        f &= !(FLAGS::N as u8);

        //setting H flag

        if(data & 0x0F) + 1 > 0x0F
        {
            f|=(FLAGS::H as u8);
        }
        else {
            f&=!(FLAGS::H as u8);
        }

        self.registers.set_f(f);

        self.cycles+=4;
        result
    }
    fn decrement_8_bit (&mut self, data:u8)-> u8
    {
        let result = data.wrapping_sub(1);
        let mut f = self.registers.get_f();

        //setting Z flag
        if result == 0
        {
            f |= FLAGS::Z as u8;
        }
        else {
            f&= !(FLAGS::Z as u8);
        }

        //setting N flag

        f |= (FLAGS::N as u8);

        //setting H flag

        if(data & 0x0F) + 1 > 0x0F
        {
            f|=(FLAGS::H as u8);
        }
        else {
            f&=!(FLAGS::H as u8);
        }

        self.registers.set_f(f);

        self.cycles+=4;
        result
    }
    fn zero_bit_8bit(&mut self, value: u8, bit: u8) -> u8
    {
        self.cycles +=8;
        value & !(1 << bit)
    }
    fn set_bit_8bit(&mut self, value: u8, bit: u8) -> u8
    {
        self.cycles +=8;
        value | (1 << bit)
    }
}

struct RAM {
    memory: [u8; 65536],
}
impl RAM {
    fn new() -> Self {
        RAM { memory: [0; 65536] }
    }
    fn read(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }
    fn write(&mut self, address: u16, data: u8) {
        self.memory[address as usize] = data;
    }
}
fn main() {
    //creating ram
    let ram = RAM::new();
    println!("Hello, world!");
}
