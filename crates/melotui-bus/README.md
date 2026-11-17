# melotui-bus

Part of [melotui](https://github.com/m-d-nabeel/melotui), a terminal music player. This crate is what melotui's threads say to one another: the commands and events between the interface and the audio, database, download and cover workers, the channels that carry them, and the bridge the spectrum crosses.

It is built for melotui and changes with it, release by release: it is not meant to be used on its own, and its API makes no promises between versions.

To install melotui itself:

```bash
cargo install --locked melotui   # or: cargo binstall melotui
```

Licensed under either of [MIT](https://github.com/m-d-nabeel/melotui/blob/master/LICENSE-MIT) or [Apache-2.0](https://github.com/m-d-nabeel/melotui/blob/master/LICENSE-APACHE), at your option.
