use super::{Info, View};

use sdl2::pixels::Color;
use sdl2::video::Window;
use sdl2::render::{Canvas, BlendMode};

fn normalize_centered(samp: f32, height: u32) -> i32 {
    let hh = height / 2;
    hh as i32 - (hh as f32 * samp) as i32
}

pub struct Scope {
    pub view: Canvas<Window>,
    pub zc_search: usize,
    pub zc_horiz: f32,
}

impl View for Scope {
    fn render<'i, 's, 'j: 'i + 's>(&mut self, info: &'j Info<'i, 's>) {
        self.view.set_draw_color(Color::RGB(0,0,0));
        self.view.clear();
        self.view.set_blend_mode(BlendMode::Add);
        let (width, height) = self.view.output_size().expect("getting size");
        let winsz = info.left.samples.len();
        let mut zc_mark = (width as f32 * self.zc_horiz) as usize;
        if zc_mark >= winsz { zc_mark = winsz - 1; }
        //let mut indices_set: Vec<usize> = Vec::new();
        let mut count = 0usize;
        let mut last = -1f32;
        let offset = (0 .. self.zc_search)
            .filter(|&i| zc_mark + i < winsz)
            .map(|i| (i, zc_mark + i, info.left.samples[zc_mark + i], info.right.samples[zc_mark + i]))
            .scan(None,
                  |state, (i, _ai, l, r)| {
                      count += 1;
                      let en = l.abs() + r.abs();
                      let sm = l + r;
                      let mut ix = 0;
                      if state.is_none() || {
                          let (_li, min, nix) = state.unwrap();
                          ix = nix + 1;
                          en < min && last <= sm
                      } {
                          //indices_set.push(ai);
                          *state = Some((i, en, ix));
                      }
                      last = sm;
                      *state
                  }
            )
            .last()
            .unwrap_or((0, 0.0, self.zc_search + 1));
        //println!("sc {:?} count {}", offset, count);
        let offset = offset.0;
        self.view.set_draw_color(Color::RGB(63,0,0));
        let zcx = width as i32 - zc_mark as i32;
        self.view.draw_line(
            (zcx, 0i32),
            (zcx, height as i32)
        ).expect("drawing");

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
            for (x, samp) in samps.iter().skip(offset).cloned().enumerate() {
                /*
                let ix = x + offset;
                if indices_set.contains(&ix) {
                    self.view.set_draw_color(Color::RGB(255,255,255));
                    self.view.draw_line(
                        (x as i32, 0i32),
                        (x as i32, height as i32),
                    ).expect("drawing");
                }
                */
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
        self.view.output_size().expect("getting output size").0 as usize + self.zc_search
    }
}
