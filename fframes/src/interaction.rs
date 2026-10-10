//! Viewer input for videos that run live (the browser runtime, a player window) instead of being
//! rendered to a file.
//!
//! A live runtime owns one [`Input`], updates it from pointer, keyboard and form events, and passes
//! it to [`Video::render_frame`](crate::Video::render_frame) through
//! [`FFramesContext::input`](crate::FFramesContext::input). File renders pass no input, so every
//! query answers as if nobody touched the video and the same code renders a video file unchanged.
//!
//! Elements opt into interaction with a `data-action` attribute. Clicking one (or pressing Enter or
//! Space while it has focus) triggers that action; hovering it makes it the hovered action.
//!
//! ```ignore
//! use fframes::{FFramesContext, Frame, Svgr, animation::Easing};
//!
//! fn button<'a>(frame: &Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
//!     let input = ctx.input();
//!     // Toggle on every click, and spring the scale from the moment of the last click.
//!     let on = input.count("toggle") % 2 == 1;
//!     let pop = fframes::timeline!(at 0.0, animate 0.8_f32 => 1.0, Easing::Spring { mass: 1.0, stiffness: 300.0, damping: 16.0 });
//!     let scale = input.seconds_since("toggle").map_or(1.0, |s| frame.animate_at(&pop, s));
//!     let fill = if on { "#fb6a22" } else if input.is_hovered("toggle") { "#334155" } else { "#0f172a" };
//!     fframes::svgr!(
//!         <g transform={format!("translate(960 540) scale({scale})")}>
//!             <rect data-action="toggle" role="button" tabindex="0" aria-label="Toggle"
//!                   x="-120" y="-60" width="240" height="120" rx="60" fill={fill} />
//!         </g>
//!     )
//! }
//! ```

use std::collections::HashMap;

/// Something the viewer did, delivered to [`Video::on_event`](crate::Video::on_event) after the
/// [`Input`] is updated.
#[derive(Debug, Clone, PartialEq)]
pub enum InteractionEvent {
    /// An element with `data-action="<name>"` was clicked or activated with the keyboard.
    Action(String),
    /// A bound value changed, for example a text field with `data-fframes-value="<key>"`.
    Value { key: String, value: String },
    /// The pointer was pressed at a point in the video's coordinates.
    PointerDown { x: f32, y: f32 },
    /// The pointer was released at a point in the video's coordinates.
    PointerUp { x: f32, y: f32 },
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct ActionState {
    count: u32,
    last_time: f32,
}

/// What the viewer is doing right now. Read it with
/// [`FFramesContext::input`](crate::FFramesContext::input).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Input {
    /// Seconds since the live session started. Unlike the frame time it never loops or resets.
    pub time: f32,
    /// Pointer position in the video's coordinates (the `viewBox` units), `None` when the pointer
    /// is outside the video.
    pub pointer: Option<(f32, f32)>,
    /// Whether the primary pointer button is held.
    pub pointer_down: bool,
    hovered: Option<String>,
    values: HashMap<String, String>,
    actions: HashMap<String, ActionState>,
}

impl Input {
    /// The input of a file render: no pointer, nothing hovered, no actions, no values.
    pub fn none() -> &'static Input {
        static NONE: std::sync::OnceLock<Input> = std::sync::OnceLock::new();
        NONE.get_or_init(Input::default)
    }

    /// How many times the action was triggered. `count % 2 == 1` makes a toggle.
    pub fn count(&self, action: &str) -> u32 {
        self.actions.get(action).map_or(0, |state| state.count)
    }

    /// Whether the action was triggered at least once.
    pub fn triggered(&self, action: &str) -> bool {
        self.count(action) > 0
    }

    /// Seconds since the action was last triggered, `None` if it never was. Feed it to
    /// [`Frame::animate_at`](crate::Frame::animate_at) to play an animation from the click.
    pub fn seconds_since(&self, action: &str) -> Option<f32> {
        self.actions
            .get(action)
            .map(|state| (self.time - state.last_time).max(0.0))
    }

    /// The `data-action` of the element under the pointer.
    pub fn hovered(&self) -> Option<&str> {
        self.hovered.as_deref()
    }

    /// Whether the element with this `data-action` is under the pointer.
    pub fn is_hovered(&self, action: &str) -> bool {
        self.hovered.as_deref() == Some(action)
    }

    /// A bound value, for example the text of `<input data-fframes-value="name">` on the page.
    pub fn value(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    /// Records a triggered action at the current [`Input::time`] and returns the event for it.
    pub fn trigger(&mut self, action: impl Into<String>) -> InteractionEvent {
        let action = action.into();
        let state = self.actions.entry(action.clone()).or_default();
        state.count += 1;
        state.last_time = self.time;
        InteractionEvent::Action(action)
    }

    /// Stores a bound value and returns the event for it.
    pub fn set_value(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> InteractionEvent {
        let (key, value) = (key.into(), value.into());
        self.values.insert(key.clone(), value.clone());
        InteractionEvent::Value { key, value }
    }

    /// Sets the hovered action, `None` when the pointer is over nothing interactive.
    pub fn set_hovered(&mut self, action: Option<String>) {
        self.hovered = action;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untouched_input_answers_like_a_file_render() {
        let input = Input::none();
        assert_eq!(input.count("a"), 0);
        assert!(!input.triggered("a"));
        assert_eq!(input.seconds_since("a"), None);
        assert!(!input.is_hovered("a"));
        assert_eq!(input.value("name"), None);
        assert_eq!(input.pointer, None);
    }

    #[test]
    fn actions_count_and_time_from_the_last_trigger() {
        let mut input = Input {
            time: 1.0,
            ..Default::default()
        };
        assert_eq!(input.trigger("go"), InteractionEvent::Action("go".into()));
        input.time = 3.0;
        input.trigger("go");
        input.time = 3.5;
        assert_eq!(input.count("go"), 2);
        assert_eq!(input.seconds_since("go"), Some(0.5));
    }

    #[test]
    fn values_and_hover() {
        let mut input = Input::default();
        input.set_value("name", "Ada");
        input.set_hovered(Some("go".into()));
        assert_eq!(input.value("name"), Some("Ada"));
        assert!(input.is_hovered("go"));
        assert_eq!(input.hovered(), Some("go"));
    }
}
