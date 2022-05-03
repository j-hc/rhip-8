use crate::{KeyPad, MainLoopHandler, Stack, Timer};
use std::thread;
use std::time::Duration;

const FONTSET: [u8; 80] = [
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
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];
pub type GFX = [u8; WIDTH * HEIGHT];

const PRINT_STEPS: bool = false;
pub const HEIGHT: usize = 32;
pub const WIDTH: usize = 64;
const TIMER_FREQ: u32 = 60;

pub struct Emu<H> {
    gfx: GFX,
    memory: [u8; 2 << 11],
    pc: u16,
    idx_reg: u16,
    stack: Stack<u16, 64>,
    delay_timer: Timer,
    sound_timer: Timer,
    keypad: KeyPad,
    regs: [u8; 16],
    step: usize,
    hz: u32,
    main_loop_handler: H,
}

impl<H: MainLoopHandler> Emu<H> {
    const FONT_MAP_OFFSET: usize = 0x50;

    pub fn new(hz: u32, main_loop_handler: H) -> Self {
        let mut memory = [0u8; 2 << 11];
        assert_eq!(
            160,
            Self::FONT_MAP_OFFSET + FONTSET.len(),
            "smth wrong with the fontset"
        );
        memory[Self::FONT_MAP_OFFSET..Self::FONT_MAP_OFFSET + FONTSET.len()]
            .copy_from_slice(&FONTSET);

        Self {
            gfx: [0u8; WIDTH * HEIGHT],
            memory,
            pc: 0x200,
            idx_reg: 0,
            stack: Stack::new(),
            delay_timer: Timer::new(hz / TIMER_FREQ),
            sound_timer: Timer::new(hz / TIMER_FREQ),
            keypad: KeyPad::default(),
            regs: [0u8; 16],
            step: 0,
            hz,
            main_loop_handler,
        }
    }

    pub fn set_hz(&mut self, hz: u32) {
        self.hz = hz;
        self.delay_timer = Timer::new(hz / TIMER_FREQ);
        self.sound_timer = Timer::new(hz / TIMER_FREQ);
    }

    pub fn run(&mut self) {
        while !self.main_loop_handler.quit() {
            self.delay_timer.time();
            self.sound_timer.time();

            let ins = self.fetch();
            self.exec(ins);

            self.main_loop_handler.render(&self.gfx);
            self.main_loop_handler.handle_key(&mut self.keypad);
            self.main_loop_handler.beep(self.sound_timer.timer > 0);

            thread::sleep(Duration::from_micros((1_000_000 / self.hz) as u64));
        }
    }

    fn fetch(&mut self) -> u16 {
        let pc = self.pc as usize;
        let opcode: u16 = (self.memory[pc] as u16) << 8 | self.memory[pc + 1] as u16;
        self.pc += 2;
        opcode
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        let pc = self.pc as usize;
        self.memory[pc..pc + rom.len()].copy_from_slice(rom);
    }

    fn exec(&mut self, ins: u16) {
        let x = ((ins >> 8) & 0xF) as usize;
        let y = ((ins >> 4) & 0xF) as usize;
        let n = ins & 0xF;
        let nn = (ins & 0xFF) as u8;
        let nnn = ins & 0xFFF;

        if PRINT_STEPS {
            println!(
                "step: {}, x: {x:01x} y: {y:01x} n: {n:01x} nn: {nn:02x} nnn: {nnn:03x} instruction: {ins:04x}",
                self.step
            );
        }

        match ins >> 0xC {
            0x0 => match n {
                0x0 => self.gfx.fill(0),
                0xE => self.pc = self.stack.pop(),
                _ => (),
            },
            0x1 => self.pc = nnn as u16,    // 1NNN jump to NNN
            0x6 => self.regs[x] = nn as u8, // 6XNN set register VX to NN
            // 7XNN add value NN to register VX, wrap around in case of overflow
            0x7 => self.regs[x] = self.regs[x as usize].wrapping_add(nn),
            0xA => self.idx_reg = nnn, // ANNN set I to NNN
            0xB => self.pc = nnn + self.regs[0] as u16,
            0xC => self.regs[x] = rand::random::<u8>() & nn as u8,
            0xD => {
                // DXYN draw
                let sprite_x = self.regs[x] as usize % WIDTH;
                let sprite_y = self.regs[y] as usize % HEIGHT;
                let sprite_height = n as usize;

                self.regs[0xF] = 0;
                for coord_y in 0..sprite_height {
                    if sprite_y + coord_y >= HEIGHT {
                        break;
                    }
                    let sprite = self.memory[self.idx_reg as usize + coord_y];
                    for coord_x in 0..8 {
                        if sprite_x + coord_x >= WIDTH {
                            break;
                        }
                        let bit = sprite >> (7 - coord_x) & 0x1;
                        if bit != 0 {
                            let idx = WIDTH * (sprite_y + coord_y) + sprite_x + coord_x;
                            // it's safe trust me bro
                            let pixel = unsafe { self.gfx.get_unchecked_mut(idx as usize) };
                            if *pixel != 0 {
                                self.regs[0xF] = 1;
                            }
                            *pixel ^= 1;
                        }
                    }
                }
            }
            0xE => match nn {
                0x9E => {
                    if self.keypad.is_pressed(self.regs[x]) {
                        self.pc += 2;
                    }
                }
                0xA1 => {
                    if !self.keypad.is_pressed(self.regs[x]) {
                        self.pc += 2;
                    }
                }
                _ => (),
            },
            0xF => match nn {
                0x07 => self.regs[x] = self.delay_timer.timer,
                0x15 => self.delay_timer.set_timer(self.regs[x]),
                0x18 => self.sound_timer.set_timer(self.regs[x]),
                0x1E => {
                    if self.idx_reg + self.regs[x] as u16 > 0xFFF {
                        self.regs[0xF] = 1;
                    } else {
                        self.regs[0xF] = 0;
                    }
                    self.idx_reg = self.idx_reg.wrapping_add(self.regs[x] as u16);
                }
                0x0A => {
                    if let Some(key) = self.keypad.get_pressed() {
                        self.regs[x] = key as u8;
                    } else {
                        self.pc -= 2;
                    }
                }
                0x29 => {
                    const BITMAP_WIDENESS: u8 = 5;
                    self.idx_reg =
                        (Self::FONT_MAP_OFFSET + (self.regs[x] * BITMAP_WIDENESS) as usize) as u16;
                }
                0x33 => {
                    let num = self.regs[x];
                    self.memory[self.idx_reg as usize] = num / 100;
                    self.memory[self.idx_reg as usize + 1] = num / 10 % 10;
                    self.memory[self.idx_reg as usize + 2] = num % 10;
                }
                0x55 => {
                    for idx in 0..=x {
                        self.memory[self.idx_reg as usize + idx] = self.regs[idx];
                    }
                }
                0x65 => {
                    for idx in 0..=x {
                        self.regs[idx] = self.memory[self.idx_reg as usize + idx];
                    }
                }
                _ => (),
            },
            0x2 => {
                self.stack.push(self.pc);
                self.pc = nnn;
            }
            0x3 => {
                if self.regs[x] == nn {
                    self.pc += 2;
                }
            }
            0x4 => {
                if self.regs[x] != nn {
                    self.pc += 2;
                }
            }
            0x5 => {
                if self.regs[x] == self.regs[y] {
                    self.pc += 2;
                }
            }
            0x9 => {
                if self.regs[x] != self.regs[y] {
                    self.pc += 2;
                }
            }
            0x8 => match n {
                0x0 => self.regs[x] = self.regs[y],
                0x1 => self.regs[x] |= self.regs[y],
                0x2 => self.regs[x] &= self.regs[y],
                0x3 => self.regs[x] ^= self.regs[y],
                0x4 => {
                    let (sum, overflowed) = self.regs[x].overflowing_add(self.regs[y]);
                    self.regs[x] = sum;
                    self.regs[0xF] = overflowed as u8;
                }
                0x5 => {
                    let (sub, overflowed) = self.regs[x].overflowing_sub(self.regs[y]);
                    self.regs[x] = sub;
                    self.regs[0xF] = !overflowed as u8;
                }
                0x7 => {
                    let (sub, overflowed) = self.regs[y].overflowing_sub(self.regs[x]);
                    self.regs[x] = sub;
                    self.regs[0xF] = !overflowed as u8;
                }
                0x6 => {
                    // self.regs[x] = self.regs[y];  <-- enable optionally
                    let shifted_out = self.regs[x] & 1;
                    self.regs[x] >>= 1;
                    self.regs[0xF] = shifted_out;
                }
                0xE => {
                    // self.regs[x] = self.regs[y];  <-- enable optionally
                    let shifted_out = self.regs[x] >> 7;
                    self.regs[x] <<= 1;
                    self.regs[0xF] = shifted_out;
                }
                _ => (),
            },

            _ => (),
        }
        self.step += 1;
    }
}
