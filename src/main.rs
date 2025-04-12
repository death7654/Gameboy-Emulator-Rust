use std::net::AddrParseError;


const DISPLAY_HEIGHT: usize = 144;
const DISPLAY_WIDTH: usize = 160;



struct REGISTERS
{
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    pc: u16,
    sp: u16
}
impl REGISTERS
{
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

    fn set_a(&mut self, val: u8) {
        self.a = val;
    }

    fn get_b(&self) -> u8 {
        self.b
    }

    fn set_b(&mut self, val: u8) {
        self.b = val;
    }

    fn get_c(&self) -> u8 {
        self.c
    }

    fn set_c(&mut self, val: u8) {
        self.c = val
    }
    fn get_d(&self) -> u8 {
        self.d
    }

    fn set_d(&mut self, val: u8) {
        self.d = val;
    }

    fn get_e(&self) -> u8 {
        self.e
    }

    fn set_e(&mut self, val: u8) {
        self.e = val;
    }

    fn get_f(&self) -> u8 {
        self.f
    }

    fn set_f(&mut self, val: u8) {
        self.f = val & 0xF0
    }
    fn get_h(&self) -> u8 {
        self.h
    }

    fn set_h(&mut self, val: u8) {
        self.h = val
    }
    fn get_l(&self) -> u8 {
        self.l
    }

    fn set_l(&mut self, val: u8) {
        self.l = val
    }
    fn get_pc(&self) -> u16 {
        self.pc
    }

    fn set_pc(&mut self, val: u16) {
        self.pc = val
    }
    fn get_sp(&self) -> u16 {
        self.sp
    }

    fn set_sp(&mut self, val: u16) {
        self.sp = val
    }

    //since b, c, d, e, h, l can act as one 16 bit instuction we need to add code for that
    fn get_bc(&self) ->u16
    {
        let mut result: u16;
        let reg1 = self.b as u16;
        let reg2 = self.c as u16;
        result = reg1 << 8;
        result = result | reg2;
        result
    }
    fn set_bc(&mut self, val: u16)
    {
        self.b = (val >> 8) as u8;
        self.c = val as u8;
    }
    fn get_de(&self) ->u16
    {
        let mut result: u16;
        let reg1 = self.d as u16;
        let reg2 = self.e as u16;
        result = reg1 << 8;
        result = result | reg2;
        result
    }
    fn set_de(&mut self, val: u16)
    {
        self.d = (val >> 8) as u8;
        self.e = val as u8;
    }
    fn get_hl(&self) ->u16
    {
        let mut result: u16;
        let reg1 = self.h as u16;
        let reg2 = self.l as u16;
        result = reg1 << 8;
        result = result | reg2;
        result
    }
    fn set_hl(&mut self, val: u16)
    {
        self.h = (val >> 8) as u8;
        self.l = val as u8;
    }
    fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | self.f as u16
    }
    fn set_af(&mut self, val: u16) {
        self.a = (val >> 8) as u8;
        self.f = (val & 0x00F0) as u8;
    }
    fn get_and_inc_pc(&mut self) -> u16 {
        let ret_pc = self.pc;
        self.pc += 1;
        ret_pc
    }

    fn inc_pc(&mut self) -> u16
    {
        self.pc +=1;
        self.pc
    }


    
}
enum FLAGS {
    Z = 0b1000_0000,
    N = 0b0100_0000,
    H = 0b0010_0000,
    C = 0b0001_0000,
}

struct CPU<'a>
{
    registers: REGISTERS,
    ime: bool, //interrupt master enable
    opcode: u8,
    cycles: u64,
    ram: &'a mut RAM, 
}
impl<'a> CPU<'a>
{
    fn new(ram: &'a mut RAM) -> Self
    {
        CPU{ registers: REGISTERS::new(), ime: true, opcode: 0, cycles: 0, ram}
    }
    fn fetch(&mut self, ram: &RAM) -> u8
    {
        let opcode = ram.read(self.registers.pc);
        self.registers.pc += 1;
        opcode
    }
    fn execute(&mut self, opcode: u8)
    {
        match opcode
        {
            0x00 => {
                //NOP
                self.cycles +=4;
                return;
            },
            0x01 =>
            {
                //Load 2 bytes into register BC
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let data = ((upper_byte as u16) <<8)| (lower_byte as u16);
                self.registers.set_bc(data);
                self.cycles+=12;
            }
            0x02 =>
            {
                //load the data in a into the ram address found in bc
                let address = self.registers.get_bc();
                let data = self.registers.get_a();
                self.ram.write(address, data);
                self.cycles +=8;
            }
            0x03 =>
            {
                //increment bc
                let bc = self.registers.get_bc();
                self.registers.set_bc((bc.wrapping_add(1)));
                self.cycles+=8;
            }
            0x04 =>
            {
                //increment B
                let b = self.registers.get_b();
                let value = b.wrapping_add(1);
                self.registers.set_b(value);

                //implement flags
                //Z
                if value == 0
                {
                    self.registers.set_f(self.registers.get_f() | FLAGS::Z as u8);
                }
                else {
                    self.registers.set_f(self.registers.get_f() & !(FLAGS::Z as u8));

                }

                //H
                if (b & 0x0F) + 1 > 0x0F {
                    self.registers.set_f(self.registers.get_f() | FLAGS::H as u8);
                } else {
                    self.registers.set_f(self.registers.get_f() & !(FLAGS::H as u8));
                }

                //N
                self.registers.set_f(self.registers.get_f() & !(FLAGS::N as u8));

                self.cycles +=4;
            }
            0x05 =>
            {
                //decrement b
                let b = self.registers.get_b();
                let value = b.wrapping_sub(1);
                self.registers.set_b(value);

                //Z flag
                if value == 0
                {
                    self.registers.set_f(self.registers.get_f() | FLAGS::Z as u8);
                }
                else {
                    self.registers.set_f(self.registers.get_f() & !(FLAGS::Z as u8));
                }

                //H Flag
                if (b & 0x0F) == 0 {
                    self.registers.set_f(self.registers.get_f() | FLAGS::H as u8);
                } else {
                    self.registers.set_f(self.registers.get_f() & !(FLAGS::H as u8));
                }

                //N
                self.registers.set_f(self.registers.get_f() | (FLAGS::N as u8));

                self.cycles +=4;

            },
            0x06 =>
            {
                //load 1 byte into B
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_b( data);
                self.cycles +=8;
            },
            0x07 =>
            {
                println!("0x07 is not implemented");
            }
            0x08 => 
            {
                //load load sp into the address from ram
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let address = ((upper_byte as u16) << 8) | lower_byte as u16;
                let sp = self.registers.get_sp();
                self.ram.write(address, (sp & 0x00FF) as u8);
                self.ram.write(address + 1, (sp >> 8) as u8);

                self.cycles += 20;

            },
            0x09 =>
            {
                //not implemented
            }
            0x0A =>
            {
                let data = self.ram.read(self.registers.get_bc());
                self.registers.set_a(data);
                self.cycles +=8;
            }
            0x0B =>
            {
                //decrement BC
                let data = self.registers.get_bc();
                self.registers.set_bc(data.wrapping_sub(1));
                self.cycles+=8;
            },
            0x0C =>
            {
                //not implemented
            }
            0x0D =>
            {
                //not implemented
            }
            0x0E=>
            {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_c(data);
                self.cycles+=8;

            },
            0x0F =>
            {
                //not implemented
            },
            0x10 =>
            {
                //not implemented, stop
            },
            0x11 =>
            {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                
                let data = ((upper_byte as u16) << 8) | lower_byte as u16;

                self.registers.set_de(data);
                self.cycles+=12;
            },
            0x12 =>
            {
                let address = self.registers.get_de();
                let data = self.registers.get_a();

                self.ram.write(address, data);
                self.cycles+=8;
            },
            0x13 =>
            {
                let value = self.registers.get_de();
                self.registers.set_de(value.wrapping_add(1));
                self.cycles+=8;
            }
            0x14 =>
            {
                //not implemented
            }
            0x15 =>
            {
                //not implemented
            }
            0x16 =>
            {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_d(data);
                self.cycles +=8;
            }
            0x17 =>
            {
                //not implemented
            }
            0x18 =>
            {
                //relative jump
                let jump = self.ram.read(self.registers.get_and_inc_pc()) as i8;
                self.registers.set_pc(self.registers.get_pc().wrapping_add(jump as u16));
                self.cycles +=12;
            }
            0x19 =>
            {
                //not implemented
            }
            0x1A =>
            {
                let data = self.ram.read(self.registers.get_de());
                self.registers.set_a(data);
                self.cycles+=8;
            }
            0x1B =>
            {
                //decrement de
                self.registers.set_de(self.registers.get_de().wrapping_sub(1));
                self.cycles+=8;
            }
            0x1C =>
            {
                //not implemented
            }
            0x1E =>
            {
                //load the next byte onto register E
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_e(data);
                self.cycles+=8;

            }
            0x1F =>
            {
                //not implemented
            }
            0x20 =>
            {
                //not implemented
            }
            0x21 =>
            {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());

                let data = ((upper_byte as u16) << 8) | lower_byte as u16;

                self.registers.set_hl(data);
                self.cycles +=12;

            }
            0x22 =>
            {
                //load a into memory with the address found in HL and increment HL by 1
                let data = self.registers.get_a();
                let address = self.registers.get_hl();
                self.ram.write(address, data);

                self.cycles +=8;

            }
            0x23 =>
            {
                self.registers.set_hl(self.registers.get_hl().wrapping_add(1));
                self.cycles +=8;
            }
            0x24 =>
            {
                //not implemented
            }
            0x25 => 
            {
                //not implemented
            }
            0x26 =>
            {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_h(data);
                self.cycles +=8;
            }
            0x27 =>
            {
                //not implemented
            }
            0x29 =>
            {
                //not implemented
            }
            0x2A =>
            {
                let data = self.ram.read(self.registers.get_hl());
                self.registers.set_hl(self.registers.get_hl().wrapping_add(1));
                self.registers.set_a(data);
                self.cycles +=8;
                
            }
            0x2B =>
            {
                self.registers.set_hl(self.registers.get_hl().wrapping_sub(1));
                self.cycles +=8;
            }
            0x2C =>
            {
                //not implemented
            }
            0x2D =>
            {
                //not implemented
            }
            0x2E =>
            {
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_l(data);
                self.cycles +=8;
            }
            0x2F =>
            {
                //not implemented
            }
            0x30 =>
            {
                //not implemented
            }
            0x31 =>
            {
                let lower_byte = self.ram.read(self.registers.get_and_inc_pc());
                let upper_byte = self.ram.read(self.registers.get_and_inc_pc());
                let data = ((upper_byte as u16)<<8)|lower_byte as u16;
                self.registers.set_sp(data);
                self.cycles +=12;
            }
            0x32 =>
            {
                let data = self.registers.get_a();
                self.ram.write(self.registers.get_hl(), data);
                self.registers.set_hl(self.registers.get_hl().wrapping_sub(1));

                self.cycles+=8;
            }
            0x33 =>
            {
                self.registers.set_sp(self.registers.get_sp().wrapping_add(1));
                self.cycles+=8;
            }
            0x34 =>
            {
                //not implmented
            }
            0x35 =>
            {
                //not implemented
            }
            0x36 =>
            {
                let address = self.registers.get_hl();
                let data = self.ram.read(self.registers.get_and_inc_pc());
                self.ram.write(address, data);
                self.cycles+=12;
            }
            0x37 =>
            {
                //not implemented
            }
            0x38 =>
            {
                //not implemented
            }
            0x39 =>
            {
                //not impleented
            }
            0x3A =>
            {
                let data = self.ram.read(self.registers.get_hl());
                self.registers.set_hl(self.registers.get_hl().wrapping_sub(1));

                self.registers.set_a(data);
                self.cycles+=8;
            }
            0x3B =>
            {
                self.registers.set_sp(self.registers.get_sp().wrapping_sub(1));
                self.cycles+=8;
            }
            0x3C =>
            {
                //not implemented
            }
            0x3D =>
            {
                //not implemented
            }
            0x3E =>
            {
                let value = self.ram.read(self.registers.get_and_inc_pc());
                self.registers.set_a(value);
                self.cycles+=8;
            }
            0x3F =>
            {
                //not implemented
            }
            0x40 =>
            {
                self.cycles +=4;
            }
            0x41 =>
            {
                self.registers.set_b(self.registers.get_c());
                self.cycles+=4;
            }
            0x42 =>
            {
                self.registers.set_b(self.registers.get_d());
                self.cycles +=4;
            }
            0x43 =>
            {
                self.registers.set_b(self.registers.get_e());
                self.cycles +=4;
            }
            0x44 =>
            {
                self.registers.set_b(self.registers.get_h());
                self.cycles +=4;
            }
            0x45 =>
            {
                self.registers.set_b(self.registers.get_l());
                self.cycles +=4;
            }
            0x46 =>
            {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_b(value);
                self.cycles+=4;

            }
            0x47 =>
            {
                self.registers.set_b(self.registers.get_a());
                self.cycles +=4;
            }
            0x48 =>
            {
                self.registers.set_c(self.registers.get_b());
                self.cycles +=4;
            }
            0x49 =>
            {
                self.cycles+=4;
            }
            0x4A =>
            {
                self.registers.set_c(self.registers.get_d());
                self.cycles +=4;
            }
            0x4B =>
            {
                self.registers.set_c(self.registers.get_e());
                self.cycles +=4;
            }
            0x4C =>
            {
                self.registers.set_c(self.registers.get_h());
                self.cycles +=4;
            }
            0x4D =>
            {
                self.registers.set_c(self.registers.get_l());
                self.cycles +=4;
            }
            0x4E =>
            {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_c(value);
                self.cycles +=8;
            }
            0x4F =>
            {
                self.registers.set_c(self.registers.get_a());
                self.cycles +=4;
            }
            0x50 =>
            {
                self.registers.set_d(self.registers.get_b());
                self.cycles +=4;
            }
            0x51 =>
            {
                self.registers.set_d(self.registers.get_c());
                self.cycles +=4;
            }
            0x52 =>
            {
                self.cycles+=4;
            }
            0x53 =>
            {
                self.registers.set_d(self.registers.get_e());
                self.cycles +=4;
            }
            0x54 =>
            {
                self.registers.set_d(self.registers.get_h());
                self.cycles +=4;
            }
            0x55 =>
            {
                self.registers.set_d(self.registers.get_l());
                self.cycles +=4;
            }
            0x56 =>
            {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_d(value);
                self.cycles+=8;
            }
            0x57 =>
            {
                self.registers.set_d(self.registers.get_a());
                self.cycles +=4;
            }
            0x58 =>
            {
                self.registers.set_e(self.registers.get_b());
                self.cycles +=4;
            }
            0x59 =>
            {
                self.registers.set_e(self.registers.get_c());
                self.cycles +=4;
            }
            0x5A =>
            {
                self.registers.set_e(self.registers.get_d());
                self.cycles +=4;
            }
            0x5B =>
            {
                self.cycles +=4;
            }
            0x5C =>
            {
                self.registers.set_e(self.registers.get_h());
                self.cycles +=4;
            }
            0x5D =>
            {
                self.registers.set_e(self.registers.get_l());
                self.cycles +=4;
            }
            0x5E =>
            {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_e(value);
                self.cycles +=8;
            }
            0x5F =>
            {
                self.registers.set_e(self.registers.get_a());
                self.cycles +=4;
            }
            0x60 =>
            {
                self.registers.set_h(self.registers.get_b());
                self.cycles +=4;
            }
            0x61 =>
            {
                self.registers.set_h(self.registers.get_c());
                self.cycles +=4;
            }
            0x62 =>
            {
                self.registers.set_h(self.registers.get_d());
                self.cycles +=4;
            }
            0x63 =>
            {
                self.registers.set_h(self.registers.get_e());
                self.cycles +=4;
            }
            0x64 =>
            {
                self.cycles +=4;
            }
            0x65 =>
            {
                self.registers.set_h(self.registers.get_l());
                self.cycles +=4;
            }
            0x66 =>
            {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_h(value);
                self.cycles +=8;
            }
            0x67 =>
            {
                self.registers.set_h(self.registers.get_a());
                self.cycles +=4;
            }
            0x68 =>
            {
                self.registers.set_l(self.registers.get_b());
                self.cycles +=4;
            }
            0x69 =>
            {
                self.registers.set_l(self.registers.get_c());
                self.cycles +=4;
            }
            0x6A =>
            {
                self.registers.set_l(self.registers.get_d());
                self.cycles +=4;
            }
            0x6B => 
            {
                self.registers.set_l(self.registers.get_e());
                self.cycles +=4;
            }
            0x6C =>
            {
                self.registers.set_l(self.registers.get_h());
                self.cycles +=4;
            }
            0x6D =>
            {
                self.cycles +=4;
            }
            0x6E =>
            {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_l(value);
                self.cycles +=8;
            }
            0x6F =>
            {
                self.registers.set_l(self.registers.get_a());
                self.cycles +=4;
            }
            0x70 =>
            {
                self.ram.write(self.registers.get_hl(), self.registers.get_b());
                self.cycles +=8;
            }
            0x71 =>
            {
                self.ram.write(self.registers.get_hl(), self.registers.get_c());
                self.cycles +=8;
            }
            0x72 =>
            {
                self.ram.write(self.registers.get_hl(), self.registers.get_d());
                self.cycles +=8;
            }
            0x73 =>
            {
                self.ram.write(self.registers.get_hl(), self.registers.get_e());
                self.cycles +=8;
            }
            0x74 =>
            {
                self.ram.write(self.registers.get_hl(), self.registers.get_h());
                self.cycles +=8;
            }
            0x75 =>
            {
                self.ram.write(self.registers.get_hl(), self.registers.get_l());
                self.cycles +=8;
            }
            0x76 =>
            {
                //halt, not implemented
            }
            0x76 =>
            {
                self.ram.write(self.registers.get_hl(), self.registers.get_a());
                self.cycles +=8;
            }
            0x78 =>
            {
                self.registers.set_a(self.registers.get_b());
                self.cycles +=4;
            }
            0x79 =>
            {
                self.registers.set_a(self.registers.get_c());
                self.cycles +=4;
            }
            0x7A =>
            {
                self.registers.set_a(self.registers.get_d());
                self.cycles +=4;
            }
            0x7B =>
            {
                self.registers.set_a(self.registers.get_e());
                self.cycles +=4;
            }
            0x7C =>
            {
                self.registers.set_a(self.registers.get_h());
                self.cycles +=4;
            }
            0x7D =>
            {
                self.registers.set_a(self.registers.get_l());
                self.cycles +=4;
            }
            0x7E =>
            {
                let value = self.ram.read(self.registers.get_hl());
                self.registers.set_a(value);
                self.cycles +=8;
            }
            0x7F =>
            {
                self.cycles +=4;
            }
            0x80 =>
            {
                //not implemented
            }
            0x81 =>
            {
                //not implemented
            }
            0x82 =>
            {
                //not implemented
            }
            0x83 =>
            {
                //not implemented
            }
            _ => println!("{} Not Implemented", opcode)
        }
        
    }
}


struct RAM{
    memory: [u8; 65536]
}
impl RAM
{
    fn new() -> Self
    {
        RAM { memory: [0; 65536] }
    }
    fn read(&self, address: u16) -> u8
    {
        self.memory[address as usize]
    }
    fn write(& mut self, address: u16, data: u8)
    {
        self.memory[address as usize] = data;
    }
}
fn main() {
    //creating ram
    let ram = RAM::new();
    println!("Hello, world!");
}
