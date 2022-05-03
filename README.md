# rhip-8

![ibm](https://user-images.githubusercontent.com/25510067/166488193-88ddb0a8-9d98-45c2-8a8f-e9bdaeb35798.png)

A **full** implementation of the Chip-8 emulator, in Rust. Fully decoupled from the graphics, audio and input handlers backend.  
SDL2 is used in this instance.

# Build and Play
- You wil need libsdl2, rustc and cargo
```console
$ pacman -S sdl2

$ git clone https://github.com/j-hc/rhip-8
$ cd rhip-8
$ cargo run --release -- ./roms/min.ch8
```
