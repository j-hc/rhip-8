use rhip_8::{
    emu::{Emu, GFX, HEIGHT, WIDTH},
    Key, KeyPad, MainLoopHandler,
};
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use sdl2::event::Event::{KeyDown, KeyUp, Quit};
use sdl2::keyboard::Scancode::*;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::{EventPump, Sdl};

const ROM_PATH: &str = "./roms/IBM Logo.ch8";
const SCREEN_SCALER: usize = 20;
const HZ: u32 = 600;
const FG_COLOUR: Color = Color::WHITE;
const BG_COLOUR: Color = Color::BLACK;

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for s in out.iter_mut() {
            *s = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

struct SDLHandler<'a> {
    canvas: Canvas<Window>,
    event_pump: EventPump,
    audio_device: AudioDevice<SquareWave>,
    is_device_beeping: bool,
    _quit: bool,
    _sdl_context: &'a Sdl,
}
impl<'a> SDLHandler<'a> {
    fn new(sdl_context: &'a Sdl) -> Result<Self, Box<dyn std::error::Error>> {
        let video_subsystem = sdl_context.video()?;

        let audio_subsystem = sdl_context.audio()?;
        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1),
            samples: None,
        };
        let audio_device =
            audio_subsystem.open_playback(None, &desired_spec, |spec| SquareWave {
                phase_inc: 410.0 / spec.freq as f32,
                phase: 0.0,
                volume: 0.3,
            })?;

        let window = video_subsystem
            .window(
                "rhip-8",
                (WIDTH * SCREEN_SCALER) as u32,
                (HEIGHT * SCREEN_SCALER) as u32,
            )
            .position_centered()
            .opengl()
            .build()?;

        let event_pump = sdl_context.event_pump()?;
        let canvas = window.into_canvas().build()?;

        Ok(Self {
            canvas,
            event_pump,
            audio_device,
            is_device_beeping: false,
            _quit: false,
            _sdl_context: sdl_context,
        })
    }
}

impl<'a> MainLoopHandler for SDLHandler<'a> {
    fn render(&mut self, gfx: &GFX) {
        self.canvas.set_draw_color(BG_COLOUR);
        self.canvas.clear();

        self.canvas.set_draw_color(FG_COLOUR);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if gfx[y * WIDTH + x] == 1 {
                    self.canvas
                        .fill_rect(Rect::new(
                            (x * SCREEN_SCALER) as i32,
                            (y * SCREEN_SCALER) as i32,
                            SCREEN_SCALER as u32,
                            SCREEN_SCALER as u32,
                        ))
                        .unwrap();
                }
            }
        }
        self.canvas.present();
    }

    fn handle_key(&mut self, keypad: &mut KeyPad) {
        for event in self.event_pump.poll_iter() {
            let (scan_code, is_pressed) = match event {
                KeyDown {
                    scancode: Some(sc), ..
                } => (sc, true),
                KeyUp {
                    scancode: Some(sc), ..
                } => (sc, false),
                e => {
                    self._quit = matches!(e, Quit { .. });
                    continue;
                }
            };
            match scan_code {
                Num1 => keypad.set_key(Key::One, is_pressed),
                Num2 => keypad.set_key(Key::Two, is_pressed),
                Num3 => keypad.set_key(Key::Three, is_pressed),
                Num4 => keypad.set_key(Key::C, is_pressed),
                Q => keypad.set_key(Key::Four, is_pressed),
                W => keypad.set_key(Key::Five, is_pressed),
                E => keypad.set_key(Key::Six, is_pressed),
                R => keypad.set_key(Key::D, is_pressed),
                A => keypad.set_key(Key::Seven, is_pressed),
                S => keypad.set_key(Key::Eight, is_pressed),
                D => keypad.set_key(Key::Nine, is_pressed),
                F => keypad.set_key(Key::E, is_pressed),
                Z => keypad.set_key(Key::A, is_pressed),
                X => keypad.set_key(Key::Zero, is_pressed),
                C => keypad.set_key(Key::B, is_pressed),
                V => keypad.set_key(Key::F, is_pressed),
                _ => (),
            }
        }
    }

    fn beep(&mut self, is_beeping: bool) {
        if is_beeping {
            if !self.is_device_beeping {
                self.audio_device.resume();
                self.is_device_beeping = true;
            }
        } else if self.is_device_beeping {
            self.audio_device.pause();
            self.is_device_beeping = false;
        }
    }

    fn quit(&self) -> bool {
        if self._quit && self.is_device_beeping {
            self.audio_device.pause();
        }
        self._quit
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg = std::env::args().nth(1);
    let rom_path = arg.as_deref().unwrap_or(ROM_PATH);

    let sdl_context = sdl2::init()?;
    let main_loop_hander = SDLHandler::new(&sdl_context)?;

    let mut emu = Emu::new(HZ, main_loop_hander);
    let rom = std::fs::read(rom_path).expect("Could not find the rom file: '{rom_path}'");
    emu.load_rom(&rom);
    emu.run();

    Ok(())
}
