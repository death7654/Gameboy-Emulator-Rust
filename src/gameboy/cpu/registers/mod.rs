// all the different flags,
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FLAGS {
    Z = 0b1000_0000,
    N = 0b0100_0000,
    H = 0b0010_0000,
    C = 0b0001_0000,
}
// the 10 registers
pub struct REGISTERS {
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
        // the default values after the boot rom is loaded in
        REGISTERS {
            a: 0x01,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            f: 0xB0,
            pc: 0x100,
            sp: 0xFFFE,
        }
    }
    // return the value stored in a
    pub fn get_a(&self) -> u8 {
        self.a
    }

    // set a
    pub fn set_a(&mut self, value: u8) {
        self.a = value;
    }


    pub fn get_b(&self) -> u8 {
        self.b
    }

    pub fn set_b(&mut self, value: u8) {
        self.b = value;
    }

    pub fn get_c(&self) -> u8 {
        self.c
    }

    pub fn set_c(&mut self, value: u8) {
        self.c = value
    }
    pub fn get_d(&self) -> u8 {
        self.d
    }

    pub fn set_d(&mut self, value: u8) {
        self.d = value;
    }

    pub fn get_e(&self) -> u8 {
        self.e
    }

    pub fn set_e(&mut self, value: u8) {
        self.e = value;
    }

    pub fn get_f(&self) -> u8 {
        self.f
    }

    pub fn set_f(&mut self, value: u8) {
        self.f = value & 0xF0
    }
    pub fn get_h(&self) -> u8 {
        self.h
    }

    pub fn set_h(&mut self, value: u8) {
        self.h = value
    }
    pub fn get_l(&self) -> u8 {
        self.l
    }

    pub fn set_l(&mut self, value: u8) {
        self.l = value
    }
    // get the program counter
    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    // set the program counter
    pub fn set_pc(&mut self, value: u16) {
        self.pc = value
    }

    // get the stack pointer
    pub fn get_sp(&self) -> u16 {
        self.sp
    }

    //set the stack pointer
    pub fn set_sp(&mut self, value: u16) {
        self.sp = value
    }

    //since b, c, d, e, h, l can act as one 16 bit instuction we need to add code for that
    pub fn get_bc(&self) -> u16 {
        compute_16bit_reg(self.get_b(),self.get_c())
    }
    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }
    pub fn get_de(&self) -> u16 {
        compute_16bit_reg(self.get_d(),self.get_e())

    }
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }
    pub fn get_hl(&self) -> u16 {
        compute_16bit_reg(self.get_h(),self.get_l())
    }
    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }
    pub fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | self.f as u16
    }
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0x00F0) as u8;
    }
    pub fn get_and_inc_pc(&mut self) -> u16 {
        let ret_pc = self.get_pc();
        self.set_pc(ret_pc.wrapping_add(1));
        ret_pc
    }

  
}

// helper functions
// computes the 16 bit value of 2 8-bit registers
fn compute_16bit_reg(reg1: u8, reg2: u8) -> u16
{
    return ((reg1 as u16)<<8) | (reg2 as u16);
}
