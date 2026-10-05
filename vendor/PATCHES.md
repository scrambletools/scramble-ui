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
in right to left interfaces.

**Changes:**

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
(`diff -ruN` of `src/` against the crates.io releases). A test in
`src/font.rs` checks the cursor positions.

**Updating:** when iced updates, re-apply the patches to the new versions,
or drop the vendored copies if iced handles right to left text input.

