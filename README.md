# scramble-ui

Material Design 3 Expressive for [iced](https://iced.rs): color schemes
from a seed, type scale, icons, shapes, spring motion, buttons, toolbars,
side sheets, popovers, drop-downs, fields and resize handles, with the
desktop's accent color and light or dark preference (Omarchy, the XDG settings portal,
Windows and macOS). Shared by [prev](https://github.com/scrambletools/prev)
and [triib](https://github.com/scrambletools/triib).

## Using it

Cargo applies `[patch.crates-io]` only in the top level workspace, so an
app using this crate adds the same patch section as this crate's
`Cargo.toml`, pointing at `vendor/` (see `vendor/PATCHES.md`):

```toml
[patch.crates-io]
iced_graphics = { path = "../scramble-ui/vendor/iced_graphics" }
iced_widget = { path = "../scramble-ui/vendor/iced_widget" }
```

Load the fonts at startup with `scramble_ui::font::files()`, and set the
components' own labels in the app's language with
`scramble_ui::labels::set`.

## Icons

`src/icon.rs` lists every icon any app draws. After adding one, rebuild
the subset fonts:

```
uv run --with fonttools scripts/build-fonts.py
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option. The bundled fonts keep their own
licenses: Roboto Flex under the SIL Open Font License 1.1
(`assets/fonts/OFL.txt`), Material Symbols under Apache 2.0
(`assets/fonts/LICENSE-MaterialSymbols.txt`). The vendored iced crates are
MIT (`vendor/*/LICENSE`).
