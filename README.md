# AutoClicker

A simple GUI auto-clicker written in Rust using `eframe` and `egui`. It is designed for macOS and allows configuration of click speed, repeat count and an activation area. The clicker stops when the mouse leaves the configured region. Toggle the auto-clicker with **Cmd+D**.

## Features

- Configure clicks per second
- Optional repeat count (0 for infinite)
- Define a rectangular region; clicking stops when the cursor leaves it
- Global hotkey **Cmd+D** to start/stop

## Building

```
cargo build --release
```

Running the application will open a small window to configure the settings. Set the desired values and press `Start` or use **Cmd+D**.


To create a macOS DMG locally you can install `cargo-bundle` and `create-dmg`:


```bash
brew install create-dmg
cargo install cargo-bundle
cargo bundle --release
create-dmg target/release/bundle/osx/AutoClicker.app
```

A GitHub Actions workflow builds the DMG automatically whenever a tag starting with `v` is pushed.


## Tests

Unit tests cover the region bounds logic.
Run them with:

```
cargo test
```

