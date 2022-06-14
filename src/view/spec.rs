use super::{Info, View};

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::video::Window;
use sdl2::render::{Canvas, BlendMode};

pub struct Spec {
    pub view: Canvas<Window>,
    pub db_bias: f32,
    pub db_range: f32,
    pub waterfall_sz: f32,
}

impl View for Spec {
    fn render<'i, 's, 'j: 'i + 's>(&mut self, info: &'j Info<'i, 's>) {
        let (width, height) = self.view.output_size().expect("getting size");

        let bias = self.db_bias / 10f32;
        let range = self.db_range / 10f32;

        let water_height = (self.waterfall_sz * height as f32) as u32;
        let water_y = water_height - 1;
        let graph_height = height - water_height;

        self.view.set_draw_color(Color::RGB(0,0,0));
        self.view.fill_rect(Rect::new(0, water_y as i32, width, graph_height + 1)).expect("clearing");
        self.view.set_blend_mode(BlendMode::Add);

        // Move up the waterfall
        {
            let surf = self.view.window().surface(info.sdl.eloop).expect("getting surface ref");
            let mut other_surf = self.view.window().surface(info.sdl.eloop).expect("getting second surface ref");
            surf.blit(
                Rect::new(0, 1, width, water_y),
                &mut other_surf,
                Rect::new(0, 0, width, water_y),
            ).expect("blitting");
            other_surf.finish().expect("finish mut");
            surf.finish().expect("finish non-mut");
        }

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
                let normx = x as f32 / width as f32;
                //let specidx = (normx * spec.len() as f32 / 2f32) as usize;
                let specidx = ((2f32.powf(normx) - 1f32) * spec.len() as f32 / 2f32) as usize;
                let specval = spec[specidx].norm();
                let specval = if specval == 0.0 {
                    -1000.0
                } else {
                    specval.log10()
                };
                // println!("debug: wh {} bias {} gh {} range {} specval {}", water_height as i32, bias, graph_height as f32, range, specval);
                let mut specy = ((bias + specval) * -(graph_height as f32) / range) as i32;
                if specy > graph_height as i32 { specy = graph_height as i32; }
                if specy < 0 { specy = 0; }
                {
                    let dc = self.view.draw_color();
                    self.view.set_draw_color(Color::RGBA(
                            dc.r, dc.g, dc.b,
                            ((1f32 - (specy as f32 / graph_height as f32)) * 255f32) as u8,
                    ));
                    self.view.draw_point((x as i32, water_y as i32)).expect("drawing");
                    self.view.set_draw_color(dc);
                }
                if x > 0 {
                    self.view.draw_line(
                        ((x - 1) as i32, last_y),
                        (x as i32, water_height as i32 + specy)
                    ).expect("drawing");
                }
                last_y = water_height as i32 + specy;
            }
        }

        self.view.set_blend_mode(BlendMode::None);
        self.view.present();
    }
}
