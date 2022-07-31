# Witcher 3 Terrain Texturing Editor

A terrain texturing editor for new hubs in "The Witcher 3: Wild Hunt" game by CD Projekt Red, part of radish community modding tools.

![Example Screenshot][img.example]

radish modding tools are a collection of community created modding tools aimed to enable the creation of new quests for "The Witcher 3: Wild Hunt" game by CDPR.

[![Example Video 1][img.example.small.1]][vid.example.1] [![Example Video 2][img.example.small.2]][vid.example.2]

The full package is available here: https://www.nexusmods.com/witcher3/mods/3620

## Building from Source

The project can be compiled with the `stable` rust-toolchain version 1.56 or higher but currently it requires a slightly patched version of bevy v0.7.

**The project is currently work-in-progress and requires some minor patches to a local bevy (v0.7) and bevy-egui repository**

1. Clone repositories:
  ```sh
  $ git clone https://github.com/bevyengine/bevy.git
  $ git clone https://github.com/mvlabat/bevy_egui.git
  $ git clone https://codeberg.org/rmemr/w3.terrain-texturing.git
  ```

2. Patch bevy v0.7 with [bevy-patch][bevy-patch]:
  ```sh
  $ cp w3.terrain-texturing/bevy-patch bevy/bevy-patch
  $ cd bevy
  $ git checkout v0.7.0
  $ git apply bevy-patch
  $ cd ..
  ```
  and bevy-egui v0.14 (to use local bevy repository) with [bevy_egui-patch][bevy_egui-patch]:
  ```sh
  $ cp w3.terrain-texturing/bevy_egui-patch bevy_egui/bevy_egui-patch
  $ cd bevy_egui
  $ git checkout 022ebd1f25dc6296fa494bb5cf42e7858dee202a
  $ git apply bevy_egui-patch
  $ cd ..
  ```
  **Note**: it may be necessary to add the `--ignore-whitespace` option if the patching produces errors:
  ```sh
  $ git apply --ignore-whitespace bevy-patch
  ```

3. On windows the dynamic linking has to by deactivated by removing the `"dynmic"` line from
  ```sh
  w3.terrain-texturing/Cargo.toml
  ```
  and
  ```sh
  bevy_egui/Cargo.toml
  ```

4. Compile terrain editor in release mode (debug mode image reading is very slow):
  ```sh
  $ cd w3.terrain-texturing
  $ cargo build --release
  ```

## Usage

The core texturing features are implemented and the editor can be started with:

```sh
  cargo run --release
```

The `Debug` menu provides some predefined test-terrain loading options.

**Important Note:** At the moment all settings and filepaths are hard-coded (see [config.rs][terrain-config] and [config.rs][material-config]) and no example terrain and texture data are provided in the repository. The editor assumes the following image formats:

  - heightmap: 16bit grayscale png
  - background texture map: indexed color 8bit png, palette with exactly 32 entries
  - overlay texture map: indexed color 8bit png, palette with exactly 32 entries
  - blendcontrol map: indexed color 8bit png, palette with exactly 64 entries
  - tint map: 8bit RGBA png image

  - material textures (normal & diffuse): 1024x1024 pixel 8bit RGBA png

with the hardcoded names in the appropriate folders.

A free-cam can be activated and deactivated with the Left-Ctrl key. Camera position is controlled with W-A-S-D keys while Q and E control height. Orientation is controlled with the mouse.

In the material palette the left mouse button selects the overlay material and right mouse button selects background material. Brush size can be changed with the slider or via mouse-wheel.

**Important Note:** Saving changes is not implemented, yet.

## Contributing

First: thank you for your interest! There are many ways to contribute. You can write bug reports, create pull requests to fix bugs or add new features or write documentation.

Please make sure your pull requests:
  * reference a ticket, if you want to add a new feature please make a ticket first and outline what you are trying to add
  * is formatted with rustfmt
  * compiles with the current main branch

If you have questions, you can find me on the [radish modding tools discord server][radishtools-discord].

## License

All code for the editor in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option. You're welcome.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions.

[bevy-patch]:                bevy-patch
[bevy_egui-patch]:           bevy_egui-patch
[terrain-config]:            https://codeberg.org/rmemr/w3.terrain-texturing/src/branch/main/src/config.rs#L257
[material-config]:           https://codeberg.org/rmemr/w3.terrain-texturing/src/branch/main/src/config.rs#L311
[radishtools-discord]:       https://discord.gg/R7Jpzfv

[img.example]:               example.screenshot.png
[img.example.small.1]:       example.video.1.png
[img.example.small.2]:       example.video.2.png
[vid.example.1]:             https://streamable.com/vwbrfo
[vid.example.2]:             https://streamable.com/xy9mmu
