# melotui-audio

Part of [melotui](https://github.com/m-d-nabeel/melotui), a terminal music player. This crate plays the music: decoding (through Symphonia, and Opus through opus-rs), mixing surround down to stereo, gapless playback, the effects of the DSP studio (equalizer, reverb, echo, spatializer, limiter and more), and the spectrum the visualizer draws.

It is built for melotui and changes with it, release by release: it is not meant to be used on its own, and its API makes no promises between versions.

To install melotui itself:

```bash
cargo install --locked melotui   # or: cargo binstall melotui
```

Licensed under either of [MIT](https://github.com/m-d-nabeel/melotui/blob/master/LICENSE-MIT) or [Apache-2.0](https://github.com/m-d-nabeel/melotui/blob/master/LICENSE-APACHE), at your option.
