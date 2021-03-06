pub mod scope;
pub mod spec;
pub mod vec;

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
