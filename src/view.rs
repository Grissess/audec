pub mod scope;
pub mod spec;

use rustfft::num_complex::Complex;

pub struct ChannelInfo<'i> {
    pub samples: &'i [f32],
    pub spectrum: &'i [Complex<f32>],
}

pub struct SDLInfo<'s> {
    pub ctx: &'s sdl2::Sdl,
    pub eloop: &'s sdl2::EventPump,
}

pub struct Info<'i, 's> {
    pub left: ChannelInfo<'i>,
    pub right: ChannelInfo<'i>,
    pub sdl: SDLInfo<'s>
}

pub trait View {
    fn render<'i, 's, 'j: 'i + 's>(&mut self, info: &'j Info<'i, 's>);
    fn requested_window(&self) -> usize { 0 }
}

fn normalize_centered(samp: f32, height: u32) -> i32 {
    let hh = height / 2;
    hh as i32 - (hh as f32 * samp) as i32
}
