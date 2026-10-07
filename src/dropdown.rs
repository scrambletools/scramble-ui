//! A drop-down under a button, to pick one value, the button showing the
//! one picked, or to choose an action. The button shows its label, cut
//! short with an ellipsis when it has no room, and an arrow; the menu
//! opens under it, as wide as its longest entry and at least as wide as
//! the button, and closes on a choice, a click outside it or Escape; a
//! choice kept open, as a style to try, leaves it open. Besides items and
//! dividers, the menu takes headings, notes, color swatches, connected
//! buttons and pictures. It keeps whether it is open itself, as iced's
//! pick list does, unless the app keeps it, to open it itself.

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
    /// Plain until pointed at, as a table's heading; the default.
    #[default]
    Quiet,
    /// Outlined, for a form that boxes its fields.
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
        /// Choosing it leaves the menu open, as for a style to try.
        stays: bool,
    },
    /// A line between groups of items.
    Divider,
    /// A small heading over a group of items.
    Heading(String),
    /// A line of quiet text, such as a note that the menu is empty.
    Note(String),
    /// Round color swatches in rows of `columns`; a swatch without a color
    /// stands for none.
    Swatches {
        swatches: Vec<Swatch<Message>>,
        columns: usize,
        stays: bool,
    },
    /// A row of connected buttons, one of them picked, such as sizes or
    /// alignments.
    Segments {
        segments: Vec<Segment<Message>>,
        stays: bool,
    },
    /// A picture with its label under it, such as a saved signature, and
    /// optionally a small action beside it that leaves the menu open.
    Picture {
        image: iced::widget::image::Handle,
        label: String,
        message: Message,
        action: Option<Action<Message>>,
    },
}

/// A swatch in [`Entry::Swatches`].
pub struct Swatch<Message> {
    pub color: Option<Color>,
    pub selected: bool,
    pub message: Message,
}

/// A button in [`Entry::Segments`]: its icon, or its label when it has
/// none.
pub struct Segment<Message> {
    pub icon: Option<Icon>,
    pub label: String,
    pub selected: bool,
    pub message: Message,
}

/// The small icon button beside an [`Entry::Picture`], with its tooltip.
pub struct Action<Message> {
    pub icon: Icon,
    pub label: String,
    pub message: Message,
}

impl<Message> Entry<Message> {
    /// An item that sends `message` when chosen.
    pub fn item(label: impl Into<String>, message: Message) -> Self {
        Entry::Item {
            icon: None,
            label: label.into(),
            message,
            checked: false,
            stays: false,
        }
    }

    pub fn divider() -> Self {
        Entry::Divider
    }

    pub fn heading(label: impl Into<String>) -> Self {
        Entry::Heading(label.into())
    }

    pub fn note(text: impl Into<String>) -> Self {
        Entry::Note(text.into())
    }

    /// Swatches in rows of `columns`.
    pub fn swatches(swatches: Vec<Swatch<Message>>, columns: usize) -> Self {
        Entry::Swatches {
            swatches,
            columns: columns.max(1),
            stays: false,
        }
    }

    pub fn segments(segments: Vec<Segment<Message>>) -> Self {
        Entry::Segments {
            segments,
            stays: false,
        }
    }

    /// A picture that sends `message` when chosen.
    pub fn picture(
        image: iced::widget::image::Handle,
        label: impl Into<String>,
        message: Message,
    ) -> Self {
        Entry::Picture {
            image,
            label: label.into(),
            message,
            action: None,
        }
    }

    /// The picture with a small action beside it.
    pub fn action(mut self, glyph: Icon, label: impl Into<String>, message: Message) -> Self {
        if let Entry::Picture { action, .. } = &mut self {
            *action = Some(Action {
                icon: glyph,
                label: label.into(),
                message,
            });
        }
        self
    }

    /// Choosing this entry leaves the menu open, to try a few.
    pub fn keep_open(mut self) -> Self {
        if let Entry::Item { stays, .. }
        | Entry::Swatches { stays, .. }
        | Entry::Segments { stays, .. } = &mut self
        {
            *stays = true;
        }
        self
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
    /// The label is the value picked, shown as body text, not a label.
    value: bool,
    entries: Vec<Entry<Message>>,
    look: Look,
    width: Length,
    height: Height,
    text: Option<Type>,
    /// A second line on the button, under its label.
    detail: Option<String>,
    /// Shown as selected while closed too, as a tool in use.
    selected: bool,
    /// The button's tooltip.
    tip: Option<String>,
    /// The narrowest the menu gets, for entries text does not size.
    menu_width: f32,
    /// Whether the menu is open, kept by the app, and the message asking
    /// to open or close it; the drop-down keeps it itself without.
    open: Option<(bool, OnToggle<Message>)>,
}

type OnToggle<Message> = std::sync::Arc<dyn Fn(bool) -> Message + Send + Sync>;

/// A drop-down that picks one of `options`, the button showing the one
/// `selected`; `on_pick` makes the message for each.
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
        value: true,
        label: label.unwrap_or_default(),
        glyph: None,
        entries,
        look: Look::Quiet,
        width: Length::Shrink,
        height: Height::Small,
        text: None,
        detail: None,
        selected: false,
        tip: None,
        menu_width: 0.0,
        open: None,
    }
}

/// A drop-down of actions under a quiet button labelled `label`.
pub fn menu<Message>(label: impl Into<String>, entries: Vec<Entry<Message>>) -> Dropdown<Message> {
    Dropdown {
        label: label.into(),
        glyph: None,
        placeholder: false,
        value: false,
        entries,
        look: Look::Quiet,
        width: Length::Shrink,
        height: Height::ExtraSmall,
        text: None,
        detail: None,
        selected: false,
        tip: None,
        menu_width: 0.0,
        open: None,
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

    /// A second line on the button, under its label, as the group a value
    /// picked belongs to.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Shows the button as selected while its menu is closed, as a tool
    /// in use.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// A tooltip on the button, as an icon button's name; hidden while
    /// the app holds the menu open.
    pub fn tip(mut self, label: impl Into<String>) -> Self {
        self.tip = Some(label.into());
        self
    }

    /// The narrowest the menu gets, such as for pictures.
    pub fn menu_width(mut self, width: f32) -> Self {
        self.menu_width = width;
        self
    }

    /// Lets the app keep whether the menu is open, such as to open it
    /// itself: `on_toggle` makes the message asking to open or close it,
    /// sent on a press of the button, a choice, a click outside or Escape.
    pub fn open(
        mut self,
        open: bool,
        on_toggle: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.open = Some((open, std::sync::Arc::new(on_toggle)));
        self
    }

    /// The type style of the button's label: by default body text for a
    /// value picked, smaller on an extra small button, and a label's for a
    /// menu of actions.
    pub fn text(mut self, style: Type) -> Self {
        self.text = Some(style);
        self
    }
}

impl<'a, Message: Clone + 'a> From<Dropdown<Message>> for Element<'a, Message> {
    fn from(dropdown: Dropdown<Message>) -> Self {
        let natural = natural_width(&dropdown.entries).max(dropdown.menu_width);
        let held_open = matches!(dropdown.open, Some((true, _)));
        // Laid out as layout made now is, as a pick list lays itself out.
        let mirrored = dir::mirrored();
        // The menu reads in the interface's direction, even from a bar.
        let _reading = dir::reading();
        let rows = dropdown.entries.into_iter().map(entry_view);
        let rows = iced::widget::Column::with_children(rows).width(Fill);
        let menu = super::popover::surface(component::scroll(rows).width(Fill));
        let widget = Element::new(DropdownWidget {
            text: dropdown
                .text
                .unwrap_or(match (dropdown.value, dropdown.height) {
                    (false, _) => Type::LabelLarge,
                    (true, Height::ExtraSmall) => Type::BodyMedium,
                    (true, Height::Small | Height::Medium) => Type::BodyLarge,
                }),
            padding: match dropdown.height {
                Height::ExtraSmall => 12.0,
                Height::Small | Height::Medium => 16.0,
            },
            label: dropdown.label,
            glyph: dropdown.glyph,
            mirrored,
            placeholder: dropdown.placeholder,
            value: dropdown.value,
            look: dropdown.look,
            width: dropdown.width,
            height: dropdown.height.height(),
            natural,
            detail: dropdown.detail,
            selected: dropdown.selected,
            open: dropdown.open,
            menu,
        });
        // A tooltip would cover the top of a menu the app holds open.
        match dropdown.tip {
            Some(tip) if !held_open => component::tip(widget, tip),
            _ => widget,
        }
    }
}

/// A menu's entries on its surface, for a menu the app places and closes
/// itself, such as a form field's choices over a page; as wide as its
/// widest entry and at least `width`.
pub fn surface<'a, Message: Clone + 'a>(
    entries: Vec<Entry<Message>>,
    width: f32,
) -> Element<'a, Message> {
    let natural = natural_width(&entries).max(width).ceil();
    let _reading = dir::reading();
    let rows = entries.into_iter().map(entry_view);
    let rows = iced::widget::Column::with_children(rows).width(natural);
    Element::from(super::popover::surface(component::scroll(rows)))
        .map(|pick: Pick<Message>| pick.message)
}

/// What a row of the menu sends: the app's message, and whether it closes
/// the menu.
#[derive(Clone)]
struct Pick<Message> {
    message: Message,
    close: bool,
}

/// The width the widest entry needs.
fn natural_width<Message>(entries: &[Entry<Message>]) -> f32 {
    entries
        .iter()
        .map(|entry| match entry {
            Entry::Item { icon, label, .. } => {
                font::measure(label, Type::LabelLarge)
                    + ROW_ROOM
                    + if icon.is_some() { ICON_ROOM } else { 0.0 }
            }
            Entry::Heading(label) => font::measure(label, Type::LabelMedium) + 2.0 * 16.0,
            Entry::Note(note) => font::measure(note, Type::BodyMedium) + 2.0 * 16.0,
            Entry::Swatches {
                swatches, columns, ..
            } => {
                let across = (*columns).min(swatches.len()) as f32;
                across * SWATCH + (across - 1.0).max(0.0) * SWATCH_GAP + 2.0 * 12.0
            }
            Entry::Segments { segments, .. } => {
                segments
                    .iter()
                    .map(|segment| match segment.icon {
                        Some(_) => 32.0,
                        None => font::measure(&segment.label, Type::LabelLarge) + 2.0 * 12.0,
                    })
                    .sum::<f32>()
                    + 2.0 * segments.len() as f32
                    + 2.0 * 12.0
            }
            Entry::Divider | Entry::Picture { .. } => 0.0,
        })
        .fold(0.0, f32::max)
}

/// The width of a swatch, and the gap between swatches.
const SWATCH: f32 = component::SWATCH_WIDTH;
const SWATCH_GAP: f32 = 2.0;

/// A line of the menu.
fn entry_view<'a, Message: Clone + 'a>(entry: Entry<Message>) -> Element<'a, Pick<Message>> {
    use iced::widget::{column, container};
    let quiet = |label: String, style: Type| {
        super::aligned(super::styled(label, style).style(super::style::on_surface_variant))
    };
    match entry {
        Entry::Item {
            icon,
            label,
            message,
            checked,
            stays,
        } => component::list_row(
            icon,
            label,
            0.0,
            checked,
            Some(Pick {
                message,
                close: !stays,
            }),
        )
        .into(),
        Entry::Divider => container(iced::widget::rule::horizontal(1).style(super::style::divider))
            .padding([4, 0])
            .into(),
        Entry::Heading(label) => container(quiet(label, Type::LabelMedium))
            .padding(iced::Padding {
                top: 8.0,
                right: 16.0,
                bottom: 4.0,
                left: 16.0,
            })
            .into(),
        Entry::Note(note) => container(quiet(note, Type::BodyMedium))
            .padding([8, 16])
            .into(),
        Entry::Swatches {
            swatches,
            columns,
            stays,
        } => {
            let mut grid = column![].spacing(SWATCH_GAP).padding([4, 12]);
            let mut swatches = swatches.into_iter().peekable();
            while swatches.peek().is_some() {
                let line = swatches.by_ref().take(columns).map(|one| {
                    component::swatch(
                        one.color,
                        one.selected,
                        Pick {
                            message: one.message,
                            close: !stays,
                        },
                    )
                });
                grid = grid.push(crate::dir::row(line).spacing(SWATCH_GAP));
            }
            grid.into()
        }
        Entry::Segments { segments, stays } => container(component::connected(
            segments
                .into_iter()
                .map(|segment| {
                    let button = match segment.icon {
                        Some(glyph) => super::icon_button(glyph),
                        None => super::button(super::button::Kind::Tonal, segment.label),
                    };
                    button
                        .kind(super::button::Kind::Tonal)
                        .size(Height::ExtraSmall)
                        .selected(segment.selected)
                        .on_press(Pick {
                            message: segment.message,
                            close: !stays,
                        })
                })
                .collect(),
        ))
        .padding([4, 12])
        .into(),
        Entry::Picture {
            image,
            label,
            message,
            action,
        } => {
            let preview = container(
                iced::widget::image(image)
                    .height(36)
                    .content_fit(iced::ContentFit::Contain),
            )
            .width(Fill)
            .height(44)
            .align_y(Vertical::Center)
            .padding([4, 8])
            // On white, as the picture will be on a page.
            .style(|_| iced::widget::container::Style {
                background: Some(Color::WHITE.into()),
                border: iced::border::rounded(shape::SMALL),
                ..Default::default()
            });
            let entry = super::button::custom(
                super::button::Kind::Row,
                column![preview, quiet(label, Type::LabelMedium)]
                    .spacing(4)
                    .padding([6, 0]),
            )
            .height(76.0)
            .width(Fill)
            .shape(super::button::Shape::Square)
            .on_press(Pick {
                message,
                close: true,
            });
            let line: Element<'a, Pick<Message>> = match action {
                Some(action) => crate::row![
                    entry,
                    component::tip(
                        super::icon_button(action.icon)
                            .size(Height::ExtraSmall)
                            .on_press(Pick {
                                message: action.message,
                                close: false,
                            }),
                        action.label,
                    )
                ]
                .spacing(4)
                .align_y(Vertical::Center)
                .into(),
                None => entry.into(),
            };
            container(line).padding([0, 8]).into()
        }
    }
}

struct DropdownWidget<'a, Message> {
    /// The label is the value picked.
    value: bool,
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
    /// The width the menu's widest entry needs.
    natural: f32,
    detail: Option<String>,
    selected: bool,
    open: Option<(bool, OnToggle<Message>)>,
    menu: Element<'a, Pick<Message>>,
}

impl<Message> DropdownWidget<'_, Message> {
    fn is_open(&self, state: &State) -> bool {
        match &self.open {
            Some((open, _)) => *open,
            None => state.open,
        }
    }
}

#[derive(Default)]
struct State {
    open: bool,
    hovered: bool,
    /// The label last measured, in its type style, and its width.
    measured: Option<(String, Type, f32)>,
    /// The second line last measured, and its width.
    measured_detail: Option<(String, f32)>,
    /// The label as it fits the button.
    shown: String,
    /// The second line as it fits the button.
    shown_detail: String,
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
        let detail_width = match (&self.detail, &state.measured_detail) {
            (None, _) => 0.0,
            (Some(detail), Some((measured, width))) if detail == measured => *width,
            (Some(detail), _) => {
                let width = font::measure(detail, DETAIL);
                state.measured_detail = Some((detail.clone(), width));
                width
            }
        };
        let around = 2.0 * self.padding + ARROW_GAP + ARROW;
        let size = limits.resolve(
            self.width,
            Length::Fixed(self.height),
            Size::new(label_width.max(detail_width) + around, self.height),
        );
        let room = (size.width - around).max(0.0);
        // A button sized to its label gets back a hair less room after
        // the sums; that much never needs an ellipsis.
        let fitted = |content: &str, style, width: f32| {
            if width <= room + 0.5 {
                content.to_owned()
            } else {
                fit(content, style, room)
            }
        };
        state.shown = fitted(&self.label, self.text, label_width);
        state.shown_detail = match &self.detail {
            Some(detail) => fitted(detail, DETAIL, detail_width),
            None => String::new(),
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
            match &self.open {
                Some((open, on_toggle)) => shell.publish(on_toggle(!open)),
                None => state.open = !state.open,
            }
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
        let open = self.is_open(state);
        // A quiet button is lit while its menu is open, or while selected.
        let lit = open || self.selected;
        let content = match (self.look, lit) {
            _ if self.placeholder => scheme.on_surface_variant,
            (Look::Quiet, true) => scheme.on_secondary_container,
            _ if self.value => scheme.on_surface,
            _ => scheme.on_surface_variant,
        };
        let border = match self.look {
            Look::Quiet => Border {
                radius: (bounds.height / 2.0).into(),
                ..Border::default()
            },
            Look::Outlined => {
                let (color, width) = if open {
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
            Look::Quiet if lit => scheme.secondary_container,
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

        // Text kept to the part of the button in view: a software renderer
        // can draw text from a scrolled area outside it.
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };
        if let Some(glyph) = self.glyph {
            let side = Height::ExtraSmall.icon();
            renderer.fill_text(
                text::Text {
                    align_x: text::Alignment::Center,
                    shaping: text::Shaping::Basic,
                    ..line(
                        glyph.codepoint().to_string(),
                        if lit { font::ICONS_FILLED } else { font::ICONS },
                        side,
                        side,
                        side,
                    )
                },
                Point::new(bounds.center_x(), bounds.center_y()),
                content,
                clip,
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
        // With a second line, the two lines are centred together. A lit
        // quiet button colors both alike; otherwise the label stands out
        // from the line under it.
        let (detail_size, detail_height) = DETAIL.metrics();
        let together = lit && self.look == Look::Quiet;
        let (label_y, label_color, detail_color) = match self.detail {
            Some(_) if together => (
                bounds.center_y() - (detail_height + LINE_GAP) / 2.0,
                content,
                content,
            ),
            Some(_) => (
                bounds.center_y() - (detail_height + LINE_GAP) / 2.0,
                scheme.on_surface,
                scheme.on_surface_variant,
            ),
            None => (bounds.center_y(), content, content),
        };
        // The label is fitted already; a box exactly its width would drop
        // its last letter, so the gap before the arrow is spare.
        renderer.fill_text(
            text::Text {
                align_x,
                ..line(
                    state.shown.clone(),
                    self.text.font(false),
                    size,
                    line_height,
                    room + ARROW_GAP,
                )
            },
            Point::new(label_x, label_y),
            label_color,
            clip,
        );
        if self.detail.is_some() {
            renderer.fill_text(
                text::Text {
                    align_x,
                    ..line(
                        state.shown_detail.clone(),
                        DETAIL.font(false),
                        detail_size,
                        detail_height,
                        room + ARROW_GAP,
                    )
                },
                Point::new(
                    label_x,
                    label_y + (line_height + detail_height) / 2.0 + LINE_GAP,
                ),
                detail_color,
                clip,
            );
        }
        // The arrow only when all of it is in view: a software renderer
        // draws a glyph the clip cuts whole.
        let arrow_left = if self.mirrored {
            arrow_x - ARROW
        } else {
            arrow_x
        };
        if arrow_left >= clip.x && arrow_left + ARROW <= clip.x + clip.width {
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
                clip,
            );
        }
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
        let on_toggle = match &self.open {
            Some((false, _)) => return None,
            Some((true, on_toggle)) => Some(on_toggle.clone()),
            None if !state.open => return None,
            None => None,
        };
        let bounds = layout.bounds();
        Some(overlay::Element::new(Box::new(Menu {
            content: &mut self.menu,
            tree: &mut children[0],
            open: &mut state.open,
            on_toggle,
            natural: self.natural,
            anchor: bounds + translation,
            mirrored: self.mirrored,
        })))
    }
}

/// The type style of a button's second line, and the gap above it.
const DETAIL: Type = Type::BodySmall;
const LINE_GAP: f32 = 2.0;

struct Menu<'a, 'b, Message> {
    content: &'a mut Element<'b, Pick<Message>>,
    tree: &'a mut Tree,
    open: &'a mut bool,
    /// Asks the app to close the menu, when the app keeps whether it is
    /// open.
    on_toggle: Option<OnToggle<Message>>,
    /// The width the widest entry needs.
    natural: f32,
    anchor: Rectangle,
    mirrored: bool,
}

impl<Message> Menu<'_, '_, Message> {
    fn close(&mut self, shell: &mut Shell<'_, Message>) {
        match &self.on_toggle {
            Some(on_toggle) => shell.publish(on_toggle(false)),
            None => *self.open = false,
        }
        shell.request_redraw();
    }
}

impl<Message: Clone> overlay::Overlay<Message, Theme, iced::Renderer> for Menu<'_, '_, Message> {
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        // As wide as the longest item and at least the button, within the
        // window; as tall as its items or the room on the roomier side.
        let width = self.natural.max(self.anchor.width).min(bounds.width).ceil();
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
                self.close(shell);
                return;
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => {
                shell.capture_event();
                self.close(shell);
                return;
            }
            _ => {}
        }
        // The rows' choices, each saying whether it closes the menu.
        let mut picked = Vec::new();
        let mut local = Shell::new(&mut picked);
        self.content.as_widget_mut().update(
            self.tree, event, layout, cursor, renderer, clipboard, &mut local, &bounds,
        );
        let closes = std::cell::Cell::new(false);
        shell.merge(local, |pick: Pick<Message>| {
            closes.set(closes.get() || pick.close);
            pick.message
        });
        if closes.get() {
            self.close(shell);
        }
        // Presses and scrolls stay in the menu; moves go on, so what lies
        // under it, such as the button's tooltip, sees the pointer leave.
        if let Event::Mouse(
            mouse::Event::ButtonPressed(_)
            | mouse::Event::ButtonReleased(_)
            | mouse::Event::WheelScrolled { .. },
        ) = event
            && cursor.is_over(bounds)
        {
            shell.capture_event();
        }
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'c>,
        renderer: &iced::Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, iced::Renderer>> {
        // A row's own overlay, such as a tooltip, sends nothing to close.
        self.content
            .as_widget_mut()
            .overlay(self.tree, layout, renderer, &layout.bounds(), Vector::ZERO)
            .map(|overlay| overlay.map(&|pick: Pick<Message>| pick.message))
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let interaction = self.content.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &layout.bounds(),
            renderer,
        );
        // Over a heading or the surface, the pointer is the menu's, not
        // what lies under it.
        if interaction == mouse::Interaction::None && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Idle
        } else {
            interaction
        }
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
        holding(pick(["Light", "Dark"], Some("Light"), |shade| shade).look(look))
    }

    /// A window with `dropdown` and a word beside it.
    fn holding(dropdown: Dropdown<&'static str>) -> iced_test::Simulator<'static, &'static str> {
        let content = iced::widget::row![dropdown, iced::widget::text("Aside")]
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

    #[test]
    fn a_choice_kept_open_leaves_the_menu_open() {
        let mut simulator = holding(menu(
            "Style",
            vec![
                Entry::heading("Width"),
                Entry::item("Thin", "thin").keep_open(),
                Entry::divider(),
                Entry::item("Reset", "reset"),
            ],
        ));
        simulator.click("Style").expect("the button");
        assert!(simulator.find("Width").is_ok(), "the heading");
        simulator.click("Thin").expect("the kept open item");
        assert!(simulator.find("Thin").is_ok(), "still open");
        simulator.click("Reset").expect("the closing item");
        assert!(simulator.find("Thin").is_err(), "closed");
        assert_eq!(
            simulator.into_messages().collect::<Vec<_>>(),
            ["thin", "reset"]
        );
    }

    #[test]
    fn the_app_can_keep_whether_the_menu_is_open() {
        let toggle = |open| if open { "open" } else { "close" };
        let entries = || vec![Entry::item("Thin", "thin"), Entry::note("Nothing more")];
        // Closed, a press asks to open it, and it waits for the app.
        let mut simulator = holding(menu("Style", entries()).open(false, toggle));
        simulator.click("Style").expect("the button");
        assert!(simulator.find("Thin").is_err(), "the app opens it");
        assert_eq!(simulator.into_messages().collect::<Vec<_>>(), ["open"]);
        // Open, a choice asks to close it.
        let mut simulator = holding(menu("Style", entries()).open(true, toggle));
        assert!(simulator.find("Nothing more").is_ok(), "the note");
        simulator.click("Thin").expect("the item");
        assert_eq!(
            simulator.into_messages().collect::<Vec<_>>(),
            ["thin", "close"]
        );
        // And so do Escape and the button.
        let mut simulator = holding(menu("Style", entries()).open(true, toggle));
        let _ = simulator.tap_key(keyboard::key::Named::Escape);
        simulator.click("Style").expect("the button");
        assert_eq!(
            simulator.into_messages().collect::<Vec<_>>(),
            ["close", "close"]
        );
    }

    #[test]
    fn swatches_send_their_colors() {
        let swatch = |color, message| Swatch {
            color,
            selected: false,
            message,
        };
        let mut simulator = holding(menu(
            "Color",
            vec![
                Entry::heading("Border"),
                Entry::swatches(
                    vec![swatch(Some(Color::BLACK), "black"), swatch(None, "none")],
                    6,
                )
                .keep_open(),
            ],
        ));
        simulator.click("Color").expect("the button");
        let heading = simulator.find("Border").expect("the heading").bounds();
        // The swatches start under the heading, at the menu's start less
        // their own padding.
        let top = heading.y + heading.height + 4.0 + 4.0;
        let start = heading.x - 16.0 + 12.0;
        for index in [0.0, 1.0] {
            simulator.point_at(Point::new(
                start + index * (SWATCH + SWATCH_GAP) + SWATCH / 2.0,
                top + 16.0,
            ));
            let _ = simulator.simulate(iced_test::simulator::click());
        }
        assert!(simulator.find("Border").is_ok(), "kept open");
        assert_eq!(
            simulator.into_messages().collect::<Vec<_>>(),
            ["black", "none"]
        );
    }

    #[test]
    fn the_menu_fits_its_widest_entry() {
        let swatches = (0..6)
            .map(|_| Swatch {
                color: None,
                selected: false,
                message: (),
            })
            .collect();
        let entries = vec![Entry::heading("A"), Entry::swatches(swatches, 4)];
        assert_eq!(
            natural_width(&entries),
            4.0 * SWATCH + 3.0 * SWATCH_GAP + 24.0
        );
        assert_eq!(natural_width::<()>(&[Entry::divider()]), 0.0);
    }
}
