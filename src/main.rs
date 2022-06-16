#[macro_use]
extern crate clap;
extern crate portaudio;
extern crate rustfft;
extern crate sdl2;

mod window;
mod fifo;
mod view;

use std::{iter, thread};
use std::time::{Instant, Duration};
use std::sync::{Arc, Mutex};
use portaudio::stream::{Parameters, InputSettings, CallbackResult, InputCallbackArgs};
use sdl2::event::Event;
use rustfft::num_complex::Complex;
use fifo::Fifo;
use view::View;

const MIN_SAMPS: usize = 256;

#[derive(Debug, Clone)]
struct ChannelInfo {
    scope: Fifo<f32>,
    win: Fifo<f32>,
}

#[derive(Debug, Clone)]
struct State {
    left: ChannelInfo,
    right: ChannelInfo,
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
            .find(|(_, info)| info.name == devname)
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
    let fft_size: usize = matches.value_of("fft-size").unwrap_or("1024").parse().expect("getting FFT size");
    let mut fft_plan = rustfft::FftPlanner::new();
    let fft = fft_plan.plan_fft_forward(fft_size);
    let mut fft_scratch: Vec<Complex<f32>> = iter::repeat(Complex { re: 0.0, im: 0.0 })
        .take(fft.get_inplace_scratch_len())
        .collect();
    let state = Arc::new(Mutex::new({
        let ci = ChannelInfo {
            scope: Fifo::new(init_width as usize),
            win: Fifo::new(fft_size),
        };
        State {
            left: ci.clone(),
            right: ci,
        }
    }));
    let win = windows.get(matches.value_of("fft-win").unwrap_or("hann")).expect("getting window function")(fft_size);
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
                    ifo.win.push(&scratch);
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
    let scope_can = scope_win.into_canvas().build().expect("creating scope canvas");
    let mut scope = view::scope::Scope {
        view: scope_can,
        zc_search: 1024,
        zc_horiz: 0.5f32,
    };

    let spec_win = sdl_video.window("spec", init_width, init_height)
        .position_centered()
        .resizable()
        .build().expect("creating spec");
    let spec_can = spec_win.into_canvas().build().expect("creating spec canvas");
    let mut spec = view::spec::Spec {
        view: spec_can,
        db_bias: -5f32,
        db_range: 30f32,
        waterfall_sz: 0.8f32,
        waterfall_data: None,
        waterfall_tex: std::ptr::null_mut(),
    };

    let mut eloop = sdl.event_pump().expect("creating event loop");
    let mut deadline;
    let mut lspec: Vec<Complex<f32>> = vec![Complex { re: 0f32, im: 0f32 }; fft_size];
    let mut rspec: Vec<Complex<f32>> = vec![Complex { re: 0f32, im: 0f32 }; fft_size];
    let rate = Duration::new(1, 0).div_f64(matches.value_of("gfx-rate").unwrap_or("60").parse::<f64>().expect("parsing frame rate"));
    stream.start().expect("starting stream");
    'main: loop {
        deadline = Instant::now() + rate;

        {
            for i in 0 ..= 1 {
                let slc = {
                    let st = state.lock().unwrap();
                    if i == 0 {
                        lspec.clear();
                        lspec.extend(st.left.win.iter().map(|&x| Complex { re: x, im: 0.0 }));
                        &mut lspec
                    } else {
                        rspec.clear();
                        rspec.extend(st.right.win.iter().map(|&x| Complex { re: x, im: 0.0 }));
                        &mut rspec
                    }
                };

                for (pt, wv) in slc.iter_mut().zip(win.shape()) {
                    *pt *= wv;
                }

                fft.process_with_scratch(slc, &mut fft_scratch);
                let fac = 1f32 / (slc.len() as f32).sqrt();
                for pt in slc {
                    *pt *= fac;
                }
            }
        }

        let stcopy = {
            let st = state.lock().unwrap();
            st.clone()
        };

        let info = view::Info {
            left: view::ChannelInfo {
                samples: &stcopy.left.scope[..],
                spectrum: &lspec[..],
            },
            right: view::ChannelInfo {
                samples: &stcopy.right.scope[..],
                spectrum: &rspec[..],
            },
            sdl: view::SDLInfo {
                ctx: &sdl,
                eloop: &eloop,
            },
        };

        scope.render(&info);
        spec.render(&info);

        {
            let mut st = state.lock().unwrap();
            let mut winsz = MIN_SAMPS;
            winsz = std::cmp::max(winsz, scope.requested_window());
            if winsz != st.left.scope.size() {
                st.left.scope.resize(winsz);
                st.right.scope.resize(winsz);
            }
        }

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
