pub mod scope;

pub struct ChannelInfo<'i> {
    pub samples: &'i [f32],
    pub spectrum: &'i [f32],
}

pub struct Info<'i> {
    pub left: ChannelInfo<'i>,
    pub right: ChannelInfo<'i>,
}

pub trait View {
    fn render<'i, 'j: 'i>(&mut self, info: &'j Info<'i>);
    fn requested_window(&self) -> usize { 0 }
}

fn normalize_centered(samp: f32, height: u32) -> i32 {
    let hh = height / 2;
    hh as i32 - (hh as f32 * samp) as i32
}
