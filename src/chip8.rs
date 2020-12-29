use std::io::prelude::*;
use std::fs::File;
use rand::{Rng, rngs::ThreadRng};
use std::convert::TryInto;

const CHARSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

///Helper variables that aren't part of chip8 definition:
const FLAG: usize = 15; // Index to the 16th V register.
const ROMTOP: usize = 512;

pub struct Chip8 {
    /// The Chip8 has 4k of memory
    memory: [u8; 4096],

    /// The chip8 includes a hexadecimal charset in binary form where
    /// each character is of size 5x8 bits.
    //charset: [u8; 80],

    /// There are 16 general purpose 8-bit registers, which are used for most operations.
    /// The 16th register - V(F) - is a special 'Flag' register and shouldn't be used
    /// by programs directly as it's value is dependent on some instructions.
    reg_v: [u8; 16],

    /// The chip8 has a stack space for 16 16-bit addresses.
    stack: [u16; 16],

    ///The 16-bit stack pointer is used to point to the top of the Stack space.
    reg_sp: u16,

    /// A 16 bit general purpose register used to store memory addresses. Only 12
    /// bits are actually used.
    reg_i: u16,

    /// The Program Counter is an internal register and can't be used by chip8 programs.
    reg_pc: u16,

    /// These 8 bit registers are used as timers. They are auto-decremented @ 60Hz,
    /// when they are non-zero. When ST is non-zero, the chip8 produces a 'tone'.
    /// NOTE: These registers are to be auto-decremented *external* to the chip8. 
    reg_dt: u8,
    reg_st: u8,

    /// Holds the value of the key currently being pressed.
    key_pressed: u8,

    // Undocumented behaviour that's required by certain programs to run correctly.
    pub shift_using_vy: bool,
    pub increment_i_on_ld: bool,

    /// The display memory of chip8.
    display: [u8; 64 * 32],

    // Used for the RND instruction.
    rng: ThreadRng,
 }

 impl Chip8 {
    pub fn new() -> Chip8 {
            let mut chip8 = Chip8{ 
                memory: [0; 4096],
                stack: [0; 16],
                display: [0; 64 * 32],
                reg_v: [0; 16],
                reg_sp: 0,
                reg_i: 0,
                reg_pc: 0x200,
                reg_dt: 0,
                reg_st: 0,
                key_pressed: 0,
                shift_using_vy: false,
                increment_i_on_ld: false,
                rng: rand::thread_rng(),
            };

            for i in 0 .. 80 {
                chip8.memory[i] = CHARSET[i];
            }

            for i in 80 .. 4096 {
                chip8.memory[i] = 0;
            }

            chip8
    }

    pub fn set_key_pressed(&mut self, key: u8) {
        self.key_pressed = key;
    }

    pub fn get_display_data(self: &Self) -> &[u8] {
        &self.display
    }

    pub fn clear_display(self: &mut Self) {
        for i in 0 .. 64 * 32 {
            self.display[i] = 0;
        }
    }

    pub fn update_timers(self: &mut Self) {
        if self.reg_dt > 0 {
            self.reg_dt -= 1;
        }

        if self.reg_st > 0 {
            self.reg_st -= 1;
        }
    }
    /// Lots of Rust-y things going on here:
    /// The method needs to return a Result because both File::open and File::read do so,
    /// as signified by the ? operator at the end of the respective functions.
    /// See https://m4rw3r.github.io/rust-questionmark-operator for reference.
    pub fn boot_rom(self: &mut Self, file_name: &str) -> std::io::Result<()> {
        let mut f = File::open(file_name)?;
        let file_len: usize = f.metadata().unwrap().len() as usize;
        let n = f.read(&mut self.memory[ROMTOP .. ROMTOP + file_len])?;
        //println!("{:?}", &self.memory[ROMTOP .. ROMTOP + 10]);
        if n != file_len {
            println!("There was an error reading the ROM. Read {}. Expected {}.", n, file_len);
        }

        self.key_pressed = 0xff;
        self.reg_sp = 0;
        self.reg_i = 0;
        self.reg_pc = ROMTOP as u16;
        self.reg_dt = 0;
        self.reg_st = 0;

        for i in 0 .. 16 {
            self.stack[i] = 0;
            self.reg_v[i] = 0;
        }

        self.clear_display();
        println!("Loaded Chip8 ROM: {}", file_name);

        Ok(())
    }

    pub fn step(self: &mut Self) {
        // Big-endian order
        let high_byte = self.memory[self.reg_pc as usize];
        let low_byte = self.memory[(self.reg_pc + 1) as usize];
        let opcode: u16 = ((high_byte as u16) << 8) | (low_byte as u16); 
        self.reg_pc += 2;
        // display[rand() % 200] = rand() % 16384;
        // cache common operations
        let nnn: u16 = opcode & 0x0fff;
        let xh: u16 = (opcode & 0xf000) >> 12;
        let x: usize = ((opcode & 0x0f00) >> 8).into();
        let y: usize = ((opcode & 0x00f0) >> 4).into();
        let kk: u8 = (opcode & 0x00ff).try_into().unwrap();
        let n: u16 = opcode & 0x0f;

        match xh {
            0x0 => {
                match opcode {
                    // CLS
                    0x00E0 => {
                        self.clear_display();
                    }
                    // RET
                    0x00EE => {
                        self.reg_pc = self.stack[self.reg_sp as usize];
                        self.reg_sp -= 1;
                    }
                    _ => {
                        println!("Unsupported instruction: {} ", opcode);
                    }
                }
            }
            // JP addr
            0x1 => {
                self.reg_pc = nnn;
            }
            // JP addr
            0x2 => {
                self.reg_sp += 1;
                self.stack[self.reg_sp as usize] = self.reg_pc;
                self.reg_pc = nnn;
            }
            // SE Vx, byte
            0x3 => { 
                if self.reg_v[x] == kk {
                   self.reg_pc += 2;
                }
            }
            // SNE Vx, byte
            0x4 => {
                if self.reg_v[x] != kk {
                   self.reg_pc += 2;
                }
            }
            // SE Vx, Vy
            0x5 => { 
                if (n == 0) && (self.reg_v[x] == self.reg_v[y]) {
                    self.reg_pc += 2;
                }
            }
            // LD Vx, byte
            0x6 => {
                self.reg_v[x] = kk;
            }
            // ADD Vx, byte
            0x7 => { 
                self.reg_v[x] = self.reg_v[x].wrapping_add(kk);
            }
        
            0x8 => {
                match n {
                    // LD Vx, Vy
                    0x0 => { 
                        self.reg_v[x] = self.reg_v[y];
                    }
                    // OR Vx, Vy
                    0x1 => { 
                        self.reg_v[x] |= self.reg_v[y];
                    }
                    // AND Vx, Vy
                    0x2 => {
                        self.reg_v[x] &= self.reg_v[y];
                    }
                    // XOR Vx, Vy
                    0x3 => { 
                        self.reg_v[x] ^= self.reg_v[y];
                    }
                    // ADD Vx, Vy
                    0x4 => {
                        let (result, carry) = self.reg_v[x].overflowing_add(self.reg_v[y]);
                        self.reg_v[x] = result;
                        self.reg_v[FLAG] = if carry {1} else {0};
                    }
                    // SUB Vx, Vy
                    0x5 => {
                        self.reg_v[FLAG] = if self.reg_v[y] > self.reg_v[x] {0} else {1};
                        self.reg_v[x] = self.reg_v[x].wrapping_sub(self.reg_v[y]);
                    }
                    // SHR Vx {, Vy}
                    0x6 => { 
                        if !self.shift_using_vy {
                            self.reg_v[FLAG] = self.reg_v[x] & 0x01;
                            self.reg_v[x] >>= 1;
                        }
                        else {
                            self.reg_v[FLAG] = self.reg_v[y] & 0x01;
                            self.reg_v[x] = self.reg_v[y] >> 1;
                        }
                    }
                    // SUBN Vx, Vy
                    0x7 => { 
                        self.reg_v[FLAG] = if self.reg_v[x] > self.reg_v[y] {0} else {1};
                        self.reg_v[x] = self.reg_v[y].wrapping_sub(self.reg_v[x]);
                    }
                    // SHL Vx {,Vy}
                    0xE => {
                        if !self.shift_using_vy {
                            self.reg_v[FLAG] = (self.reg_v[x] & 0x80) >> 7;
                            self.reg_v[x] <<= 1;
                        }
                        else {
                            self.reg_v[FLAG] = (self.reg_v[y] & 0x80) >> 7;
                            self.reg_v[x] = self.reg_v[y] << 1;
                        }
                    }
                    _ => {
                        println!("Uknown instruction: {}", opcode);
                    }
                }
            }
            // SNE Vx, Vy
            0x9 => { 
                if (n == 0) && (self.reg_v[x] != self.reg_v[y]) {
                    self.reg_pc += 2;
                }
            }
            // LD I, addr
            0xa => { 
                self.reg_i = nnn;
            }
            // JP V0 + addr
            0xb => { 
                self.reg_pc = nnn.wrapping_add(self.reg_v[0] as u16);
            }
            // RND Vx, byte
            0xc => { 
                let r: u8 = self.rng.gen();
                self.reg_v[x] = r & kk;
            }
            // DRW Vx, Vy, nibble
            0xd => { 
                self.reg_v[FLAG] = 0;
                
                for c in 0 .. n {
                    let mut sprite = self.memory[(self.reg_i + c) as usize];
                    let row = ((self.reg_v[y] as u16) + c) % 32;

                    for f in 0 .. 8 {
                        let b = (sprite & 0x80) >> 7;
                        let col = (self.reg_v[x] + f) % 64;
                        let offset = (row * 64 + (col as u16)) as usize;

                        if b == 1 {
                            if self.display[offset] != 0 {
                                self.display[offset] = 0;
                                self.reg_v[FLAG] = 1;
                            }
                            else {
                                self.display[offset] = 1;
                            }
                        }

                        sprite <<= 1;
                    }
                }
            }
            0xe => {
                match kk {
                    // SKP Vx
                    0x9e => { 
                        if self.key_pressed == self.reg_v[x] {
                           self.reg_pc += 2;
                        }
                    }
                    // SKNP Vx
                    0xA1 => { 
                        if self.key_pressed != self.reg_v[x] {
                           self.reg_pc += 2;
                        }

                    }
                    _ => {
                        println!("Uknown instruction: {}", opcode);
                    }
                }
            }
            0xf => {
                match kk {
                    // LD Vx, DT
                    0x07 => {
                        self.reg_v[x] = self.reg_dt;
                    }
                    // LD Vx, K
                    0x0a => { 
                        if self.key_pressed != 0xff {
                           self.reg_v[x] = self.key_pressed;
                        }
                        else {
                            self.reg_pc -= 2;
                        }
                    }
                    // LD DT, Vx
                    0x15 => { 
                        self.reg_dt = self.reg_v[x];
                    }
                    // LD ST, Vx
                    0x18 => { 
                        self.reg_st = self.reg_v[x];
                    }
                    // ADD I, Vx
                    0x1e => { 
                        // From Wikipedia:
                        // VF is set to 1 when there is a range overflow (I+VX>0xFFF), and to
                        // 0 when there isn't. This is an undocumented feature of the CHIP - 8
                        // and used by the Spacefight 2091!game
                        let add = self.reg_i + (self.reg_v[x] as u16);
                        self.reg_v[FLAG] = if add > 0xfff {1} else {0};
                        self.reg_i = add & 0xfff;
                    }
                    // LD F, Vx
                    0x29 => { 
                        self.reg_i = (self.reg_v[x] * 5).into();
                        self.reg_i &= 0xfff;
                    }
                    // LD B, Vx
                    0x33 => { 
                        let mut bcd = self.reg_v[x];
                        let unit = bcd % 10;
                        bcd = bcd / 10;
                        let tens = bcd % 10;
                        bcd = bcd / 10;
                        let hundreds = bcd % 10;
                        let i = self.reg_i as usize;
                        self.memory[i] = hundreds;
                        self.memory[i + 1] = tens;
                        self.memory[i + 2] = unit;
                    }
                    // LD [I], Vx
                    0x55 => {
                        let i = self.reg_i as usize;

                        for a in 0 .. x+1 {
                           self.memory[i + a] = self.reg_v[a];
                        }

                        if self.increment_i_on_ld {
                            self.reg_i += (x + 1) as u16;
                        }
                    }
                    // LD Vx, [I]
                    0x65 => { 
                        let i = self.reg_i as usize;

                        for a in 0 .. x+1  {
                            self.reg_v[a] = self.memory[i + a];
                        }

                        if self.increment_i_on_ld {
                            self.reg_i += (x + 1) as u16;
                        }
                    }
                    _ => {
                        println!("Unknown instruction: {}", opcode);
                    }
                }
            }
            _ => {
                println!("Unknown instruction: {}", opcode);
            }
        }
    }
}  
