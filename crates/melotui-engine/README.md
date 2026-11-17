# melotui-engine

Part of [melotui](https://github.com/m-d-nabeel/melotui), a terminal music player. This crate starts melotui's workers (audio, database, downloads, album art) and carries messages between them and the interface.

It is built for melotui and changes with it, release by release: it is not meant to be used on its own, and its API makes no promises between versions.

To install melotui itself:

```bash
cargo install --locked melotui   # or: cargo binstall melotui
```

Licensed under either of [MIT](https://github.com/m-d-nabeel/melotui/blob/master/LICENSE-MIT) or [Apache-2.0](https://github.com/m-d-nabeel/melotui/blob/master/LICENSE-APACHE), at your option.
