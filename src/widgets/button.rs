use std::panic::Location;

use crate::{BoxConstraints, VisualEq, context::{EventCtx, LayoutCtx, LifeCycleCtx, PaintCtx, UpdateCtx}, cx::Cx, event::{Event, LifeCycle}, kurbo::Size, render::{Properties, RenderObject}, tree::Children};
use druid::{Affine, Insets, LinearGradient, MouseButton, RenderContext, UnitPoint};
use style::{Style, StyleSheet};

// the minimum padding added to a button.
// NOTE: these values are chosen to match the existing look of TextBox; these
// should be reevaluated at some point.
const LABEL_INSETS: Insets = Insets::uniform_xy(8., 2.);

#[derive(Default, PartialEq)]
pub struct Button {
    label: String,
    disabled: bool,
    style: Option<Box<dyn StyleSheet>>,
}

impl Properties for Button {
    type Object = ButtonObject;
    type Action = bool;
}

impl Button {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[track_caller]
    pub fn build(self, cx: &mut Cx) -> bool {
        let caller = Location::caller().into();
        cx.render_object::<ButtonObject>(caller, self).is_some()
    }
}

pub enum ButtonAction {
    Clicked,
}

#[derive(Default)]
pub struct ButtonObject {
    props: Button,
    label_size: Size,
}

impl ButtonObject {
    fn style(&self, hovered: bool, pressed: bool) -> Style {
        let sheet = match self.props.style {
            Some(ref sheet) => sheet.as_ref(),
            None => &style::Default,
        };
        let disabled = self.props.disabled;
        match (disabled, hovered, pressed) {
            (true, _, _) => sheet.disabled(),
            (false, true, true) => sheet.pressed(),
            (false, true, false) => sheet.hovered(),
            (false, false, _) => sheet.enabled(),
        }
    }
}

impl RenderObject for ButtonObject {
    type Props = Button;
    type Action = ButtonAction;

    fn update(&mut self, ctx: &mut UpdateCtx, props: Button) {
        if self.props != props {
            ctx.request_layout();
            self.props = props;
        }
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, children: &mut Children) {
        match event {
            Event::MouseDown(mouse_event) => {
                if mouse_event.button == MouseButton::Left {
                    ctx.set_active(true);
                    ctx.request_paint();
                }
            }
            Event::MouseUp(mouse_event) => {
                if ctx.is_active() && mouse_event.button == MouseButton::Left {
                    ctx.set_active(false);
                    if ctx.is_hot() {
                        //ctx.submit_action(ButtonAction::Clicked);
                        ctx.set_handled();
                    }
                    ctx.request_paint();
                }
            }
            _ => {}
        }

        for child in children {
            child.event(ctx, event);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        children: &mut Children,
    ) -> Size {
        bc.debug_check("Button");
        let style = self.style(ctx.is_hot(), ctx.is_active());
        let padding = Size::new(LABEL_INSETS.x_value(), LABEL_INSETS.y_value());
        let label_bc = bc.shrink(padding).loosen();
        self.label_size = children[0].layout(ctx, &label_bc);
        // HACK: to make sure we look okay at default sizes when beside a textbox,
        // we make sure we will have at least the same height as the default textbox.
        let min_height = style.min_height;
        let baseline = children[0].baseline_offset();
        ctx.set_baseline_offset(baseline + LABEL_INSETS.y1);

        bc.constrain(Size::new(
            self.label_size.width + padding.width,
            (self.label_size.height + padding.height).max(min_height),
        ))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, children: &mut Children) {
        let is_active = ctx.is_active();
        let is_hot = ctx.is_hot();
        let size = ctx.size();
        let style = self.style(ctx.is_hot(), ctx.is_active());
        let stroke_width = style.border_width;

        let rounded_rect = size
            .to_rect()
            .inset(-stroke_width / 2.0)
            .to_rounded_rect(style.border_radius);

        #[allow(clippy::infallible_destructuring_match)]
        let bg = match style.background {
            style::Background::Color(color) => color,
        };

        let border_color = style.border_color;

        ctx.stroke(rounded_rect, &border_color, stroke_width);

        ctx.fill(rounded_rect, &bg);

        let label_offset = (size.to_vec2() - self.label_size.to_vec2()) / 2.0;

        ctx.with_save(|ctx| {
            ctx.transform(Affine::translate(label_offset));
            children[0].paint(ctx);
        });
    }
}

pub mod style {
    use std::any::Any;

    use druid::{Color, Vec2};

    const TRANSPARENT: Color = Color::rgba8(0, 0, 0, 0);

    /// The background of some element.
    #[derive(Debug, Clone, PartialEq)]
    pub enum Background {
        /// A solid color
        Color(Color),
        // TODO: Add gradient and image variants
    }

    impl From<Color> for Background {
        fn from(color: Color) -> Self {
            Background::Color(color)
        }
    }

    /// The appearance of a button.
    #[derive(Debug, Clone)]
    pub struct Style {
        pub min_height: f64,
        pub border_width: f64,
        pub border_radius: f64,
        pub border_color: Color,
        pub background: Background,

        pub shadow_offset: Vec2,
        pub text_color: Color,
    }

    impl std::default::Default for Style {
        fn default() -> Self {
            Self {
                min_height: 0.0,
                shadow_offset: Vec2::default(),
                background: Background::Color(TRANSPARENT),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: TRANSPARENT,
                text_color: Color::BLACK,
            }
        }
    }

    impl PartialEq for Box<dyn StyleSheet> {
        fn eq(&self, other: &Self) -> bool {
            self.as_ref().eq(other.as_ref())
        }
    }

    /// A set of rules that dictate the style of a button.
    pub trait StyleSheet : Any {
        fn eq(&self, other: &dyn StyleSheet) -> bool;

        fn enabled(&self) -> Style;

        fn hovered(&self) -> Style {
            let active = self.enabled();

            Style {
                shadow_offset: active.shadow_offset + Vec2::new(0.0, 1.0),
                ..active
            }
        }

        fn pressed(&self) -> Style {
            Style {
                shadow_offset: Vec2::default(),
                ..self.enabled()
            }
        }

        fn disabled(&self) -> Style {
            let active = self.enabled();

            Style {
                shadow_offset: Vec2::default(),
                background: match active.background {
                    Background::Color(color) => Background::Color(color.with_alpha(0.5)),
                },
                text_color: active.text_color.with_alpha(0.5),
                ..active
            }
        }
    }

    #[derive(Debug, PartialEq)]
    pub struct Default;

    impl StyleSheet for Default {
        fn enabled(&self) -> Style {
            Style {
                min_height: 24.0,
                shadow_offset: Vec2::new(0.0, 0.0),
                background: Background::Color(Color::rgb(0.87, 0.87, 0.87)),
                border_radius: 2.0,
                border_width: 1.0,
                border_color: Color::rgb(0.7, 0.7, 0.7),
                text_color: Color::BLACK,
            }
        }

        fn eq(&self, other: &dyn StyleSheet) -> bool {
            Any::type_id(other) == std::any::TypeId::of::<Self>()
        }
    }

    impl std::default::Default for Box<dyn StyleSheet> {
        fn default() -> Self {
            Box::new(Default)
        }
    }

    impl<T> From<T> for Box<dyn StyleSheet>
    where
        T: 'static + StyleSheet,
    {
        fn from(style: T) -> Self {
            Box::new(style)
        }
    }
}
