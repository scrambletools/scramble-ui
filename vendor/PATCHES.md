# Vendored dependencies

## iced_graphics 0.14.0 and iced_widget 0.14.2

Copied from crates.io. Cargo applies `[patch.crates-io]` only in the top
level workspace, so this crate's `Cargo.toml` and every app using it carry
the same patch section pointing here. License: MIT (see each crate's `LICENSE`).

**Why:** iced's text input and pick list assume left to right text. In
right to left text the cursor was drawn at the wrong character, a
selection was not drawn at all, the arrow keys moved the cursor against
the way they point, and a click or drag started left of the text began
at its start instead of its end. Pick lists kept their text on the left
in right to left interfaces. Right to left text drawn right aligned or
centred in unbounded width, as a canvas draws it, put its glyphs at
infinity, which overflowed the glyph cache.

**Changes:**

- `iced_graphics/src/text.rs`: `align` keeps the relayout right to left
  text needs when it clears a single line's alignment, so the line is
  laid out in its own width rather than the unbounded one, where
  cosmic-text starts right to left lines at the right edge.

- `iced_graphics/src/text/paragraph.rs`: `grapheme_position` finds the
  grapheme's glyph by its byte offset (glyphs are in visual order) and
  measures a right to left glyph from its right edge; past the end of the
  text it uses the logically last glyph.
- `iced_widget/src/text_input.rs`: a selection is drawn from its leftmost
  to its rightmost edge, since in right to left text its logical start is
  on the right; in text that starts right to left, Left and Right are
  swapped so the cursor moves the way the arrow points; a press left of
  such text puts the cursor at its end, which is on the left; and the
  cursor of an empty field goes by the value, not the placeholder, so in
  a right aligned field it sits at the right edge.
- `iced_widget/src/text_input.rs` also takes a `placeholder_align`, so
  a search box's placeholder can sit on the interface's side while an
  empty field's cursor follows the language typed in.
- `iced_widget/src/pick_list.rs` and `src/overlay/menu.rs`: a
  `right_to_left` option puts a pick list's text and its menu's options
  on the right and the handle on the left. The text is measured and
  drawn from its left edge, since right aligned text laid out wider than
  itself is misplaced.

The full diffs are `iced_graphics-rtl.patch` and `iced_widget-rtl.patch`
(`diff -ruN` of `src/` against the crates.io releases). Tests in
`src/font.rs` check the cursor positions and that right to left text
stays within its width however it is aligned.

**Updating:** when iced updates, re-apply the patches to the new versions,
or drop the vendored copies if iced handles right to left text input.


## winit 0.30.13

The same copy prev carries in its own `vendor/winit`, from crates.io with
prev's changes. License: Apache-2.0 (see `winit/LICENSE`).

**Why:** X11 and Wayland tell every client which of the keymap's layouts
is active, but winit keeps it to itself. `src/input.rs` reads it to set
the side an empty text field starts on.

**Changes used here:**

- `src/platform_impl/linux/common/xkb/state.rs`: the XKB state keeps the
  keymap it was made from, and whenever the active layout changes (and
  when the state is made) it looks up what that layout types on the
  three letter rows.
- `src/platform/keyboard_layout.rs` (new): `letters()` returns those
  letters; `src/input.rs` tells the layout's direction from their script,
  so it needs no list of layout names.

The copy also has prev's macOS changes, described in prev's
`vendor/PATCHES.md`. Files to open and drags act only when an app sets
their hooks; the menu bar, cursor and appearance fixes apply to every
iced app on macOS. The full diff is `winit.patch`.
