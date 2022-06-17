use super::{Info, View};

use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::video::Window;
use sdl2::render::{Canvas, BlendMode, Texture, TextureAccess};

pub struct Spec {
    pub view: Canvas<Window>,
    pub db_bias: f32,
    pub db_range: f32,
    pub waterfall_sz: f32,
    pub waterfall_data: Option<Vec<u8>>,
    pub waterfall_tex: *mut sdl2_sys::SDL_Texture,
}

impl Spec {
    fn rebuild_texture(&mut self, w: usize, h: usize) {
        let _g = hprof::enter("rebuild_texture");
        self.waterfall_data = Some(vec![0u8; w * h * 4]);
        let tc = self.view.texture_creator();
        let wf = tc.create_texture(
            PixelFormatEnum::RGBA8888,
            TextureAccess::Streaming,
            w as u32, h as u32
        ).expect("creating spec backing texture");
        if !self.waterfall_tex.is_null() {
            unsafe { sdl2_sys::SDL_DestroyTexture(self.waterfall_tex) };
        }
        self.waterfall_tex = wf.raw();
        std::mem::forget(wf);
    }
}

impl View for Spec {
    fn render<'i, 's, 'j: 'i + 's>(&mut self, info: &'j Info<'i, 's>) {
        let _g = hprof::enter("Spec::render");
        let (width, height) = self.view.output_size().expect("getting size");

        let bias = self.db_bias / 10f32;
        let range = self.db_range / 10f32;

        let water_height = (self.waterfall_sz * height as f32) as u32;
        let water_y = water_height - 1;
        let graph_height = height - water_height;

        if let Some(d) = &self.waterfall_data {
            if d.len() != width as usize * water_height as usize * 4 {
                self.rebuild_texture(width as usize, water_height as usize);
            }
        } else {
            self.rebuild_texture(width as usize, water_height as usize);
        }

        self.view.set_draw_color(Color::RGB(0,0,0));
        self.view.fill_rect(Rect::new(0, water_y as i32, width, graph_height + 1)).expect("clearing");
        self.view.set_blend_mode(BlendMode::Add);

        // Move up the waterfall
        let g2 = hprof::enter("waterfall");
        self.waterfall_data
            .as_mut()
            .unwrap()
            .copy_within(width as usize * 4.., 0);
        let lw = self.waterfall_data.as_ref().unwrap().len();
        (&mut self.waterfall_data.as_mut().unwrap()[lw - width as usize * 4 ..]).fill(0u8);

        let mut spectrums = Vec::with_capacity(info.left.len() * 2);
        spectrums.extend_from_slice(&info.left);
        spectrums.extend_from_slice(&info.right);
        let spectrums: Vec<shaders::Vec2> = spectrums.into_iter().map(|c| shaders::Vec2::new(c.re, c.im)).collect();

        let (width, height) = self.view.output_size().expect("getting size");

        let bias = self.db_bias / 10f32;
        let range = self.db_range / 10f32;
        
        let water_height = (self.waterfall_sz * height as f32) as u32;
        let water_y = water_height - 1;
        let graph_height = height - water_height;
         
        
        for chan in 0 ..= 1 {
            let mut last_y = 0i32;
            let wd_offset = water_y as usize * width as usize * 4;
            for x in 0..width {
                let nonsdl = hprof::enter("inner loop");
    
                shaders::spectrogram_line( UVec3::new(x, chan, 0), &shaders::Spectrogram {
                    bias,
                    range,
                    width,
                    height,
                    water_height,
                    water_y,
                    graph_height,
                    wd_offset,
                    spectrum_length: info.left.spectrum.len() as u32,
                }, &mut spectrums, &mut img);
            }

        }

        drop(g2);

        let mut wf: Texture<'static> = unsafe { std::mem::transmute(self.waterfall_tex) };
        wf.update(
            None,
            &self.waterfall_data.as_ref().unwrap(),
            width as usize * 4,
        )
        .expect("uploading");
        self.view
            .copy(&wf, None, Some(Rect::new(0, 0, width, water_height)))
            .expect("blitting");
        std::mem::forget(wf);

        drop(_g);

        self.view.set_blend_mode(BlendMode::None);
        self.view.present();
    }
}
