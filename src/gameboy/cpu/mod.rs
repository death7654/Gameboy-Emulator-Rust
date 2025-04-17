mod registers;

use registers::REGISTERS;
use crate::RAM;
use registers::FLAGS;

struct CPU<'a> {
    registers: REGISTERS,
    ime: bool, //interrupt master enable
    opcode: u8,
    cycles: u64,
    ram: &'a mut RAM,
}
impl<'a> CPU<'a> {
    pub fn new(ram: &'a mut RAM) -> Self {
        CPU {
            registers: REGISTERS::new(),
            ime: true,
            opcode: 0,
            cycles: 0,
            ram,
        }
    }
    pub fn fetch(&mut self) -> u8 {
        let opcode = self.ram.read(self.registers.get_pc()); 
        self.registers.set_pc(self.registers.get_pc()+1);
        opcode
    }
    pub fn execute(&mut self, opcode: u8) {
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
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_b(data);
                self.cycles += 8;
            }
            0x07 => {
                let a = self.registers.get_a();
                let msb = (a & 0b1000_0000) >> 7;
                let result = (a << 1) | msb;

                self.registers.set_a(result);

                if result == 0 {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::Z as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::Z as u8));
                }

                self.registers
                    .set_f(self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8));

                if msb != 0 {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::C as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::C as u8));
                }
                self.cycles += 8;
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
                self.registers
                    .set_bc(self.registers.get_bc().wrapping_sub(1));
                self.cycles += 8;
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
                let data = self.increment_8_bit(self.registers.get_d());
                self.registers.set_d(data);
            }
            0x15 => {
                let data = self.decrement_8_bit(self.registers.get_d());
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
                let data = self.increment_8_bit(self.registers.get_e());
                self.registers.set_e(data);
            }
            0x1d => {
                let data = self.decrement_8_bit(self.registers.get_e());
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
                let data = self.increment_8_bit(self.registers.get_h());
                self.registers.set_h(data);
            }
            0x25 => {
                let data = self.decrement_8_bit(self.registers.get_h());
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
                let data = self.increment_8_bit(self.registers.get_l());
                self.registers.set_l(data);
            }
            0x2D => {
                let data = self.decrement_8_bit(self.registers.get_l());
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
                self.registers
                    .set_f(self.registers.get_f() | FLAGS::N as u8 | FLAGS::H as u8);

                self.cycles += 4;
            }
            0x30 => {
                let offset = self.ram.read(self.registers.get_and_inc_pc()) as i8;
                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(offset as i16 as u16));
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
                self.registers
                    .set_f(self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8));

                self.registers
                    .set_f(self.registers.get_f() | FLAGS::C as u8);

                self.cycles += 4;
            }
            0x38 => {
                let offset = self.ram.read(self.registers.get_and_inc_pc()) as i8;
                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    self.registers
                        .set_pc(self.registers.get_pc().wrapping_add(offset as i16 as u16));
                    self.cycles += 12;
                } else {
                    self.cycles += 8;
                }
            }
            0x39 => {
                let hl = self.registers.get_hl();
                let sp = self.registers.get_sp();
                let result = hl.wrapping_add(sp);
                self.registers.set_hl(result);

                if (hl & 0xFFF) + (sp & 0xFFF) > 0xFFF {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::H as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::H as u8));
                }

                // Set C flag if carry occurs from bit 15 to bit 16
                if hl > 0xFFFF - sp {
                    self.registers
                        .set_f(self.registers.get_f() | FLAGS::C as u8);
                } else {
                    self.registers
                        .set_f(self.registers.get_f() & !(FLAGS::C as u8));
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
                let data = self.increment_8_bit(self.registers.get_a());
                self.registers.set_a(data);
            }
            0x3D => {
                let data = self.decrement_8_bit(self.registers.get_a());
                self.registers.set_a(data);
            }
            0x3E => {
                let value = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_a(value);
                self.cycles += 8;
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
                self.cycles += 4;
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
                self.add_8bit(self.ram.read(self.registers.get_hl()));
                self.cycles += 4;
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
                self.add_with_carry(self.ram.read(self.registers.get_hl()));
                self.cycles += 4;
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
                self.sub_8bit(self.ram.read(self.registers.get_hl()));
                self.cycles += 4;
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
                self.sub_with_carry(self.ram.read(self.registers.get_hl()));
                self.cycles += 4;
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
                self.cycles += 4;
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
                self.and(self.ram.read(self.registers.get_hl()));
                self.cycles += 4;
            }
            0xA7 => {
                self.and(self.registers.get_a());
            }
            0xA8 => {
                self.XOR(self.registers.get_b());
            }
            0xA9 => {
                self.XOR(self.registers.get_c());
            }
            0xAA => {
                self.XOR(self.registers.get_d());
            }
            0xAB => {
                self.XOR(self.registers.get_e());
            }
            0xAC => {
                self.XOR(self.registers.get_h());
            }
            0xAD => {
                self.XOR(self.registers.get_l());
            }
            0xAE => {
                self.XOR(self.ram.read(self.registers.get_hl()));
                self.cycles += 4;
            }
            0xAF => {
                self.XOR(self.registers.get_a());
                self.registers
                    .set_f(self.registers.get_f() | (FLAGS::Z as u8));
            }
            0xB0 => {
                self.OR(self.registers.get_b());
            }
            0xB1 => {
                self.OR(self.registers.get_c());
            }
            0xB2 => {
                self.OR(self.registers.get_d());
            }
            0xB3 => {
                self.OR(self.registers.get_e());
            }
            0xB4 => {
                self.OR(self.registers.get_h());
            }
            0xB5 => {
                self.OR(self.registers.get_l());
            }
            0xB6 => {
                self.OR(self.ram.read(self.registers.get_hl()));
                self.cycles += 4;
            }
            0xB7 => {
                self.OR(self.registers.get_a());
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
                self.compare(self.ram.read(self.registers.get_hl()));
                self.cycles += 4;
            }
            0xBF => {
                self.compare(self.registers.get_a());
                let mut f = self.registers.get_f() & !(FLAGS::H as u8 | FLAGS::C as u8);
                f |= (FLAGS::Z as u8 | FLAGS::N as u8);
                self.registers.set_f(f);
            }
            0xC0 => {
                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    let lower_byte = self.ram.read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    let upper_byte = self.ram.read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);

                    self.cycles += 20;
                } else {
                    self.cycles += 8;
                }
            }
            0xC1 => {
                let lower_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                let upper_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));

                let value = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_bc(value);

                self.cycles += 12;
            }
            0xC2 => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    self.registers.set_pc(address);
                    self.cycles += 16;
                } else {
                    self.cycles += 12;
                }
            }
            0xC3 => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_pc(address);
                self.cycles += 16;
            }
            0xC4 => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::Z as u8 == 0 {
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );

                    self.registers.set_pc(address);

                    self.cycles += 24;
                } else {
                    self.cycles += 12;
                }
            }
            0xC5 => {
                let bc = self.registers.get_bc();

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram.write(self.registers.get_sp(), (bc >> 8) as u8);

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram.write(self.registers.get_sp(), (bc & 0xFF) as u8);

                self.cycles += 16;
            }
            0xC6 => {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.add_8bit(data);
                self.cycles += 8;
            }
            0xC7 => {
                self.reset(0x00);
            }
            0xC8 => {
                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    let lower_byte = self.ram.read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    let upper_byte = self.ram.read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);

                    self.cycles += 20;
                } else {
                    self.cycles += 8;
                }
            }
            0xC9 => {
                let lower_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                let upper_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));

                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_pc(address);

                self.cycles += 16;
            }
            0xCA => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    self.registers.set_pc(address);

                    self.cycles += 16;
                } else {
                    self.cycles += 12;
                }
            }
            0xCB => {
                let opcode = self.fetch();
                self.cb(opcode);
            }
            0xCC => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::Z as u8 != 0 {
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );

                    self.registers.set_pc(address);

                    self.cycles += 24;
                } else {
                    self.cycles += 12;
                }
            }
            0xCD => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram.write(
                    self.registers.get_sp(),
                    (self.registers.get_pc() >> 8) as u8,
                );
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram.write(
                    self.registers.get_sp(),
                    (self.registers.get_pc() & 0xFF) as u8,
                );

                self.registers.set_pc(address);

                self.cycles += 24;
            }
            0xCE => {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.add_with_carry(data);
                self.cycles += 4;
            }
            0xCF => {
                self.reset(0x08);
            }
            0xD0 => {
                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    let lower_byte = self.ram.read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    let upper_byte = self.ram.read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);

                    self.cycles += 20;
                } else {
                    self.cycles += 8;
                }
            }
            0xD1 => {
                let lower_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                let upper_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));

                let value = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_de(value);

                self.cycles += 12
            }
            0xD2 => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.registers.set_pc(address);
                    self.cycles += 16;
                } else {
                    self.cycles += 12;
                }
            }
            0xD3 => {
                println!("D3 NOT VALID");
                self.cycles += 4;
            }
            0xD4 => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::C as u8 == 0 {
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );

                    self.registers.set_pc(address);

                    self.cycles += 24;
                } else {
                    self.cycles += 12;
                }
            }
            0xD5 => {
                let de = self.registers.get_de();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram.write(self.registers.get_sp(), (de >> 8) as u8);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram.write(self.registers.get_sp(), (de & 0xFF) as u8);
                self.cycles += 16;
            }
            0xD6 => {
                let address = self.registers.get_and_inc_pc();
                self.sub_8bit(self.ram.read(address));
                self.cycles += 4;
            }
            0xD7 => {
                self.reset(0x10);
            }
            0xD8 => {
                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    let lower_byte = self.ram.read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));
                    let upper_byte = self.ram.read(self.registers.get_sp());
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_add(1));

                    let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                    self.registers.set_pc(address);

                    self.cycles += 20;
                } else {
                    self.cycles += 8;
                }
            }
            0xD9 => {
                let lower_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                let upper_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));

                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_pc(address);

                self.ime = true;
                self.cycles += 16;
            }
            0xDA => {
                let lower = self.ram.read(self.registers.get_and_inc_pc());
                let upper = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper as u16) << 8) | lower as u16;
                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    self.registers.set_pc(address);
                    self.cycles += 16;
                } else {
                    self.cycles += 12;
                }
            }
            0xDB => {
                println!("DB ILLEGAL");
                self.cycles += 4;
            }
            0xDC => {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;

                if self.registers.get_f() & FLAGS::C as u8 != 0 {
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() >> 8) as u8,
                    );
                    self.registers
                        .set_sp(self.registers.get_sp().wrapping_sub(1));
                    self.ram.write(
                        self.registers.get_sp(),
                        (self.registers.get_pc() & 0xFF) as u8,
                    );

                    self.registers.set_pc(address);

                    self.cycles += 24;
                } else {
                    self.cycles += 12;
                }
            }
            0xDD => {
                println!("ILLEGAL_DD");
                self.cycles += 4;
            }
            0xDE => {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.sub_with_carry(data);
                self.cycles += 4;
            }
            0xDF => {
                self.reset(0x18);
            }
            0xE0 => {
                let offset = self.ram.read(self.registers.get_and_inc_pc());
                let address = 0xFF00 | (offset as u16);

                self.ram.write(address, self.registers.get_a());

                self.cycles += 12;
            }
            0xE1 => {
                let lower_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                let upper_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));

                let value = ((upper_byte as u16) << 8) | lower_byte as u16;
                self.registers.set_hl(value);

                self.cycles += 12;
            }
            0xE2 => {
                let address = 0xff00 | self.registers.get_c() as u16;
                self.ram.write(address, self.registers.get_a());

                self.cycles += 8;
            }
            0xE3 => {
                print!("E3 is Illegal");
                self.cycles += 4;
            }
            0xE4 => {
                print!("E4 is Illegal");
                self.cycles += 4;
            }
            0xE5 => {
                let hl = self.registers.get_hl();

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram.write(self.registers.get_sp(), (hl >> 8) as u8);

                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram.write(self.registers.get_sp(), (hl & 0xFF) as u8);

                self.cycles += 16;
            }
            0xE6 => {
                let address = self.registers.get_and_inc_pc();
                self.and(self.ram.read(address));
            }
            0xE7 => {
                self.reset(0x20);
            }
            0xE8 => {
                let sp = self.registers.get_sp();
                let e8 = self.ram.read(self.registers.get_and_inc_pc()) as i8; // Read signed immediate value

                let result = sp.wrapping_add(e8 as u16);
                self.registers.set_sp(result);

                let mut f = 0;

                if (sp & 0x0F) + (e8 as u16 & 0x0F) > 0x0F {
                    f |= FLAGS::H as u8;
                }

                if (sp & 0xFF) + (e8 as u16 & 0xFF) > 0xFF {
                    f |= FLAGS::C as u8;
                }

                self.registers.set_f(f);
                self.cycles += 16;
            }
            0xE9 => {
                self.registers.set_pc(self.registers.get_hl());
            }
            0xEA => {
                let lower = self.ram.read(self.registers.get_and_inc_pc());
                let upper = self.ram.read(self.registers.get_and_inc_pc());
                let address = (upper as u16) << 8 | lower as u16;

                self.ram.write(address, self.registers.get_a());
                self.cycles += 16;
            }
            0xEB => {
                println!("EB ILLEGAL");
                self.cycles += 4;
            }
            0xEC => {
                println!("EC ILLEGAL");
                self.cycles += 4;
            }
            0xED => {
                println!("ED ILLEGAL");
                self.cycles += 4;
            }
            0xEE => {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.XOR(data);
            }
            0xEF => {
                self.reset(0x28);
            }
            0xF0 => {
                let offset = self.ram.read(self.registers.get_and_inc_pc());
                let address = 0xFF00 | (offset as u16);

                let value = self.ram.read(address);
                self.registers.set_a(value);

                self.cycles += 12;
            }
            0xF1 => {
                let lower_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));
                let upper_byte = self.ram.read(self.registers.get_sp());
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_add(1));

                let value = ((upper_byte as u16) << 8) | (lower_byte & 0xF0) as u16;
                self.registers.set_af(value);

                self.cycles += 12;
            }
            0xF2 => {
                let address = 0xFF00 | self.registers.get_c() as u16;
                let data = self.ram.read(address);
                self.registers.set_a(data);
                self.cycles += 8;
            }
            0xF3 => {
                self.ime = false;
                self.cycles += 4;
            }
            0xF4 => {
                println!("ILLEGAL F4");
                self.cycles += 4;
            }
            0xF5 => {
                let af = self.registers.get_af();
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram.write(self.registers.get_sp(), (af >> 8) as u8);
                self.registers
                    .set_sp(self.registers.get_sp().wrapping_sub(1));
                self.ram.write(self.registers.get_sp(), (0xFF & af) as u8);

                self.cycles += 16;
            }
            0xF6 => {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.OR(data);
            }
            0xF7 => {
                self.reset(0x30);
            }
            0xF8 => {
                let sp = self.registers.get_sp();
                let e8 = self.ram.read(self.registers.get_and_inc_pc()) as i8; // Signed immediate value

                let result = sp.wrapping_add(e8 as u16);
                self.registers.set_hl(result);

                let mut f = 0;

                if (sp & 0x0F) + (e8 as u16 & 0x0F) > 0x0F {
                    f |= FLAGS::H as u8;
                }

                if (sp & 0xFF) + (e8 as u16 & 0xFF) > 0xFF {
                    f |= FLAGS::C as u8;
                }

                self.registers.set_f(f);
                self.cycles += 12;
            }
            0xF9 => {
                self.registers.set_sp(self.registers.get_hl());
                self.cycles += 8;
            }
            0xFA => {
                let lower = self.ram.read(self.registers.get_and_inc_pc());
                let upper = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper as u16) << 8) | lower as u16;

                self.registers.set_a(self.ram.read(address));
                self.cycles += 16;
            }
            0xFB => {
                self.ime = true;
                self.cycles += 4;
            }
            0xFC => {
                println!("FC ILLEGAL");
                self.cycles += 4;
            }
            0xFD => {
                println!("FD ILLEGAL");
                self.cycles += 4;
            }
            0xFE => {
                let a = self.registers.get_a();
                let n8 = self.ram.read(self.registers.get_and_inc_pc());

                let (result, borrow) = a.overflowing_sub(n8);

                let mut f = FLAGS::N as u8;
                if result == 0 {
                    f |= FLAGS::Z as u8;
                }

                if (a & 0x0F) < (n8 & 0x0F) {
                    f |= FLAGS::H as u8;
                }

                if borrow {
                    f |= FLAGS::C as u8;
                }

                self.registers.set_f(f);
                self.cycles += 8;
            }
            0xFF => {
                self.reset(0x38);
            }
            _ => println!("{} Not Implemented", opcode),
        }
    }
    fn cb(&mut self, opcode: u8) {
        match opcode {
            0x00 => {
                let data = self.rotate_without_carry(self.registers.get_b(), 0);
                self.registers.set_b(data);
            }
            0x01 => {
                let data = self.rotate_without_carry(self.registers.get_c(), 0);
                self.registers.set_c(data);
            }
            0x02 => {
                let data = self.rotate_without_carry(self.registers.get_d(), 0);
                self.registers.set_d(data);
            }
            0x03 => {
                let data = self.rotate_without_carry(self.registers.get_e(), 0);
                self.registers.set_e(data);
            }
            0x04 => {
                let data = self.rotate_without_carry(self.registers.get_h(), 0);
                self.registers.set_h(data);
            }
            0x05 => {
                let data = self.rotate_without_carry(self.registers.get_l(), 0);
                self.registers.set_l(data);
            }
            0x06 => {
                let data = self.rotate_without_carry(self.ram.read(self.registers.get_hl()), 0);
                self.ram.write(self.registers.get_hl(), data);
                self.cycles += 8;
            }
            0x07 => {
                let data = self.rotate_without_carry(self.registers.get_a(), 0);
                self.registers.set_a(data);
            }
            0x08 => {
                let data = self.rotate_without_carry(self.registers.get_b(), 1);
                self.registers.set_b(data);
            }
            0x09 => {
                let data = self.rotate_without_carry(self.registers.get_c(), 1);
                self.registers.set_c(data);
            }
            0x0A => {
                let data = self.rotate_without_carry(self.registers.get_d(), 1);
                self.registers.set_d(data);
            }
            0x0B => {
                let data = self.rotate_without_carry(self.registers.get_e(), 1);
                self.registers.set_e(data);
            }
            0x0C => {
                let data = self.rotate_without_carry(self.registers.get_h(), 1);
                self.registers.set_h(data);
            }
            0x0D => {
                let data = self.rotate_without_carry(self.registers.get_l(), 1);
                self.registers.set_l(data);
            }
            0x0E => {
                let data = self.rotate_without_carry(self.ram.read(self.registers.get_hl()), 1);
                self.ram.write(self.registers.get_hl(), data);
                self.cycles += 8;
            }
            0x0F => {
                let data = self.rotate_without_carry(self.registers.get_a(), 1);
                self.registers.set_a(data);
            }
            0x10 => {
                let data = self.rotate(self.registers.get_b(), 0);
                self.registers.set_b(data);
            }
            0x11 => {
                let data = self.rotate(self.registers.get_c(), 0);
                self.registers.set_c(data);
            }
            0x12 => {
                let data = self.rotate(self.registers.get_d(), 0);
                self.registers.set_d(data);
            }
            0x13 => {
                let data = self.rotate(self.registers.get_e(), 0);
                self.registers.set_e(data);
            }
            0x14 => {
                let data = self.rotate(self.registers.get_h(), 0);
                self.registers.set_h(data);
            }
            0x15 => {
                let data = self.rotate(self.registers.get_l(), 0);
                self.registers.set_l(data);
            }
            0x16 => {
                let data = self.rotate(self.ram.read(self.registers.get_hl()), 0);
                self.ram.write(self.registers.get_hl(), data);
                self.cycles += 8;
            }
            0x17 => {
                let data = self.rotate(self.registers.get_a(), 0);
                self.registers.set_a(data);
            }
            0x18 => {
                let data = self.rotate(self.registers.get_b(), 1);
                self.registers.set_b(data);
            }
            0x19 => {
                let data = self.rotate(self.registers.get_c(), 1);
                self.registers.set_c(data);
            }
            0x1A => {
                let data = self.rotate(self.registers.get_d(), 1);
                self.registers.set_d(data);
            }
            0x1B => {
                let data = self.rotate(self.registers.get_e(), 1);
                self.registers.set_e(data);
            }
            0x1C => {
                let data = self.rotate(self.registers.get_h(), 1);
                self.registers.set_h(data);
            }
            0x1D => {
                let data = self.rotate(self.registers.get_l(), 1);
                self.registers.set_l(data);
            }
            0x1E => {
                let data = self.rotate(self.ram.read(self.registers.get_hl()), 1);
                self.ram.write(self.registers.get_hl(), data);
                self.cycles += 8;
            }
            0x1F => {
                let data = self.rotate(self.registers.get_a(), 1);
                self.registers.set_a(data);
            }
            0x20 => {
                let data = self.shift(self.registers.get_b(), 0);
                self.registers.set_b(data);
            }
            0x21 => {
                let data = self.shift(self.registers.get_c(), 0);
                self.registers.set_c(data);
            }
            0x22 => {
                let data = self.shift(self.registers.get_d(), 0);
                self.registers.set_d(data);
            }
            0x23 => {
                let data = self.shift(self.registers.get_e(), 0);
                self.registers.set_e(data);
            }
            0x24 => {
                let data = self.shift(self.registers.get_h(), 0);
                self.registers.set_h(data);
            }
            0x25 => {
                let data = self.shift(self.registers.get_l(), 0);
                self.registers.set_l(data);
            }
            0x26 => {
                let data = self.shift(self.ram.read(self.registers.get_hl()), 0);
                self.ram.write(self.registers.get_hl(), data);
                self.cycles += 8;
            }
            0x27 => {
                let data = self.shift(self.registers.get_a(), 0);
                self.registers.set_a(data);
            }
            0x28 => {
                let data = self.shift(self.registers.get_b(), 1);
                self.registers.set_b(data);
            }
            0x29 => {
                let data = self.shift(self.registers.get_c(), 1);
                self.registers.set_c(data);
            }
            0x2A => {
                let data = self.shift(self.registers.get_d(), 1);
                self.registers.set_d(data);
            }
            0x2B => {
                let data = self.shift(self.registers.get_e(), 1);
                self.registers.set_e(data);
            }
            0x2C => {
                let data = self.shift(self.registers.get_h(), 1);
                self.registers.set_h(data);
            }
            0x2D => {
                let data = self.shift(self.registers.get_l(), 1);
                self.registers.set_l(data);
            }
            0x2E => {
                let data = self.shift(self.ram.read(self.registers.get_hl()), 1);
                self.ram.write(self.registers.get_hl(), data);
                self.cycles += 8;
            }
            0x2F => {
                let data = self.shift(self.registers.get_a(), 1);
                self.registers.set_a(data);
            }
            0x30 => {
                let data = self.swap(self.registers.get_b());
                self.registers.set_b(data);
            }
            0x31 => {
                let data = self.swap(self.registers.get_c());
                self.registers.set_c(data);
            }
            0x32 => {
                let data = self.swap(self.registers.get_d());
                self.registers.set_d(data);
            }
            0x33 => {
                let data = self.swap(self.registers.get_e());
                self.registers.set_e(data);
            }
            0x34 => {
                let data = self.swap(self.registers.get_h());
                self.registers.set_h(data);
            }
            0x35 => {
                let data = self.swap(self.registers.get_l());
                self.registers.set_l(data);
            }
            0x36 => {
                let data = self.swap(self.ram.read(self.registers.get_hl()));
                self.ram.write(self.registers.get_hl(), data);
                self.cycles += 8;
            }
            0x37 => {
                let data = self.swap(self.registers.get_a());
                self.registers.set_a(data);
            }
            0x38 => {
                let data = self.right_shift(self.registers.get_b());
                self.registers.set_b(data);
            }
            0x39 => {
                let data = self.right_shift(self.registers.get_c());
                self.registers.set_c(data);
            }
            0x3A => {
                let data = self.right_shift(self.registers.get_d());
                self.registers.set_d(data);
            }
            0x3B => {
                let data = self.right_shift(self.registers.get_e());
                self.registers.set_e(data);
            }
            0x3C => {
                let data = self.right_shift(self.registers.get_h());
                self.registers.set_h(data);
            }
            0x3D => {
                let data = self.right_shift(self.registers.get_l());
                self.registers.set_l(data);
            }
            0x3E => {
                let data = self.right_shift(self.ram.read(self.registers.get_hl()));
                self.ram.write(self.registers.get_hl(), data);
                self.cycles += 8;
            }
            0x3F => {
                let data = self.right_shift(self.registers.get_a());
                self.registers.set_a(data);
            }
            0x40 => self.bit(self.registers.get_b(), 0),
            0x41 => self.bit(self.registers.get_c(), 0),
            0x42 => self.bit(self.registers.get_d(), 0),
            0x43 => self.bit(self.registers.get_e(), 0),
            0x44 => self.bit(self.registers.get_h(), 0),
            0x45 => self.bit(self.registers.get_l(), 0),
            0x46 => {
                self.bit(self.ram.read(self.registers.get_hl()), 0);
                self.cycles += 8;
            }
            0x47 => self.bit(self.registers.get_a(), 0),
            0x48 => self.bit(self.registers.get_b(), 1),
            0x49 => self.bit(self.registers.get_c(), 1),
            0x4A => self.bit(self.registers.get_d(), 1),
            0x4B => self.bit(self.registers.get_e(), 1),
            0x4C => self.bit(self.registers.get_h(), 1),
            0x4D => self.bit(self.registers.get_l(), 1),
            0x4E => {
                self.bit(self.ram.read(self.registers.get_hl()), 1);
                self.cycles += 8;
            }
            0x4F => self.bit(self.registers.get_a(), 1),
            0x50 => self.bit(self.registers.get_b(), 2),
            0x51 => self.bit(self.registers.get_c(), 2),
            0x52 => self.bit(self.registers.get_d(), 2),
            0x53 => self.bit(self.registers.get_e(), 2),
            0x54 => self.bit(self.registers.get_h(), 2),
            0x55 => self.bit(self.registers.get_l(), 2),
            0x56 => {
                self.bit(self.ram.read(self.registers.get_hl()), 2);
                self.cycles += 8;
            }
            0x57 => self.bit(self.registers.get_a(), 2),
            0x58 => self.bit(self.registers.get_b(), 3),
            0x59 => self.bit(self.registers.get_c(), 3),
            0x5A => self.bit(self.registers.get_d(), 3),
            0x5B => self.bit(self.registers.get_e(), 3),
            0x5C => self.bit(self.registers.get_h(), 3),
            0x5D => self.bit(self.registers.get_l(), 3),
            0x5E => {
                self.bit(self.ram.read(self.registers.get_hl()), 3);
                self.cycles += 8;
            }
            0x5F => self.bit(self.registers.get_a(), 3),
            0x60 => self.bit(self.registers.get_b(), 4),
            0x61 => self.bit(self.registers.get_c(), 4),
            0x62 => self.bit(self.registers.get_d(), 4),
            0x63 => self.bit(self.registers.get_e(), 4),
            0x64 => self.bit(self.registers.get_h(), 4),
            0x65 => self.bit(self.registers.get_l(), 4),
            0x66 => {
                self.bit(self.ram.read(self.registers.get_hl()), 4);
                self.cycles += 8;
            }
            0x67 => self.bit(self.registers.get_a(), 4),
            0x68 => self.bit(self.registers.get_b(), 5),
            0x69 => self.bit(self.registers.get_c(), 5),
            0x6A => self.bit(self.registers.get_d(), 5),
            0x6B => self.bit(self.registers.get_e(), 5),
            0x6C => self.bit(self.registers.get_h(), 5),
            0x6D => self.bit(self.registers.get_l(), 5),
            0x6E => {
                self.bit(self.ram.read(self.registers.get_hl()), 5);
                self.cycles += 8;
            }
            0x6F => self.bit(self.registers.get_a(), 5),
            0x70 => self.bit(self.registers.get_b(), 6),
            0x71 => self.bit(self.registers.get_c(), 6),
            0x72 => self.bit(self.registers.get_d(), 6),
            0x73 => self.bit(self.registers.get_e(), 6),
            0x74 => self.bit(self.registers.get_h(), 6),
            0x75 => self.bit(self.registers.get_l(), 6),
            0x76 => {
                self.bit(self.ram.read(self.registers.get_hl()), 6);
                self.cycles += 8;
            }
            0x77 => self.bit(self.registers.get_a(), 6),
            0x78 => self.bit(self.registers.get_b(), 7),
            0x79 => self.bit(self.registers.get_c(), 7),
            0x7A => self.bit(self.registers.get_d(), 7),
            0x7B => self.bit(self.registers.get_e(), 7),
            0x7C => self.bit(self.registers.get_h(), 7),
            0x7D => self.bit(self.registers.get_l(), 7),
            0x7E => {
                self.bit(self.ram.read(self.registers.get_hl()), 7);
                self.cycles += 8;
            }
            0x7F => self.bit(self.registers.get_a(), 7),

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
            0x84 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 0);
                self.registers.set_h(data);
            }
            0x85 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 0);
                self.registers.set_l(data);
            }
            0x86 => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & !(1 << 0)); 
                self.cycles+=16;
            }
            0x87 => {
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
            0x8C => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 1);
                self.registers.set_h(data);
            }
            0x8D => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 1);
                self.registers.set_l(data);
            }
            0x8E => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & !(1 << 1)); 
                self.cycles+=16;
            }
            0x8F => {
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
            0x94 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 2);
                self.registers.set_h(data);
            }
            0x95 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 2);
                self.registers.set_l(data);
            }
            0x96 => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & !(1 << 2)); 
                self.cycles+=16;
            }
            0x97 => {
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
            0x9C => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 3);
                self.registers.set_h(data);
            }
            0x9D => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 3);
                self.registers.set_l(data);
            }
            0x9E => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & !(1 << 3)); 
                self.cycles+=16;
            }
            0x9F => {
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
            0xA4 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 4);
                self.registers.set_h(data);
            }
            0xA5 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 4);
                self.registers.set_l(data);
            }
            0xA6 => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & !(1 << 4)); 
                self.cycles+=16;
            }
            0xA7 => {
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
            0xAC => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 5);
                self.registers.set_h(data);
            }
            0xAD => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 5);
                self.registers.set_l(data);
            }
            0xAE => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & !(1 << 5)); 
                self.cycles+=16;
            }
            0xAF => {
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
            0xB4 => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 6);
                self.registers.set_h(data);
            }
            0xB5 => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 6);
                self.registers.set_l(data);
            }
            0xB6 => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & !(1 << 6)); 
                self.cycles+=16;
            }
            0xB7 => {
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
            0xBC => {
                let data = self.zero_bit_8bit(self.registers.get_h(), 7);
                self.registers.set_h(data);
            }
            0xBD => {
                let data = self.zero_bit_8bit(self.registers.get_l(), 7);
                self.registers.set_l(data);
            }
            0xBE => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & !(1 << 7)); 
                self.cycles+=16;
            }
            0xBF => {
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
            0xC4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 0);
                self.registers.set_h(data);
            }
            0xC5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 0);
                self.registers.set_l(data);
            }
            0xC6 => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & (1 << 0)); 
                self.cycles+=16;

            }
            0xC7 => {
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
                self.registers.set_e(data);            }
            0xCC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 1);
                self.registers.set_h(data);
            }
            0xCD => {
                let data = self.set_bit_8bit(self.registers.get_l(), 1);
                self.registers.set_l(data);
            }
            0xCE => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & (1 << 1)); 
                self.cycles+=16;
            }
            0xCF => {
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
            0xD4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 2);
                self.registers.set_h(data);
            }
            0xD5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 2);
                self.registers.set_l(data);
            }
            0xD6 => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & (1 << 2)); 
                self.cycles+=16;
            }
            0xD7 => {
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
            0xDC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 3);
                self.registers.set_h(data);
            }
            0xDD => {
                let data = self.set_bit_8bit(self.registers.get_l(), 3);
                self.registers.set_l(data);
            }
            0xDE => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & (1 << 3)); 
                self.cycles+=16;
            }
            0xDF => {
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
            0xE4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 4);
                self.registers.set_h(data);
            }
            0xE5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 4);
                self.registers.set_l(data);
            }
            0xE6 => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & (1 << 4)); 
                self.cycles+=16;
            }
            0xE7 => {
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
            0xEC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 5);
                self.registers.set_h(data);
            }
            0xED => {
                let data = self.set_bit_8bit(self.registers.get_l(), 5);
                self.registers.set_l(data);
            }
            0xEE => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & (1 << 5)); 
                self.cycles+=16;
            }
            0xEF => {
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
            0xF4 => {
                let data = self.set_bit_8bit(self.registers.get_h(), 6);
                self.registers.set_h(data);
            }
            0xF5 => {
                let data = self.set_bit_8bit(self.registers.get_l(), 6);
                self.registers.set_l(data);
            }
            0xF6 => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & (1 << 6)); 
                self.cycles+=16;
            }
            0xF7 => {
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
            0xFC => {
                let data = self.set_bit_8bit(self.registers.get_h(), 7);
                self.registers.set_h(data);
            }
            0xFD => {
                let data = self.set_bit_8bit(self.registers.get_l(), 7);
                self.registers.set_l(data);
            }
            0xFE => {
                let address = self.registers.get_hl();
                let data = self.ram.read(address);
                self.ram.write(address, data & (1 << 7)); 
                self.cycles+=16;
            }
            0xFF => {
                let data = self.set_bit_8bit(self.registers.get_a(), 7);
                self.registers.set_a(data);
            }
            _ => {
                println!("Special Case CB OPCODE: {} not implemented", opcode);
            }
        }
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
            f |= (FLAGS::H as u8);
        } else {
            f &= !(FLAGS::H as u8);
        }

        self.registers.set_f(f);

        self.cycles += 4;
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

        f |= (FLAGS::N as u8);

        //setting H flag

        if (data & 0x0F) == 0 {
            f |= (FLAGS::H as u8);
        } else {
            f &= !(FLAGS::H as u8);
        }

        self.registers.set_f(f);

        self.cycles += 4;
        result
    }
    fn zero_bit_8bit(&mut self, value: u8, bit: u8) -> u8 {
        self.cycles += 8;
        value & !(1 << bit)
    }
    fn set_bit_8bit(&mut self, value: u8, bit: u8) -> u8 {
        self.cycles += 8;
        value | (1 << bit)
    }

    fn add_8bit(&mut self, b: u8) {
        let a = self.registers.get_a();
        let (result, carry) = a.overflowing_add(b); // Separate carry result

        let mut f = (self.registers.get_f() & !(FLAGS::N as u8));

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
        self.cycles += 4;
    }

    fn add_with_carry(&mut self, b: u8) {
        let a = self.registers.get_a();
        let carry = (self.registers.get_f() & FLAGS::C as u8) >> 4;
        let (result, carry1) = a.overflowing_add(b);
        let (result, carry2) = result.overflowing_add(carry);

        let mut f = self.registers.get_f() & !(FLAGS::N as u8);

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

        self.registers.set_f(f);
        self.cycles += 4;
    }
    fn sub_8bit(&mut self, b: u8) {
        let a = self.registers.get_a();
        let (result, borrow) = a.overflowing_sub(b); // Separate carry result

        let mut f = (self.registers.get_f() | (FLAGS::N as u8));

        self.registers.set_a(result);
        // Z flag
        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        // H flag
        if (a & 0x0F) < (b & 0x0F) {
            f |= FLAGS::H as u8;
        } else {
            f &= !(FLAGS::H as u8);
        }

        if borrow {
            f |= FLAGS::C as u8;
        } else {
            f &= !(FLAGS::C as u8);
        }

        self.registers.set_f(f);
        self.cycles += 4;
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
        self.cycles += 4;
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
        self.cycles += 4;
    }
    fn XOR(&mut self, b: u8) {
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
        self.cycles += 4;
    }
    fn OR(&mut self, b: u8) {
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
        self.cycles += 4;
    }

    fn compare(&mut self, b: u8) {
        let a = self.registers.get_a();
        let (result, borrow) = a.overflowing_sub(b); // Simulates subtraction for flag updates

        let mut f = self.registers.get_f();
        f |= FLAGS::N as u8;

        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8)
        }

        if (a & 0x0F) < (b & 0x0F) {
            f |= FLAGS::H as u8;
        } else {
            f &= !(FLAGS::H as u8)
        }

        if borrow {
            f |= FLAGS::C as u8;
        } else {
            f &= !(FLAGS::C as u8)
        }

        self.registers.set_f(f);
        self.cycles += 4;
    }
    fn reset(&mut self, address: u16) {
        self.registers
            .set_sp(self.registers.get_sp().wrapping_sub(1));
        self.ram.write(
            self.registers.get_sp(),
            (self.registers.get_pc() >> 8) as u8,
        );
        self.registers
            .set_sp(self.registers.get_sp().wrapping_sub(1));
        self.ram.write(
            self.registers.get_sp(),
            (self.registers.get_pc() & 0xFF) as u8,
        );

        self.registers.set_pc(address);

        self.cycles += 16;
    }
    fn rotate_without_carry(&mut self, mut value: u8, type_: u8) -> u8 {
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
        }
        else {
            f &= !(FLAGS::Z as u8)
        }

        if bool {
            f |= FLAGS::C as u8;
        }
        else {
            f &= !(FLAGS::C as u8)
        }

        self.registers.set_f(f);
        self.cycles += 8;

        value
    }
    fn rotate(&mut self, mut value: u8, type_: u8) -> u8 {
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
        }
        else {
            f &= !(FLAGS::Z as u8)
        }

        if bool {
            f |= FLAGS::C as u8;
        }
        else {
            f &= !(FLAGS::C as u8)
        }

        self.registers.set_f(f);
        self.cycles += 8;

        value
    }

    fn shift(&mut self, mut value: u8, type_: u8) -> u8 {
        let msb = value & 0x80;
        let lsb_bool = (value & 0x01) != 0;
        let msb_bool = (value & 0x80) != 0;
        let mut f: u8 = self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8);
        if type_ == 0 {
            value <<= 1;
            if msb_bool {
                f |= FLAGS::C as u8;
            }
            else {
                f &= !(FLAGS::C as u8)
            }
        } else {
            value = (value >> 1) | msb;
            if lsb_bool {
                f |= FLAGS::C as u8;
            }
            else {
                f &= !(FLAGS::C as u8)
            }
        }

        if value == 0 {
            f |= FLAGS::Z as u8;
        }

        self.registers.set_f(f);
        self.cycles += 8;
        value
    }
    fn swap(&mut self, value: u8) -> u8 {
        let result = (value >> 4) | (value << 4);
        let mut f = self.registers.get_f() & !(FLAGS::N as u8 | FLAGS::H as u8 | FLAGS::C as u8);

        if result == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }
        self.registers.set_f(f);
        self.cycles += 8;
        result
    }
    fn right_shift(&mut self, mut value: u8) -> u8 {
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
        self.cycles += 8;

        value
    }
    fn bit(&mut self, value: u8, bit: u8) {
        let tested_bit = value & (1 << bit);

        let mut f = self.registers.get_f() | FLAGS::H as u8;
        f &= !(FLAGS::N as u8);

        if tested_bit == 0 {
            f |= FLAGS::Z as u8;
        } else {
            f &= !(FLAGS::Z as u8);
        }

        self.registers.set_f(f);
        self.cycles += 8;
    }
}
