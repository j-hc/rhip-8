#![allow(dead_code)]
#![allow(clippy::single_match)]

use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
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

fn main() {
    let mut emu = Emu::new();
    let rom = std::fs::read("./roms/IBM Logo.ch8").unwrap();
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

    'app: loop {
        renderer.set_draw_color(Color::BLACK);
        renderer.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'app,
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

        thread::sleep(Duration::from_nanos(1_000_000_000 / HZ));
    }
}

struct Emu {
    gfx: [u8; WIDTH * HEIGHT],
    memory: [u8; 2 << 11],
    pc: usize,
    i: u16,
    stack: Vec<u16>,
    sp: usize,
    delay_t: u8,
    sound_t: u8,
    keypad: [u8; 16],
    regs: [u8; 16],
}

impl Emu {
    fn new() -> Self {
        let mut memory = [0u8; 2 << 11];
        memory[0x50..=0x9F].copy_from_slice(&FONTSET);

        Self {
            gfx: [0u8; WIDTH * HEIGHT],
            memory,
            pc: 0x200,
            i: 0,
            stack: Vec::new(),
            sp: 0,
            delay_t: 0,
            sound_t: 0,
            keypad: [0u8; 16],
            regs: [0u8; 16],
        }
    }

    fn fetch(&mut self) -> u16 {
        let opcode: u16 = (self.memory[self.pc] as u16) << 8 | self.memory[self.pc + 1] as u16;
        self.pc += 2;
        opcode
    }

    fn load_rom(&mut self, rom: &[u8]) {
        let pc = self.pc as usize;
        self.memory[pc..pc + rom.len()].copy_from_slice(rom);
    }

    fn exec(&mut self, ins: u16) {
        let x = (ins >> 8) & 0xF;
        let y = (ins >> 4) & 0xF;
        let n = ins & 0xF;
        let nn = ins & 0xFF;
        let nnn = ins & 0xFFF;

        match ins >> 0xC {
            0xE0 => {
                self.gfx.fill(0);
            }
            0x1 => self.pc = nnn as usize, // 1NNN jump to NNN
            0x6 => self.regs[x as usize] = nn as u8, // 6XNN set register VX to NN
            // 7XNN add value NN to register VX, saturate in case of overflow
            0x7 => self.regs[x as usize] = self.regs[x as usize].saturating_add(nn as u8),
            0xA => self.i = nnn, // ANNN set I to NNN
            0xD => {
                // DXYN draw
                let sprite_x = self.regs[x as usize] as usize % WIDTH;
                let sprite_y = self.regs[y as usize] as usize % HEIGHT;
                let sprite_height = n;

                self.regs[0xF] = 0;

                for coord_y in 0..sprite_height {
                    let coord_y = coord_y as usize;
                    if sprite_y + coord_y >= HEIGHT {
                        break;
                    }
                    let sprite = self.memory[self.i as usize + coord_y];
                    for coord_x in 0..8 {
                        if sprite_x + coord_x >= WIDTH {
                            break;
                        }
                        let bit = sprite >> (7 - coord_x) & 0x1;
                        if bit != 0 {
                            let idx = WIDTH * (sprite_y + coord_y) + sprite_x + coord_x;
                            let pixel = self.gfx.get_mut(idx as usize).unwrap();
                            if *pixel != 0 {
                                *pixel = 0;
                                self.regs[0xF] = 1;
                            } else if *pixel == 0 {
                                *pixel = 1;
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}
