#[macro_use]
extern crate clap;
extern crate portaudio;
extern crate rustfft;
extern crate sdl2;

mod window;
mod fifo;

use std::{iter, thread};
use std::time::{Instant, Duration};
use std::sync::{Arc, Mutex};
use portaudio::stream::{Parameters, InputSettings, CallbackResult, InputCallbackArgs};
use sdl2::pixels::Color;
use sdl2::render::BlendMode;
use sdl2::event::Event;
use fifo::Fifo;

const FPB: u32 = 256;
const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

#[derive(Debug, Clone)]
struct ChannelInfo {
    scope: Fifo<f32>,
    win: Fifo<f32>,
}

#[derive(Debug)]
struct State {
    left: ChannelInfo,
    right: ChannelInfo,
}

fn normalize_centered(samp: f32, height: u32) -> i32 {
    let hh = height / 2;
    hh as i32 - (hh as f32 * samp) as i32
}

fn main() {
    let parser_yaml = load_yaml!("args.yml");
    let parser = clap::App::from_yaml(parser_yaml);
    let matches = parser.get_matches();

    let windows = window::windows();
    let pa = portaudio::PortAudio::new().expect("initializing PortAudio");

    // Take care of listing options first
    if matches.is_present("list-win") {
        for name in windows.keys() {
            println!("{}", name);
        }
        return;
    }
    if matches.is_present("list-dev") {
        for dev in pa.devices().expect("listing devices") {
            if let Ok((idx, info)) = dev {
                // Only input devices
                if info.max_input_channels == 0 { continue; }
                println!("{:?}: {} (default {} Hz, up to {} channels)", idx, info.name, info.default_sample_rate, info.max_input_channels);
            } else {
                eprintln!("(error enumerating device)");
            }
        }
        return;
    }

    let init_width: u32 = matches.value_of("sco-width").unwrap_or("800").parse().expect("getting scope initial width");
    let init_height: u32 = matches.value_of("sco-height").unwrap_or("200").parse().expect("getting scope initial height");

    let didx = if let Some(devname) = matches.value_of("aud-dev") {
        let (didx, _) = pa.devices().expect("listing devices").filter_map(Result::ok)
            .find(|(didx, info)| info.name == devname)
            .expect("finding named device");
        didx
    } else {
        pa.default_input_device().expect("getting default input device")
    };

    let info = pa.device_info(didx).expect("getting device info");

    let params = Parameters::<f32>::new(
        didx, 2, true, 0.0
    );
    let fpb: u32 = matches.value_of("aud-period").unwrap_or("256").parse().expect("getting audio period");
    let settings = InputSettings::new(
        params, if let Some(rate) = matches.value_of("aud-rate") {
            rate.parse().expect("getting audio sample rate")
        } else {
            info.default_sample_rate
        }, fpb
    );
    println!("Settings: {:?}", settings);
    let state = Arc::new(Mutex::new({
        let ci = ChannelInfo {
            scope: Fifo::new(WIDTH as usize),
            win: Fifo::new(1024),
        };
        State {
            left: ci.clone(),
            right: ci,
        }
    }));
    let mut stream = pa.open_non_blocking_stream(
        settings,
        {
            let st = state.clone();
            let mut scratch: Vec<f32> = Vec::with_capacity(32768);
            move |InputCallbackArgs {buffer, frames, ..}| {
                let mut state = st.lock().unwrap();
                assert_eq!(buffer.len(), frames * 2);
                for offs in 0..=1 {
                    let ifo = if offs == 0 { &mut state.left } else { &mut state.right };
                    scratch.clear();
                    scratch.extend(buffer.chunks(2).map(|s| s[offs]));
                    ifo.scope.push(&scratch);
                }
                CallbackResult::Continue
            }
        },
    ).expect("opening stream");

    let sdl = sdl2::init().expect("initializing SDL");
    let sdl_video = sdl.video().expect("initializing SDL video");

    let scope_win = sdl_video.window("scope", init_width, init_height)
        .position_centered()
        .resizable()
        .build().expect("creating scope");
    let mut scope_can = scope_win.into_canvas().build().expect("creating scope canvas");
    scope_can.set_draw_color(Color::RGB(0,0,0));
    scope_can.clear();
    scope_can.present();

    let mut eloop = sdl.event_pump().expect("creating event loop");
    let mut deadline;
    let rate = Duration::new(1, 0).div_f64(matches.value_of("gfx-rate").unwrap_or("60").parse::<f64>().expect("parsing frame rate"));
    stream.start().expect("starting stream");
    'main: loop {
        deadline = Instant::now() + rate;

        scope_can.set_draw_color(Color::RGB(0,0,0));
        scope_can.clear();
        scope_can.set_blend_mode(BlendMode::Add);
        let (width, height) = scope_can.output_size().expect("getting size");

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
            
            let mut st = state.lock().unwrap();
            let samps = if offs == 0 {
                &mut st.left.scope
            } else {
                &mut st.right.scope
            };

            let mut last_samp = 0.0f32;
            for (x, samp) in samps.iter().cloned().enumerate() {
                if x > 0 {
                    scope_can.set_draw_color(
                        if samp > 1.0 || samp < -1.0 {
                            clip_color
                        } else {
                            def_color
                        }
                    );
                    scope_can.draw_line(
                        ((x - 1) as i32, normalize_centered(last_samp, height)),
                        (x as i32, normalize_centered(samp, height))
                    ).expect("drawing");
                }
                last_samp = samp;
            }
            if width as usize != samps.size() {
                samps.resize(width as usize);
            }
        }


        scope_can.set_blend_mode(BlendMode::None);
        scope_can.present();

        for event in eloop.poll_iter() {
            match event {
                Event::Quit {..} => break 'main,
                _ => (),
            }
        }

        let wait = deadline.saturating_duration_since(Instant::now());
        if !wait.is_zero() {
            thread::sleep(wait);
        }
        // println!("tick");
    }
}
