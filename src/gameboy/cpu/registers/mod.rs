#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FLAGS {
    Z = 0b1000_0000,
    N = 0b0100_0000,
    H = 0b0010_0000,
    C = 0b0001_0000,
}
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
    pub fn get_a(&self) -> u8 {
        self.a
    }

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
    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    pub fn set_pc(&mut self, value: u16) {
        self.pc = value
    }
    pub fn get_sp(&self) -> u16 {
        self.sp
    }

    pub fn set_sp(&mut self, value: u16) {
        self.sp = value
    }

    //since b, c, d, e, h, l can act as one 16 bit instuction we need to add code for that
    pub fn get_bc(&self) -> u16 {
        let mut result: u16;
        let reg1 = self.b as u16;
        let reg2 = self.c as u16;
        result = reg1 << 8;
        result = result | reg2;
        result
    }
    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }
    pub fn get_de(&self) -> u16 {
        let mut result: u16;
        let reg1 = self.d as u16;
        let reg2 = self.e as u16;
        result = reg1 << 8;
        result = result | reg2;
        result
    }
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }
    pub fn get_hl(&self) -> u16 {
        let mut result: u16;
        let reg1 = self.h as u16;
        let reg2 = self.l as u16;
        result = reg1 << 8;
        result = result | reg2;
        result
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
        let ret_pc = self.pc;
        self.pc += 1;
        ret_pc
    }

}
