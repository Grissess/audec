# audec

_A somewhat-modern tool for precisely rendering real-time audio analysis._

**audec** is a real-time audio visualizer intended for utility and information
density more than entertainment (it is the (au)dio (dec)ompiler). It is
intended as a FOSS alternative to another proprietary real-time
waterfall-rendering analyzer often touted by Linux musicians, and attempts to
loosely emulate its appearance.

## Screenshots

Some music--note the stereo separation, broadband noise, pitch-space mapping...

![scrot1](../doc/scrot1.png)

An example configuration of the lovely [ZynAddSubFX][zyn] soft-synth:

![scrot2](../doc/scrot2.png)

A clipping sine wave with multiple harmonics (note that the views can be
resized fluidly):

![scrot3](../doc/scrot3.png)

[zyn]: https://zynaddsubfx.sourceforge.io/

## Requirements

- [Cargo][cargo] (normally part of your Rust toolchain);
- A host version of [PortAudio][portaudio] (or your platform's build tools,
  e.g., `build-essential`, so that Rust `portaudio` can build it on its own);
- For Linux: ensure you have [ALSA][alsa]'s "development files" as well (e.g.
  `libasound-dev`), as that seems to be the default on this platform;
- [SDL2][sdl2]'s development files.

On Linux and perhaps other POSIX platforms, these should be available in your
favorite package manager (unless you're doing things "from scratch"). Cygwin is
recommended for Windows users, or you could try your hand at building and
installing from source.

[cargo]: https://doc.rust-lang.org/cargo/
[portaudio]: http://portaudio.com/
[alsa]: https://alsa-project.org/wiki/Main_Page
[sdl2]: https://www.libsdl.org/download-2.0.php

## Usage

`cargo build` builds the binary (somewhere into the `target` directory), `cargo run`
runs it. Pass `--release` to either to disable runtime checks for performance.

Without command line arguments, sane defaults are assumed:
- Your default input device (usually a built-in microphone, or whatever would be preferred for audio recording);
- The selected device's default sample rate;
- An 800x200 scope and an 800x600 spectrogram/waterfall;
- A Hann-window FFT of 1024 samples;
- Zero-crossing search of 1024 samples;
- A -5dBFS spectral bias and a range of 30dBFS.

All of these can be overridden from the command line; use `--help` to see the
options. (If you're using `cargo run`, make sure you put a `--` before
`--help`.)

### A Note on Monitors

Since you'll probably want to analyze the signal coming _from_ your computer,
PortAudio on Linux most often uses ALSA, which does not have an in-built
concept of "monitor devices" (inputs derived from signal sent to an output
device); you'd normally have to use `snd-aloop` for that. However, modern audio
routing daemons often provide their own:

- PulseAudio users can use `pavucontrol` to change the "Recording stream" to
  one of its monitor devices, presuming you have the monitor module loaded
  (this is the default)--you can probably also use `pactl` from the command
  line if you need to;
- JACK users can route things however they want, using `jack_connect`,
  `jack_disconnect`, `jack_lsp`, and any of the graphical patchbays like
  `qjackctl` or `patchage`;
- PipeWire users can do the same with `pw-link` or graphical patchbays like
  `helvum`, as well as tools compatible with either of the above through its
  own emulation (e.g. with `pw-jack`)--you can also set the `PIPEWIRE_NODE`
  envvar to your audio output node ID (see `pw-cli dump short Node`) so that
  the ALSA Pipewire plugin connects to the monitor automatically.

Your mileage may vary on other platforms.

## Contributing

**Help wanted!** Report missing features and bugs on the [GitHub issue tracker][ghissue].
Some known issues, for example:
- No scale yet (on either axis) in the spectrum/waterfall view;
- No dynamic adjustment of parameters;
- Could use more/better window functions;
- Pure software-rendering of waterfall limits performance;
- Changing window size dumps historical data;
- No prebuilt binaries;
- No changing the default color scheme;
... and probably others I'm unaware of.

If you're feeling motivated, [pull requests][ghpr] are also welcome :)

[ghissue]: https://github.com/Grissess/audec/issues
[ghpr]: https://github.com/Grissess/audec/pulls

## License

This software is hereby released under the "Rust license", which is also known
as the "dual Apache/MIT license"; specifically, you may choose, at your option:

- [Apache License, Version 2.0][apache2] (mirrored [here][mir-apache2]); or
- [MIT license][mit] (mirrored [here][mir-mit] with placeholders).

Contributions Submitted for inclusion into the Work by You (as per _supra_
Apache license) are assumed to comply with the terms of the selfsame Apache
license without any additional terms or conditions (see _id._, section 5) and
without patent encumbrance (_id._, section 3).

[apache2]: ./LICENSE.APACHE2
[mir-apache2]: https://www.apache.org/licenses/LICENSE-2.0
[mit]: ./LICENSE.MIT
[mir-mit]: https://opensource.org/licenses/MIT
