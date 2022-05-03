use sdl2::event::Event::{KeyDown, KeyUp, Quit};
use sdl2::keyboard::Scancode::{self, *};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::io::Write;
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

const HEIGHT: usize = 32;
const WIDTH: usize = 64;
const SCALER: usize = 15;

const HZ: u64 = 600;
const ROM_PATH: &str = "./roms/min.ch8";

fn main() {
    let mut emu = Emu::new();
    let rom = std::fs::read(ROM_PATH).unwrap();
    emu.load_rom(&rom);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("rhip-8", (WIDTH * SCALER) as u32, (HEIGHT * SCALER) as u32)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut renderer = window.into_canvas().build().unwrap();
    let mut delay_timer_decrementer = 0;
    'app: loop {
        renderer.set_draw_color(Color::BLACK);
        renderer.clear();

        if emu.delay_t > 0 {
            delay_timer_decrementer += 1;
            if delay_timer_decrementer == HZ / 60 {
                emu.delay_t -= 1;
                delay_timer_decrementer = 0;
            }
        }
        // println!("timer: {}", emu.delay_t);

        for event in event_pump.poll_iter() {
            match event {
                Quit { .. } => break 'app,
                KeyDown { scancode: Some(sc), .. } => emu.press_key(sc),
                KeyUp { scancode: Some(sc), .. } => emu.release_key(sc),
                _ => (),
            }
        }

        let ins = emu.fetch();
        emu.exec(ins);

        renderer.set_draw_color(Color::WHITE);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if emu.gfx[y * WIDTH + x] == 1 {
                    renderer
                        .fill_rect(Rect::new(
                            (x * SCALER) as i32,
                            (y * SCALER) as i32,
                            SCALER as u32,
                            SCALER as u32,
                        ))
                        .unwrap();
                }
            }
        }
        renderer.present();

        thread::sleep(Duration::from_micros(1_000_000 / HZ));
    }
}

struct Emu {
    gfx: [u8; WIDTH * HEIGHT],
    memory: [u8; 2 << 11],
    pc: u16,
    idx_reg: u16,
    stack: [u16; 2 << 6],
    sp: usize,
    delay_t: u8,
    sound_t: u8,
    keypad: [u8; 16],
    regs: [u8; 16],
    step: usize,
}

impl Emu {
    const FONT_MAP_OFFSET: usize = 0x50;

    fn new() -> Self {
        let mut memory = [0u8; 2 << 11];
        assert_eq!(
            160,
            Self::FONT_MAP_OFFSET + FONTSET.len(),
            "smth wrong with the fontset"
        );
        memory[Self::FONT_MAP_OFFSET..Self::FONT_MAP_OFFSET + FONTSET.len()].copy_from_slice(&FONTSET);

        Self {
            gfx: [0u8; WIDTH * HEIGHT],
            memory,
            pc: 0x200,
            idx_reg: 0,
            stack: [0u16; 2 << 6],
            sp: 0,
            delay_t: 0,
            sound_t: 0,
            keypad: [0u8; 16],
            regs: [0u8; 16],
            step: 0,
        }
    }

    #[inline]
    fn fetch(&mut self) -> u16 {
        let pc = self.pc as usize;
        let opcode: u16 = (self.memory[pc] as u16) << 8 | self.memory[pc + 1] as u16;
        self.pc += 2;
        opcode
    }

    #[inline]
    fn load_rom(&mut self, rom: &[u8]) {
        let pc = self.pc as usize;
        self.memory[pc..pc + rom.len()].copy_from_slice(rom);
    }

    pub fn press_key(&mut self, scancode: Scancode) {
        self.handle_key(scancode, 1)
    }

    pub fn release_key(&mut self, scancode: Scancode) {
        self.handle_key(scancode, 0)
    }

    fn handle_key(&mut self, scancode: Scancode, flag: u8) {
        match scancode {
            Num1 => self.keypad[0x1] = flag,
            Num2 => self.keypad[0x2] = flag,
            Num3 => self.keypad[0x3] = flag,
            Num4 => self.keypad[0xC] = flag,
            Q => self.keypad[0x4] = flag,
            W => self.keypad[0x5] = flag,
            E => self.keypad[0x6] = flag,
            R => self.keypad[0xD] = flag,
            A => self.keypad[0x7] = flag,
            S => self.keypad[0x8] = flag,
            D => self.keypad[0x9] = flag,
            F => self.keypad[0xE] = flag,
            Z => self.keypad[0xA] = flag,
            X => self.keypad[0x0] = flag,
            C => self.keypad[0xB] = flag,
            V => self.keypad[0xF] = flag,
            _ => (),
        }
    }

    fn exec(&mut self, ins: u16) {
        let x = ((ins >> 8) & 0xF) as usize;
        let y = ((ins >> 4) & 0xF) as usize;
        let n = ins & 0xF;
        let nn = (ins & 0xFF) as u8;
        let nnn = ins & 0xFFF;

        // println!(
        //     "step: {}, x: {x:01x} y: {y:01x} n: {n:01x} nn: {nn:02x} nnn: {nnn:03x} instruction: {ins:04x}",
        //     self.step
        // );

        match ins >> 0xC {
            0x0 => match n {
                0x0 => self.gfx.fill(0),
                0xE => {
                    if self.sp <= 0 {
                        panic!("stack underflow");
                    }
                    self.pc = self.stack[self.sp];
                    self.sp -= 1;
                }
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
                            let pixel = self.gfx.get_mut(idx as usize).unwrap();
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
                    if self.keypad[self.regs[x] as usize] == 1 {
                        self.pc += 2;
                    }
                }
                0xA1 => {
                    if self.keypad[self.regs[x] as usize] == 0 {
                        self.pc += 2;
                    }
                }
                _ => (),
            },
            0xF => match nn {
                0x07 => self.regs[x] = self.delay_t,
                0x15 => self.delay_t = self.regs[x],
                0x18 => self.sound_t = self.regs[x],
                0x1E => {
                    if self.idx_reg + self.regs[x] as u16 > 0xFFF {
                        self.regs[0xF] = 1;
                    } else {
                        self.regs[0xF] = 0;
                    }
                    self.idx_reg = self.idx_reg.wrapping_add(self.regs[x] as u16);
                }
                0x0A => {
                    println!("waiting");
                    if let Some(key) = self.keypad.iter().position(|k| k == &1) {
                        println!("found: {key}");
                        std::process::exit(1);
                        self.regs[x] = key as u8;
                    } else {
                        println!("nope");
                        self.pc -= 2;
                    }
                }
                0x29 => {
                    const BITMAP_WIDENESS: u8 = 5;
                    self.idx_reg = (Self::FONT_MAP_OFFSET + (self.regs[x] * BITMAP_WIDENESS) as usize) as u16;
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
                self.sp += 1;
                self.stack[self.sp] = self.pc;
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
