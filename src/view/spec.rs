use super::{Info, View};

use sdl2::pixels::Color;
use sdl2::video::Window;
use sdl2::render::{Canvas, BlendMode};

pub struct Spec {
    pub view: Canvas<Window>,
}

impl View for Spec {
    fn render<'i, 'j: 'i>(&mut self, info: &'j Info<'i>) {
        self.view.set_draw_color(Color::RGB(0,0,0));
        self.view.clear();
        self.view.set_blend_mode(BlendMode::Add);
        let (width, height) = self.view.output_size().expect("getting size");

        for chan in 0 ..= 1 {
            let spec = if chan == 0 {
                self.view.set_draw_color(Color::RGB(0,255,0));
                &info.left.spectrum
            } else {
                self.view.set_draw_color(Color::RGB(0,0,255));
                &info.right.spectrum
            };

            let mut last_y = 0i32;
            for x in 0 .. width {
                // Since this is an RFFT, only half the spec is useful
                let specidx = ((x as f32 / width as f32) * spec.len() as f32 / 2f32) as usize;
                let specval = spec[specidx];
                let specy = (specval.norm().log10() * -(height as f32) / 5f32) as i32;
                if x > 0 {
                    self.view.draw_line(
                        ((x - 1) as i32, last_y),
                        (x as i32, specy)
                    );
                }
                last_y = specy;
            }
        }

        self.view.set_blend_mode(BlendMode::None);
        self.view.present();
    }
}
