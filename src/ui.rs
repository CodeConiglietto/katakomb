use flo_binding::{Binding, Bound};
use ggez::{
    graphics::{self, Align, DrawParam, Rect, Text},
    Context, GameResult,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        assert!(width >= 0.0);
        assert!(height >= 0.0);
        Self { width, height }
    }

    pub fn shrink(&self, size: Size) -> Self {
        Self {
            width: (self.width - size.width).max(0.0),
            height: (self.height - size.height).max(0.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxConstraints {
    pub min: Size,
    pub max: Size,
}

impl BoxConstraints {
    pub fn new(min: Size, max: Size) -> Self {
        assert!(min.width <= max.width);
        assert!(min.height <= max.height);
        Self { min, max }
    }

    pub fn shrink(&self, size: Size) -> Self {
        Self {
            min: self.min.shrink(size),
            max: self.max.shrink(size),
        }
    }
}

pub trait Element {
    fn layout(&mut self, ctx: &mut Context, constraints: BoxConstraints) -> Size;
    fn draw(&mut self, ctx: &mut Context, rect: Rect) -> GameResult<()>;
}

pub struct TextBox {
    text: Binding<String>,
    gg_text: Option<Text>,
}

impl TextBox {
    pub fn new(text: Binding<String>) -> Self {
        Self {
            text,
            gg_text: None,
        }
    }
}

impl Element for TextBox {
    fn layout(&mut self, ctx: &mut Context, constraints: BoxConstraints) -> Size {
        let mut gg_text = Text::new(self.text.get().as_str());
        gg_text.set_bounds([constraints.max.width, constraints.max.height], Align::Left);

        let size = gg_text.dimensions(ctx);
        self.gg_text = Some(gg_text);
        Size::new(size.0 as f32, size.1 as f32)
    }

    fn draw(&mut self, ctx: &mut Context, rect: Rect) -> GameResult<()> {
        let gg_text = self.gg_text.as_ref().unwrap();
        let size = gg_text.dimensions(ctx);

        graphics::draw(
            ctx,
            gg_text,
            DrawParam::default()
                .dest([rect.x, rect.y])
                .scale([rect.w / size.0 as f32, rect.h / size.1 as f32]),
        )?;

        Ok(())
    }
}

pub struct List<T> {
    pub elements: Vec<ListElement<T>>,
}

impl<T: Element> List<T> {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    pub fn from_vec(elements: Vec<ListElement<T>>) -> Self {
        Self { elements }
    }
}

impl<T: Element> Element for List<T> {
    fn layout(&mut self, ctx: &mut Context, constraints: BoxConstraints) -> Size {
        let mut size = Size::new(0.0, 0.0);

        for element in self.elements.iter_mut() {
            let element_constraint = constraints.shrink(Size::new(0.0, size.height));

            if element_constraint.max.height == 0.0 {
                break;
            }

            let element_size = element.element.layout(ctx, element_constraint);

            size.width = size.width.max(element_size.width);
            size.height += element_size.height;

            element.size = Some(element_size);
        }

        size
    }

    fn draw(&mut self, ctx: &mut Context, rect: Rect) -> GameResult<()> {
        let mut y: f32 = 0.0;

        for element in self.elements.iter_mut() {
            if let Some(size) = element.size {
                element
                    .element
                    .draw(ctx, Rect::new(rect.x, rect.y + y, size.width, size.height))?;

                y += size.height;
            }
        }

        Ok(())
    }
}

pub struct ListElement<T> {
    pub element: T,
    size: Option<Size>,
}

impl<T: Element> ListElement<T> {
    pub fn new(element: T) -> Self {
        Self {
            element,
            size: None,
        }
    }
}

pub struct Padding<T> {
    inner: T,
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}

impl<T: Element> Padding<T> {
    pub fn new(inner: T, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        assert!(top >= 0.0);
        assert!(right >= 0.0);
        assert!(bottom >= 0.0);
        assert!(left >= 0.0);

        Self {
            inner,
            top,
            right,
            bottom,
            left,
        }
    }
}

impl<T: Element> Padding<T> {
    fn layout(&mut self, ctx: &mut Context, constraints: BoxConstraints) -> Size {
        self.inner.layout(
            ctx,
            constraints.shrink(Size::new(self.right + self.left, self.top + self.bottom)),
        )
    }

    fn draw(&mut self, ctx: &mut Context, rect: Rect) -> GameResult<()> {
        self.inner.draw(
            ctx,
            Rect::new(
                rect.x + self.left,
                rect.y + self.top,
                (rect.w - (self.left + self.right)).max(0.0),
                (rect.h - (self.top + self.bottom)).max(0.0),
            ),
        )
    }
}
