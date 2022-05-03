pub mod emu;
mod keypad;
pub use keypad::{Key, KeyPad};

use core::marker::Copy;

pub trait MainLoopHandler {
    fn render(&mut self, gfx: &emu::GFX);
    fn handle_key(&mut self, keypad: &mut KeyPad);
    fn beep(&mut self, is_beeping: bool);
    fn quit(&self) -> bool;
}

struct Stack<T, const S: usize> {
    inner: [T; S],
    sp: usize,
}
impl<T: Default + Copy, const S: usize> Stack<T, S> {
    fn new() -> Self {
        Self {
            inner: [T::default(); S],
            sp: 0,
        }
    }

    fn push(&mut self, e: T) {
        if self.sp >= S {
            panic!("stack overflow");
        }
        self.sp += 1;
        self.inner[self.sp] = e;
    }

    fn pop(&mut self) -> T {
        if self.sp == 0 {
            panic!("stack underflow");
        }
        let e = self.inner[self.sp];
        self.sp -= 1;
        e
    }
}

pub(crate) struct Timer {
    pub timer: u8,
    per: u32,
    t: u32,
}
impl Timer {
    pub(crate) fn new(per: u32) -> Self {
        Self {
            timer: 0,
            per,
            t: 0,
        }
    }

    fn time(&mut self) {
        if self.timer > 0 {
            self.t += 1;
            if self.t >= self.per {
                self.timer -= 1;
                self.t = 0;
            }
        }
    }

    fn set_timer(&mut self, timer: u8) {
        self.timer = timer
    }
}
