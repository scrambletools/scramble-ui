//! A drop-down under a button, to pick one value, the button showing the
//! one picked, or to choose an action. The button shows its label, cut
//! short with an ellipsis when it has no room, and an arrow; the menu
//! opens under it, as wide as its longest entry and at least as wide as
//! the button, and closes on a choice, a click outside it or Escape. It
//! keeps whether it is open itself, as iced's pick list does.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad};
use iced::advanced::text::{self, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::alignment::Vertical;
use iced::mouse::{self, Cursor};
use iced::widget::text::LineHeight;
use iced::{
    Background, Border, Color, Element, Event, Fill, Font, Length, Pixels, Point, Rectangle, Size,
    Theme, Vector, keyboard, touch,
};

use super::button::Size as Height;
use super::font::{self, Type};
use super::icon::Icon;
use super::{Scheme, component, dir, faded, shape, state_layer};

const GAP: f32 = 4.0;
const ARROW: f32 = 18.0;
/// Between the label and the arrow.
const ARROW_GAP: f32 = 4.0;
/// What a menu row adds to its label: its padding either side and room to
/// spare; with an icon, the icon and the gap after it.
const ROW_ROOM: f32 = 2.0 * 16.0 + 8.0;
const ICON_ROOM: f32 = 20.0 + 12.0;

/// How the button looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Look {
    /// Plain until pointed at, as a table's heading or a choice in a bar.
    #[default]
    Quiet,
    /// Outlined, as a form's field.
    Outlined,
}

/// A line in the menu.
pub enum Entry<Message> {
    Item {
        icon: Option<Icon>,
        label: String,
        message: Message,
        /// Shown as the one picked.
        checked: bool,
    },
    /// A line between groups of items.
    Divider,
}

impl<Message> Entry<Message> {
    /// An item that sends `message` when chosen.
    pub fn item(label: impl Into<String>, message: Message) -> Self {
        Entry::Item {
            icon: None,
            label: label.into(),
            message,
            checked: false,
        }
    }

    pub fn divider() -> Self {
        Entry::Divider
    }

    /// The item with an icon before its label.
    pub fn icon(mut self, glyph: Icon) -> Self {
        if let Entry::Item { icon, .. } = &mut self {
            *icon = Some(glyph);
        }
        self
    }

    /// The item shown as the one picked, or not.
    pub fn checked(mut self, picked: bool) -> Self {
        if let Entry::Item { checked, .. } = &mut self {
            *checked = picked;
        }
        self
    }
}

/// A drop-down, made with [`pick`], [`menu`] or [`icon_menu`].
pub struct Dropdown<Message> {
    label: String,
    /// The icon an icon button shows in place of a label and arrow.
    glyph: Option<Icon>,
    /// The label stands in for a value not picked yet.
    placeholder: bool,
    entries: Vec<Entry<Message>>,
    look: Look,
    width: Length,
    height: Height,
    text: Option<Type>,
}

/// A drop-down that picks one of `options`, the button showing the one
/// `selected`; `on_pick` makes the message for each. Outlined, as a form's
/// field, unless given another look.
pub fn pick<T, Message>(
    options: impl IntoIterator<Item = T>,
    selected: Option<T>,
    on_pick: impl Fn(T) -> Message,
) -> Dropdown<Message>
where
    T: ToString + PartialEq,
{
    let entries = options
        .into_iter()
        .map(|option| {
            let checked = selected.as_ref() == Some(&option);
            Entry::item(option.to_string(), on_pick(option)).checked(checked)
        })
        .collect();
    let label = selected.as_ref().map(ToString::to_string);
    Dropdown {
        placeholder: label.is_none(),
        label: label.unwrap_or_default(),
        glyph: None,
        entries,
        look: Look::Outlined,
        width: Length::Shrink,
        height: Height::Small,
        text: None,
    }
}

/// A drop-down of actions under a quiet button labelled `label`.
pub fn menu<Message>(label: impl Into<String>, entries: Vec<Entry<Message>>) -> Dropdown<Message> {
    Dropdown {
        label: label.into(),
        glyph: None,
        placeholder: false,
        entries,
        look: Look::Quiet,
        width: Length::Shrink,
        height: Height::ExtraSmall,
        text: None,
    }
}

/// A drop-down of actions under a round icon button, as one that adds
/// something of a kind picked from the menu.
pub fn icon_menu<Message>(glyph: Icon, entries: Vec<Entry<Message>>) -> Dropdown<Message> {
    Dropdown {
        glyph: Some(glyph),
        ..menu(String::new(), entries)
    }
}

impl<Message> Dropdown<Message> {
    /// What the button shows while nothing is picked.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        if self.placeholder {
            self.label = placeholder.into();
        }
        self
    }

    pub fn look(mut self, look: Look) -> Self {
        self.look = look;
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// The button's height: extra small (32 pixels) for headings and bars,
    /// small (40) for forms.
    pub fn size(mut self, height: Height) -> Self {
        self.height = height;
        self
    }

    /// The type style of the button's label: by default a label's for the
    /// quiet look and body text for the outlined one, smaller on an extra
    /// small button.
    pub fn text(mut self, style: Type) -> Self {
        self.text = Some(style);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Dropdown<Message>> for Element<'a, Message> {
    fn from(dropdown: Dropdown<Message>) -> Self {
        let items = dropdown
            .entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Item { icon, label, .. } => Some((label.clone(), icon.is_some())),
                Entry::Divider => None,
            })
            .collect();
        // Laid out as layout made now is, as a pick list lays itself out.
        let mirrored = dir::mirrored();
        // The menu reads in the interface's direction, even from a bar.
        let _reading = dir::reading();
        let rows = dropdown.entries.into_iter().map(|entry| match entry {
            Entry::Item {
                icon,
                label,
                message,
                checked,
            } => component::list_row(icon, label, 0.0, checked, Some(message)).into(),
            Entry::Divider => iced::widget::rule::horizontal(1).into(),
        });
        let rows = iced::widget::Column::with_children(rows).width(Fill);
        let menu = super::popover::surface(component::scroll(rows).width(Fill));
        Element::new(DropdownWidget {
            text: dropdown
                .text
                .unwrap_or(match (dropdown.look, dropdown.height) {
                    (Look::Quiet, _) => Type::LabelLarge,
                    (Look::Outlined, Height::ExtraSmall) => Type::BodyMedium,
                    (Look::Outlined, Height::Small | Height::Medium) => Type::BodyLarge,
                }),
            padding: match dropdown.height {
                Height::ExtraSmall => 12.0,
                Height::Small | Height::Medium => 16.0,
            },
            label: dropdown.label,
            glyph: dropdown.glyph,
            mirrored,
            placeholder: dropdown.placeholder,
            look: dropdown.look,
            width: dropdown.width,
            height: dropdown.height.height(),
            items,
            menu,
        })
    }
}

struct DropdownWidget<'a, Message> {
    label: String,
    glyph: Option<Icon>,
    /// The label at the end and the arrow at the start, in right to left
    /// languages.
    mirrored: bool,
    placeholder: bool,
    look: Look,
    width: Length,
    height: f32,
    padding: f32,
    text: Type,
    /// Each item's label and whether it has an icon, to size the menu by.
    items: Vec<(String, bool)>,
    menu: Element<'a, Message>,
}

#[derive(Default)]
struct State {
    open: bool,
    hovered: bool,
    /// The label last measured, in its type style, and its width.
    measured: Option<(String, Type, f32)>,
    /// The label as it fits the button.
    shown: String,
}

/// `content` cut short with an ellipsis to fit `room` pixels.
fn fit(content: &str, style: Type, room: f32) -> String {
    if font::measure(content, style) <= room {
        return content.to_owned();
    }
    // Where each prefix of `content` ends, shortest first.
    let ends: Vec<usize> = content
        .char_indices()
        .map(|(index, _)| index)
        .skip(1)
        .chain(std::iter::once(content.len()))
        .collect();
    let cut = |count: usize| format!("{}…", content[..ends[count - 1]].trim_end());
    // The most characters that fit, found by halving.
    let (mut low, mut high) = (0, ends.len() - 1);
    while low < high {
        let middle = (low + high).div_ceil(2);
        if font::measure(&cut(middle), style) <= room {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    if low == 0 { "…".to_owned() } else { cut(low) }
}

/// One line of text laid out as the button draws it.
fn line(content: String, font: Font, size: f32, line_height: f32, width: f32) -> text::Text {
    text::Text {
        content,
        bounds: Size::new(width, line_height),
        size: Pixels(size),
        line_height: LineHeight::Absolute(Pixels(line_height)),
        font,
        align_x: text::Alignment::Left,
        align_y: Vertical::Center,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, iced::Renderer>
    for DropdownWidget<'a, Message>
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.menu)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.menu));
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Fixed(self.height))
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        if self.glyph.is_some() {
            let side = Length::Fixed(self.height);
            return layout::Node::new(limits.resolve(
                side,
                side,
                Size::new(self.height, self.height),
            ));
        }
        let state = tree.state.downcast_mut::<State>();
        let label_width = match &state.measured {
            Some((label, style, width)) if *label == self.label && *style == self.text => *width,
            _ => {
                let width = font::measure(&self.label, self.text);
                state.measured = Some((self.label.clone(), self.text, width));
                width
            }
        };
        let around = 2.0 * self.padding + ARROW_GAP + ARROW;
        let size = limits.resolve(
            self.width,
            Length::Fixed(self.height),
            Size::new(label_width + around, self.height),
        );
        let room = (size.width - around).max(0.0);
        state.shown = if label_width <= room {
            self.label.clone()
        } else {
            fit(&self.label, self.text, room)
        };
        layout::Node::new(size)
    }

    fn operate(
        &mut self,
        _tree: &mut Tree,
        layout: Layout<'_>,
        _renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if self.glyph.is_none() {
            operation.text(None, layout.bounds(), &self.label);
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let hovered = cursor.is_over(layout.bounds());
        if hovered != state.hovered {
            state.hovered = hovered;
            shell.request_redraw();
        }
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        | Event::Touch(touch::Event::FingerPressed { .. }) = event
            && hovered
        {
            state.open = !state.open;
            shell.capture_event();
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let state = tree.state.downcast_ref::<State>();
        let scheme = Scheme::of(theme);
        let bounds = layout.bounds();
        let content = match (self.look, state.open) {
            _ if self.placeholder => scheme.on_surface_variant,
            (Look::Quiet, true) => scheme.on_secondary_container,
            (Look::Quiet, false) => scheme.on_surface_variant,
            (Look::Outlined, _) => scheme.on_surface,
        };
        let border = match self.look {
            Look::Quiet => Border {
                radius: (bounds.height / 2.0).into(),
                ..Border::default()
            },
            Look::Outlined => {
                let (color, width) = if state.open {
                    (scheme.primary, 2.0)
                } else if state.hovered {
                    (scheme.on_surface, 1.0)
                } else {
                    (scheme.outline, 1.0)
                };
                Border {
                    color,
                    width,
                    radius: shape::EXTRA_SMALL.into(),
                }
            }
        };
        let container = match self.look {
            Look::Quiet if state.open => scheme.secondary_container,
            _ => Color::TRANSPARENT,
        };
        renderer.fill_quad(
            Quad {
                bounds,
                border,
                ..Quad::default()
            },
            Background::Color(container),
        );
        if self.look == Look::Quiet && state.hovered {
            renderer.fill_quad(
                Quad {
                    bounds,
                    border,
                    ..Quad::default()
                },
                Background::Color(faded(content, state_layer::HOVERED)),
            );
        }

        if let Some(glyph) = self.glyph {
            let side = Height::ExtraSmall.icon();
            renderer.fill_text(
                text::Text {
                    align_x: text::Alignment::Center,
                    shaping: text::Shaping::Basic,
                    ..line(
                        glyph.codepoint().to_string(),
                        if state.open {
                            font::ICONS_FILLED
                        } else {
                            font::ICONS
                        },
                        side,
                        side,
                        side,
                    )
                },
                Point::new(bounds.center_x(), bounds.center_y()),
                content,
                *viewport,
            );
            return;
        }

        // The label at the start and the arrow at the end.
        let room = (bounds.width - 2.0 * self.padding - ARROW_GAP - ARROW).max(0.0);
        let align_x = if self.mirrored {
            text::Alignment::Right
        } else {
            text::Alignment::Left
        };
        let (label_x, arrow_x) = if self.mirrored {
            (
                bounds.x + bounds.width - self.padding,
                bounds.x + self.padding + ARROW,
            )
        } else {
            (
                bounds.x + self.padding,
                bounds.x + bounds.width - self.padding - ARROW,
            )
        };
        let (size, line_height) = self.text.metrics();
        renderer.fill_text(
            text::Text {
                align_x,
                ..line(
                    state.shown.clone(),
                    self.text.font(false),
                    size,
                    line_height,
                    room,
                )
            },
            Point::new(label_x, bounds.center_y()),
            content,
            *viewport,
        );
        renderer.fill_text(
            text::Text {
                align_x,
                shaping: text::Shaping::Basic,
                ..line(
                    Icon::ArrowDropDown.codepoint().to_string(),
                    font::ICONS,
                    ARROW,
                    ARROW,
                    ARROW,
                )
            },
            Point::new(arrow_x, bounds.center_y()),
            content,
            *viewport,
        );
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &iced::Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();
        if !state.open {
            return None;
        }
        let bounds = layout.bounds();
        Some(overlay::Element::new(Box::new(Menu {
            content: &mut self.menu,
            tree: &mut children[0],
            open: &mut state.open,
            items: &self.items,
            anchor: bounds + translation,
            mirrored: self.mirrored,
        })))
    }
}

struct Menu<'a, 'b, Message> {
    content: &'a mut Element<'b, Message>,
    tree: &'a mut Tree,
    open: &'a mut bool,
    items: &'a [(String, bool)],
    anchor: Rectangle,
    mirrored: bool,
}

impl<Message> Menu<'_, '_, Message> {
    /// The width the longest item needs.
    fn natural_width(&self) -> f32 {
        self.items
            .iter()
            .map(|(label, icon)| {
                font::measure(label, Type::LabelLarge)
                    + ROW_ROOM
                    + if *icon { ICON_ROOM } else { 0.0 }
            })
            .fold(0.0, f32::max)
    }
}

impl<Message: Clone> overlay::Overlay<Message, Theme, iced::Renderer> for Menu<'_, '_, Message> {
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        // As wide as the longest item and at least the button, within the
        // window; as tall as its items or the room on the roomier side.
        let width = self
            .natural_width()
            .max(self.anchor.width)
            .min(bounds.width)
            .ceil();
        let below = bounds.height - (self.anchor.y + self.anchor.height + GAP);
        let above = self.anchor.y - GAP;
        let limits = layout::Limits::new(
            Size::new(width, 0.0),
            Size::new(width, below.max(above).max(0.0)),
        );
        let node = self
            .content
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);
        let size = node.size();
        // Lined up with the button's start.
        let x = if self.mirrored {
            self.anchor.x + self.anchor.width - size.width
        } else {
            self.anchor.x
        };
        let x = x.min(bounds.width - size.width).max(0.0);
        let y = if size.height <= below || below >= above {
            self.anchor.y + self.anchor.height + GAP
        } else {
            self.anchor.y - GAP - size.height
        };
        node.move_to(Point::new(x, y.max(0.0)))
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        let bounds = layout.bounds();
        self.content
            .as_widget()
            .draw(self.tree, renderer, theme, style, layout, cursor, &bounds);
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let bounds = layout.bounds();
        match event {
            // A press on the button closes the menu through the button.
            Event::Mouse(mouse::Event::ButtonPressed(_))
            | Event::Touch(touch::Event::FingerPressed { .. })
                if !cursor.is_over(bounds) && !cursor.is_over(self.anchor) =>
            {
                *self.open = false;
                shell.request_redraw();
                return;
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => {
                *self.open = false;
                shell.capture_event();
                shell.request_redraw();
                return;
            }
            _ => {}
        }
        let unchosen = shell.is_empty();
        self.content.as_widget_mut().update(
            self.tree, event, layout, cursor, renderer, clipboard, shell, &bounds,
        );
        if unchosen && !shell.is_empty() {
            *self.open = false;
            shell.request_redraw();
        }
        if matches!(event, Event::Mouse(_)) && cursor.is_over(bounds) {
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &layout.bounds(),
            renderer,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_mark_the_one_selected() {
        let dropdown = pick(["Light", "Dark"], Some("Dark"), str::len);
        assert_eq!(dropdown.label, "Dark");
        assert!(!dropdown.placeholder);
        let checked: Vec<bool> = dropdown
            .entries
            .iter()
            .map(|entry| matches!(entry, Entry::Item { checked: true, .. }))
            .collect();
        assert_eq!(checked, [false, true]);
        let unpicked = pick(["Light", "Dark"], None, str::len).placeholder("Pick one");
        assert_eq!(unpicked.label, "Pick one");
        assert!(unpicked.placeholder);
        // A placeholder does not replace a value picked.
        let picked = pick(["Light"], Some("Light"), str::len).placeholder("Pick one");
        assert_eq!(picked.label, "Light");
    }

    #[test]
    fn entries_take_icons_and_checks() {
        let entry = Entry::item("Remove column", 1)
            .icon(Icon::Close)
            .checked(true);
        assert!(matches!(
            entry,
            Entry::Item {
                icon: Some(Icon::Close),
                checked: true,
                ..
            }
        ));
        assert!(matches!(
            Entry::<u8>::divider().icon(Icon::Close),
            Entry::Divider
        ));
    }

    #[test]
    fn long_labels_end_in_an_ellipsis() {
        let label = "Stream input 1 from the stage box";
        let width = font::measure(label, Type::LabelLarge);
        assert_eq!(fit(label, Type::LabelLarge, width), label);
        let fitted = fit(label, Type::LabelLarge, width / 2.0);
        assert!(fitted.ends_with('…'), "{fitted}");
        assert!(font::measure(&fitted, Type::LabelLarge) <= width / 2.0);
        assert!(fitted.len() > "…".len() + 4, "{fitted}");
        assert_eq!(fit(label, Type::LabelLarge, 0.0), "…");
    }

    /// A window with a drop-down picking a shade and a word beside it.
    fn window(look: Look) -> iced_test::Simulator<'static, &'static str> {
        let content = iced::widget::row![
            pick(["Light", "Dark"], Some("Light"), |shade| shade).look(look),
            iced::widget::text("Aside"),
        ]
        .spacing(40)
        .padding(20);
        iced_test::Simulator::with_size(
            iced::Settings {
                fonts: font::files().collect(),
                default_font: font::TEXT,
                ..iced::Settings::default()
            },
            Size::new(400.0, 300.0),
            content,
        )
    }

    #[test]
    fn a_choice_sends_its_message_and_closes_the_menu() {
        for look in [Look::Quiet, Look::Outlined] {
            let mut simulator = window(look);
            assert!(simulator.find("Dark").is_err(), "closed at first");
            simulator.click("Light").expect("the button");
            simulator.click("Dark").expect("the menu's item");
            assert!(simulator.find("Dark").is_err(), "closed after choosing");
            assert_eq!(simulator.into_messages().collect::<Vec<_>>(), ["Dark"]);
        }
    }

    #[test]
    fn escape_a_click_outside_or_the_button_closes_the_menu() {
        let mut simulator = window(Look::Outlined);
        let open = |simulator: &mut iced_test::Simulator<'_, &str>| {
            simulator.click("Light").expect("the button");
            assert!(simulator.find("Dark").is_ok(), "open");
        };
        open(&mut simulator);
        let _ = simulator.tap_key(keyboard::key::Named::Escape);
        assert!(simulator.find("Dark").is_err(), "closed by Escape");
        open(&mut simulator);
        simulator.click("Aside").expect("the word beside");
        assert!(simulator.find("Dark").is_err(), "closed by a click outside");
        open(&mut simulator);
        simulator.click("Light").expect("the button");
        assert!(simulator.find("Dark").is_err(), "closed by the button");
        assert_eq!(simulator.into_messages().count(), 0);
    }
}
