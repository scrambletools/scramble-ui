//! A drag handle for resizing a panel: upright on a side panel's edge, or
//! lying along a bottom panel's, shown as an M3 drag handle while hovered
//! or dragged.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::{Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::mouse::{self, Cursor};
use iced::{Background, Element, Event, Length, Rectangle, Size, Theme, border};

use super::{Scheme, shape};

/// Width of the grab area, or its height when it lies along an edge.
pub const HANDLE_WIDTH: f32 = 8.0;
const PILL: Size = Size::new(4.0, 48.0);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Drag {
    Started,
    /// Distance from where the drag started, positive rightwards, or
    /// downwards for a handle that resizes a height.
    Moved(f32),
    Ended,
}

/// What a handle resizes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Axis {
    /// A side panel's width: the handle stands along its edge.
    #[default]
    Width,
    /// A bottom or top panel's height: the handle lies along its edge.
    Height,
}

pub struct Handle<'a, Message> {
    on_drag: Box<dyn Fn(Drag) -> Message + 'a>,
    axis: Axis,
}

pub fn handle<'a, Message>(on_drag: impl Fn(Drag) -> Message + 'a) -> Handle<'a, Message> {
    Handle {
        on_drag: Box::new(on_drag),
        axis: Axis::Width,
    }
}

impl<Message> Handle<'_, Message> {
    /// Resizes a panel's height instead, lying along its top or bottom.
    pub fn height(mut self) -> Self {
        self.axis = Axis::Height;
        self
    }

    /// The handle's width and height.
    fn length(&self) -> Size<Length> {
        match self.axis {
            Axis::Width => Size::new(Length::Fixed(HANDLE_WIDTH), Length::Fill),
            Axis::Height => Size::new(Length::Fill, Length::Fixed(HANDLE_WIDTH)),
        }
    }

    /// Where along the axis the cursor is.
    fn along(&self, position: iced::Point) -> f32 {
        match self.axis {
            Axis::Width => position.x,
            Axis::Height => position.y,
        }
    }
}

#[derive(Default)]
struct State {
    /// Cursor x where the drag started, or y for a height.
    origin: Option<f32>,
    hovered: bool,
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Handle<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        self.length()
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let length = self.length();
        layout::Node::new(limits.resolve(length.width, length.height, Size::ZERO))
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
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if hovered => {
                if let Some(position) = cursor.position() {
                    state.origin = Some(self.along(position));
                    shell.publish((self.on_drag)(Drag::Started));
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(origin) = state.origin {
                    let moved = self.along(*position) - origin;
                    shell.publish((self.on_drag)(Drag::Moved(moved)));
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.origin.take().is_some() =>
            {
                shell.publish((self.on_drag)(Drag::Ended));
                shell.capture_event();
                shell.request_redraw();
            }
            _ => {}
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
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let state = tree.state.downcast_ref::<State>();
        let dragging = state.origin.is_some();
        if !state.hovered && !dragging {
            return;
        }
        let scheme = Scheme::of(theme);
        let bounds = layout.bounds();
        let color = if dragging {
            scheme.on_surface
        } else {
            scheme.outline
        };
        let pill = match self.axis {
            Axis::Width => PILL,
            Axis::Height => Size::new(PILL.height, PILL.width),
        };
        renderer.fill_quad(
            Quad {
                bounds: Rectangle::new(
                    iced::Point::new(
                        bounds.center_x() - pill.width / 2.0,
                        bounds.center_y() - pill.height / 2.0,
                    ),
                    pill,
                ),
                border: border::rounded(shape::FULL),
                ..Quad::default()
            },
            Background::Color(color),
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();
        if state.origin.is_some() || cursor.is_over(layout.bounds()) {
            match self.axis {
                Axis::Width => mouse::Interaction::ResizingHorizontally,
                Axis::Height => mouse::Interaction::ResizingVertically,
            }
        } else {
            mouse::Interaction::default()
        }
    }
}

impl<'a, Message: 'a> From<Handle<'a, Message>> for Element<'a, Message> {
    fn from(handle: Handle<'a, Message>) -> Self {
        Element::new(handle)
    }
}

/// A panel width, or height, being resized between `min` and `max`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Width {
    pub value: f32,
    min: f32,
    max: f32,
    /// Width when the current drag started.
    start: Option<f32>,
}

impl Width {
    pub const fn new(value: f32, min: f32, max: f32) -> Self {
        Self {
            value,
            min,
            max,
            start: None,
        }
    }

    /// Applies a drag of the handle on the panel's right edge.
    pub fn drag(&mut self, drag: Drag) {
        match drag {
            Drag::Started => self.start = Some(self.value),
            Drag::Moved(offset) => {
                if let Some(start) = self.start {
                    self.value = (start + offset).clamp(self.min, self.max);
                }
            }
            Drag::Ended => self.start = None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drags_within_limits() {
        let mut width = Width::new(264.0, 200.0, 480.0);
        width.drag(Drag::Started);
        width.drag(Drag::Moved(40.0));
        assert_eq!(width.value, 304.0);
        width.drag(Drag::Moved(-500.0));
        assert_eq!(width.value, 200.0);
        // Coming back from past the limit follows the cursor again.
        width.drag(Drag::Moved(10.0));
        assert_eq!(width.value, 274.0);
        width.drag(Drag::Moved(900.0));
        assert_eq!(width.value, 480.0);
        width.drag(Drag::Ended);
        width.drag(Drag::Moved(-100.0));
        assert_eq!(width.value, 480.0, "moves outside a drag are ignored");
    }

    /// The drags a handle sends as the cursor presses at `from`, moves to
    /// each of `path` and lets go, in a window with the handle lying
    /// across its middle or standing at its left.
    fn drags(axis: Axis, from: iced::Point, path: &[iced::Point]) -> Vec<Drag> {
        use iced::widget::{column, container};
        let handle = match axis {
            Axis::Width => handle(|drag| drag),
            Axis::Height => handle(|drag| drag).height(),
        };
        let content: Element<'_, Drag> = match axis {
            Axis::Width => iced::widget::row![handle, container("").width(Length::Fill)].into(),
            Axis::Height => column![
                container("").height(96.0),
                handle,
                container("").height(Length::Fill),
            ]
            .into(),
        };
        let mut simulator = iced_test::Simulator::with_size(
            iced::Settings::default(),
            Size::new(400.0, 300.0),
            content,
        );
        simulator.point_at(from);
        let mut events = vec![Event::Mouse(mouse::Event::ButtonPressed(
            mouse::Button::Left,
        ))];
        events.extend(
            path.iter()
                .map(|&position| Event::Mouse(mouse::Event::CursorMoved { position })),
        );
        events.push(Event::Mouse(mouse::Event::ButtonReleased(
            mouse::Button::Left,
        )));
        let _ = simulator.simulate(events);
        simulator.into_messages().collect()
    }

    #[test]
    fn a_handle_measures_along_what_it_resizes() {
        use iced::Point;
        // Lying across at y 96 to 104: only the cursor's height counts.
        let height = drags(
            Axis::Height,
            Point::new(200.0, 100.0),
            &[Point::new(260.0, 70.0), Point::new(20.0, 130.0)],
        );
        assert_eq!(
            height,
            [
                Drag::Started,
                Drag::Moved(-30.0),
                Drag::Moved(30.0),
                Drag::Ended
            ]
        );
        // Standing at x 0 to 8: only its width counts.
        let width = drags(
            Axis::Width,
            Point::new(4.0, 150.0),
            &[Point::new(44.0, 10.0)],
        );
        assert_eq!(width, [Drag::Started, Drag::Moved(40.0), Drag::Ended]);
        // A press beside the handle starts nothing.
        let beside = drags(
            Axis::Height,
            Point::new(200.0, 140.0),
            &[Point::new(200.0, 40.0)],
        );
        assert!(beside.is_empty(), "{beside:?}");
    }
}
