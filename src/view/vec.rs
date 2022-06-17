use super::{Info, View};
use super::scope::normalize_centered;

use sdl2::pixels::Color;
use sdl2::video::Window;
use sdl2::render::{Canvas, BlendMode};

pub struct Vector {
    pub view: Canvas<Window>,
    pub fade_rate: u8,
    pub brightness: u8
}

impl View for Vector {
    fn render<'i, 's, 'j: 'i + 's>(&mut self, info: &'j Info<'i, 's>) {
        let _g = hprof::enter("Vector::render");
        self.view
            .set_draw_color(Color::RGBA(0, 0, 0, self.fade_rate));
        self.view.set_blend_mode(BlendMode::None);
        self.view.fill_rect(None).expect("clearing");

        let (width, height) = self.view.output_size().expect("getting size");
        self.view.set_draw_color(Color::RGB(0,self.brightness,self.brightness));
        self.view.set_blend_mode(BlendMode::Add);

        let mut lastpt = None;
        for (&x, &y) in info.left.samples.iter().zip(info.right.samples) {
            let (x, y) = (
                width as i32 - normalize_centered(x, width),
                normalize_centered(y, height)
            );
            if let Some((lx, ly)) = lastpt {
                self.view.draw_line((lx, ly), (x, y)).expect("drawing");
            }
            lastpt = Some((x, y));
        }

        drop(_g);

        self.view.present();
    }
}
