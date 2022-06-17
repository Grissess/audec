#![cfg_attr(
    target_arch = "spirv",
    feature(register_attr),
    register_attr(spirv),
    no_std
)]

pub use spirv_std::glam::{UVec3, Vec2, Vec4};
#[cfg(target_arch = "spirv")]
use spirv_std::num_traits::Float;

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

#[repr(C)]
pub struct Pixel {
    a: u8,
    b: u8,
    g: u8,
    r: u8,
}

#[repr(C)]
pub struct Spectrogram {
    pub bias: f32,
    pub range: f32,
    pub width: u32,
    pub height: u32,
    pub water_height: u32,
    pub water_y: u32,
    pub graph_height: u32,
    pub spectrum_length: u32,
    pub wd_offset: u32,
}

fn spectral_lookup(x: u32, sp: &Spectrogram, channel: u32, spectrum: & [Vec2]) -> (f32, i32) {
    let normx = x as f32 / sp.width as f32;
    //let specidx = (normx * spec.len() as f32 / 2f32) as usize;
    let specidx = ((2f32.powf(normx) - 1f32) * sp.spectrum_length as f32 / 2f32) as usize;
    let specval = spectrum[sp.spectrum_length as usize * channel as usize + specidx].length();
    let specval = if specval == 0.0 {
        -1000.0
    } else {
        specval.log10()
    };
    let mut specy = ((sp.bias + specval) * -(sp.graph_height as f32) / sp.range) as i32;
    if specy > sp.graph_height as i32 {
        specy = sp.graph_height as i32;
    }
    if specy < 0 {
        specy = 0;
    }
    (specval, specy)
}

const CHAN_COLORS: &'static [[u8; 3]] = &[[0, 255, 0], [0, 0, 255]];
#[inline]
#[spirv(compute(threads(32)))]
pub fn spectrogram_line(
    // (pixel_x, channel_idx, 0)
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(push_constant)] sp: &Spectrogram,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] spectrum: &[Vec2],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] image_out: &mut [u8],
) {
    let chan = id.y;
    let chan_color = CHAN_COLORS[chan as usize];

    let (_, specy) = spectral_lookup(id.x, sp, chan, spectrum);
    // println!("debug: wh {} bias {} gh {} range {} specval {}", water_height as i32, bias, graph_height as f32, range, specval);
    {
        let a = 1f32 - (specy as f32 / sp.graph_height as f32);
        let win = &mut image_out[sp.wd_offset as usize + id.x as usize * 4
            ..sp.wd_offset as usize + (id.x + 1) as usize * 4];
        win[1 - chan as usize + 1] = (a * 255f32) as u8;
    }
    if id.x != 0 {
        let (_, prev_specy) = spectral_lookup(id.x - 1, sp, chan, spectrum);
        let cutoff = (specy - prev_specy) / 2;
        let count = cutoff.abs();
        let s = cutoff.signum();
        for y in 0..count*2 {
            let xoff = if y > count { id.x as usize * 4 } else { (id.x as usize - 1) * 4 };
            let pxy = (sp.water_height as usize + prev_specy as usize).wrapping_add((s * y) as usize) * sp.width as usize * 4 + xoff;
            if pxy + 4 < image_out.len() {  // Seems to happen when the window resizes for at least a frame
                let win = &mut image_out[pxy..pxy + 4];
                win[1] = chan_color[2];
                win[2] = chan_color[1];
                win[3] = chan_color[0];
            }
        }
    }
}
