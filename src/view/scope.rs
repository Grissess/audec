use super::{Info, View, normalize_centered};

use sdl2::pixels::Color;
use sdl2::video::Window;
use sdl2::render::{Canvas, BlendMode};

pub struct Scope {
    pub view: Canvas<Window>,
}

impl View for Scope {
    fn render<'i, 'j: 'i>(&mut self, info: &'j Info<'i>) {
        self.view.set_draw_color(Color::RGB(0,0,0));
        self.view.clear();
        self.view.set_blend_mode(BlendMode::Add);
        let (width, height) = self.view.output_size().expect("getting size");

        for offs in 0..=1 {
            let def_color = if offs == 0 {
                Color::RGB(0,255,0)
            } else {
                Color::RGB(0,0,255)
            };
            let clip_color = if offs == 0 {
                Color::RGB(255,0,0)
            } else {
                Color::RGB(255,0,255)
            };
            
            let samps = if offs == 0 {
                &info.left.samples
            } else {
                &info.right.samples
            };

            let mut last_samp = 0.0f32;
            for (x, samp) in samps.iter().cloned().enumerate() {
                if x > 0 {
                    self.view.set_draw_color(
                        if samp > 1.0 || samp < -1.0 {
                            clip_color
                        } else {
                            def_color
                        }
                    );
                    self.view.draw_line(
                        ((x - 1) as i32, normalize_centered(last_samp, height)),
                        (x as i32, normalize_centered(samp, height))
                    ).expect("drawing");
                }
                last_samp = samp;
            }
        }


        self.view.set_blend_mode(BlendMode::None);
        self.view.present();
    }

    fn requested_window(&self) -> usize {
        self.view.output_size().expect("getting output size").0 as usize
    }
}
