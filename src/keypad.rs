pub enum Key {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    A,
    B,
    C,
    D,
    E,
    F,
}

impl Key {
    fn addr(&self) -> usize {
        use Key::*;
        match self {
            Zero => 0,
            One => 1,
            Two => 2,
            Three => 3,
            Four => 4,
            Five => 5,
            Six => 6,
            Seven => 7,
            Eight => 8,
            Nine => 9,
            A => 0xA,
            B => 0xB,
            C => 0xC,
            D => 0xD,
            E => 0xE,
            F => 0xF,
        }
    }
}

#[derive(Default)]
pub struct KeyPad {
    inner: [u8; 16],
}
impl KeyPad {
    pub fn press_key(&mut self, key: Key) {
        self.handle_key(key, 1)
    }

    pub fn release_key(&mut self, key: Key) {
        self.handle_key(key, 0)
    }

    pub fn set_key(&mut self, key: Key, is_pressed: bool) {
        self.handle_key(key, is_pressed.into())
    }

    fn handle_key(&mut self, key: Key, flag: u8) {
        let addr = key.addr();
        self.inner[addr] = flag;
    }

    pub(crate) fn is_pressed(&self, key: u8) -> bool {
        self.inner[key as usize] == 1
    }

    pub(crate) fn get_pressed(&self) -> Option<u8> {
        self.inner.iter().position(|k| k == &1).map(|p| p as u8)
    }
}
