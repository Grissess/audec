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
    r: u8
}

#[repr(C)]
pub struct Spectrogram {
    bias: f32,
    range: f32,
    width: u32,
    height: u32,
    water_height: u32,
    water_y: u32,
    graph_height: u32,
    spectrum_length: u32,
    wd_offset: u32,
}

#[spirv(compute(threads(32)))]
pub fn spectrogram_line(
    // (pixel_x, channel_idx, 0)
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(push_constant)] sp: &Spectrogram,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] spectrum: &mut [Vec2],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] image_out: &mut [u8]
) {
    let chan = id.y;
    let normx = id.x as f32 / sp.width as f32;
    //let specidx = (normx * spec.len() as f32 / 2f32) as usize;
    let specidx = ((2f32.powf(normx) - 1f32) * sp.spectrum_length as f32 / 2f32) as usize;
    let specval = spectrum[sp.spectrum_length as usize * id.y as usize + specidx].length();
    let specval = if specval == 0.0 {
        -1000.0
    } else {
        specval.log10()
    };
    // println!("debug: wh {} bias {} gh {} range {} specval {}", water_height as i32, bias, graph_height as f32, range, specval);
    let mut specy = ((sp.bias + specval) * -(sp.graph_height as f32) / sp.range) as i32;
    if specy > sp.graph_height as i32 { specy = sp.graph_height as i32; }
    if specy < 0 { specy = 0; }
    {
        let a = 1f32 - (specy as f32 / sp.graph_height as f32);
        let win = &mut image_out[sp.wd_offset as usize + id.x as usize * 4 .. sp.wd_offset as usize + (id.x+1) as usize * 4];
        win[1 - chan as usize + 1] = (a * 255f32) as u8;
    }
    if x != 0 {
        self.view.draw_line(
            ((x - 1) as i32, last_y),
            (x as i32, water_height as i32 + specy)
        ).expect("drawing");
    }
    last_y = water_height as i32 + specy;
}