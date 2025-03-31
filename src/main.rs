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
        self.f
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

struct CPU
{
    registers: REGISTERS,
    ime: bool, //interrupt master enable
    opcode: u8,
    cycles: u64
}
impl CPU
{
    fn new()
    {
        //CPU {registers: REGISTERS { a: 0, b: 0, c: 0, d: 0, e: 0, f: 0, h: 0, l: 0, pc: 0, sp: 0 },ime: true}
    }
    fn fetch(&mut self) -> u8
    {
        self.opcode
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
