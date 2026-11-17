# melotui-core

Part of [melotui](https://github.com/m-d-nabeel/melotui), a terminal music player. This crate is the player's state and what changes it: the reducer that every message passes through, the `:` commands and their completion, the settings, the music library model, the choice of colour theme, and the session saved between runs.

It is built for melotui and changes with it, release by release: it is not meant to be used on its own, and its API makes no promises between versions.

To install melotui itself:

```bash
cargo install --locked melotui   # or: cargo binstall melotui
```

Licensed under either of [MIT](https://github.com/m-d-nabeel/melotui/blob/master/LICENSE-MIT) or [Apache-2.0](https://github.com/m-d-nabeel/melotui/blob/master/LICENSE-APACHE), at your option.
