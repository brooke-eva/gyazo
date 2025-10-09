# gyazo

[![ci](https://github.com/brooke-eva/gyazo/actions/workflows/ci.yaml/badge.svg)](https://github.com/brooke-eva/gyazo/actions/workflows/ci.yaml)

Better [Gyazo][gyazo] for Linux.

Install with:
```sh
cargo install --features cli --path .
```

Set a keybinding to `gyazo capture --open` to capture images, or `gyazo record --open` to record videos.

## Dependencies
- `import` command from [ImageMagick][imagemagick] for captures
- [`ffmpeg`][ffmpeg] and [`slop`][slop] commands for recordings

[ffmpeg]: https://github.com/FFmpeg/FFmpeg
[gyazo]: https://gyazo.com/
[imagemagick]: https://github.com/ImageMagick/ImageMagick
[slop]: https://github.com/naelstrof/slop
