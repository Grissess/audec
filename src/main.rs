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
use sdl2::keyboard::Keycode;
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
    if matches.is_present("list-api") {
        for (idx, api) in pa.host_apis() {
            println!("{}: {} ({:?})", idx, api.name, api);
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

    let init_sco_width: u32 = matches.value_of("sco-width").unwrap_or("800").parse().expect("getting scope initial width");
    let init_sco_height: u32 = matches.value_of("sco-height").unwrap_or("200").parse().expect("getting scope initial height");
    let init_spec_width: u32 = matches.value_of("spec-width").unwrap_or("800").parse().expect("getting spectrogram initial width");
    let init_spec_height: u32 = matches.value_of("spec-height").unwrap_or("600").parse().expect("getting spectrogram initial height");
    let init_vec_width: u32 = matches.value_of("vec-width").unwrap_or("400").parse().expect("getting vectorscope initial width");
    let init_vec_height: u32 = matches.value_of("vec-height").unwrap_or("400").parse().expect("getting vectorscope initial height");

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
            scope: Fifo::new(init_sco_width as usize),
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
            let scale: f32 = matches.value_of("aud-scale").unwrap_or("1.0").parse().expect("getting audio scale");
            let mut scratch: Vec<f32> = Vec::with_capacity(32768);
            move |InputCallbackArgs {buffer, frames, ..}| {
                let mut state = st.lock().unwrap();
                assert_eq!(buffer.len(), frames * 2);
                for offs in 0..=1 {
                    let ifo = if offs == 0 { &mut state.left } else { &mut state.right };
                    scratch.clear();
                    scratch.extend(buffer.chunks(2).map(|s| s[offs]));
                    if scale != 1.0 {
                        for samp in scratch.iter_mut() {
                            *samp *= scale;
                        }
                    }
                    ifo.scope.push(&scratch);
                    ifo.win.push(&scratch);
                }
                CallbackResult::Continue
            }
        },
    ).expect("opening stream");

    let sdl = sdl2::init().expect("initializing SDL");
    let sdl_video = sdl.video().expect("initializing SDL video");

    let mut views: Vec<Box<dyn View>> = Vec::new();

    if !matches.is_present("no-sco") {
        let scope_win = sdl_video.window("scope", init_sco_width, init_sco_height)
            .position_centered()
            .resizable()
            .build().expect("creating scope");
        let scope_can = scope_win.into_canvas().build().expect("creating scope canvas");
        let scope = view::scope::Scope {
            view: scope_can,
            zc_search: matches.value_of("sco-search").unwrap_or("1024").parse().expect("getting scope search"),
            zc_horiz: matches.value_of("sco-pos").unwrap_or("0.5").parse().expect("getting scope zc pos"),
            pow: matches.value_of("sco-pow").unwrap_or("1.0").parse().expect("getting scope pow"),
        };
        views.push(Box::new(scope));
    }

    if !matches.is_present("no-spec") {
        let spec_win = sdl_video.window("spec", init_spec_width, init_spec_height)
            .position_centered()
            .resizable()
            .build().expect("creating spec");
        let spec_can = spec_win.into_canvas().build().expect("creating spec canvas");
        let spec = view::spec::Spec {
            view: spec_can,
            db_bias: matches.value_of("spec-bias").unwrap_or("-5.0").parse().expect("getting spectrogram bias"),
            db_range: matches.value_of("spec-range").unwrap_or("30.0").parse().expect("getting spectrogram range"),
            waterfall_sz: matches.value_of("spec-water-size").unwrap_or("0.8").parse().expect("getting spectrogam waterfall size"),
            waterfall_data: None,
            waterfall_tex: std::ptr::null_mut(),
        };
        views.push(Box::new(spec));
    }

    if !matches.is_present("no-vec") {
        let vec_win = sdl_video.window("vec", init_vec_width, init_vec_height)
            .position_centered()
            .resizable()
            .build().expect("creating vec");
        let vec_can = vec_win.into_canvas().build().expect("creating vec canvas");
        let vec = view::vec::Vector {
            view: vec_can,
            fade_rate: matches.value_of("vec-fade").unwrap_or("32").parse().expect("getting vec fade"),
            brightness: matches.value_of("vec-brightness").unwrap_or("32").parse().expect("getting vec brightness"),
        };
        views.push(Box::new(vec));
    }

    let mut eloop = sdl.event_pump().expect("creating event loop");
    let mut deadline;
    let mut lspec: Vec<Complex<f32>> = vec![Complex { re: 0f32, im: 0f32 }; fft_size];
    let mut rspec: Vec<Complex<f32>> = vec![Complex { re: 0f32, im: 0f32 }; fft_size];
    let rate = Duration::new(1, 0).div_f64(matches.value_of("gfx-rate").unwrap_or("60").parse::<f64>().expect("parsing frame rate"));
    stream.start().expect("starting stream");
    'main: loop {
        deadline = Instant::now() + rate;
        hprof::start_frame();

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

        let mut winsz = MIN_SAMPS;
        for view in &mut views {
            view.render(&info);
            winsz = std::cmp::max(winsz, view.requested_window());
        }

        {
            let mut st = state.lock().unwrap();
            if winsz != st.left.scope.size() {
                st.left.scope.resize(winsz);
                st.right.scope.resize(winsz);
            }
        }

        hprof::end_frame();

        for event in eloop.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'main,
                _ => (),
            }
        }

        let wait = deadline.saturating_duration_since(Instant::now());
        if !wait.is_zero() {
            thread::sleep(wait);
        }
        // println!("tick");
    }
    hprof::profiler().print_timing();
}
