use flo_binding::{bind, Binding, Bound, MutableBound};
use ggez::{
    graphics::{self, Align, DrawParam, Rect, Text},
    Context, GameResult,
};

use crate::rendering::font::KataFontBatch;

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

    pub fn with_inf_height(&self) -> Self {
        Self {
            min: Size::new(self.min.width, f32::INFINITY),
            max: Size::new(self.max.width, f32::INFINITY),
        }
    }
}

pub trait Element {
    fn layout(&mut self, ctx: &mut Context, constraints: BoxConstraints) -> Size;
    fn draw(&mut self, ctx: &mut Context, fb: &mut KataFontBatch, rect: Rect) -> GameResult<()>;
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

    fn draw(&mut self, ctx: &mut Context, fb: &mut KataFontBatch, rect: Rect) -> GameResult<()> {
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
    pub scrollbar: ScrollBar,
    pub scrollbar_size: Option<Size>,
    pub scroll_offset: Binding<usize>,
}

impl<T: Element> List<T> {
    pub fn new() -> Self {
        Self::from_vec(Vec::new())
    }

    pub fn from_vec(elements: Vec<ListElement<T>>) -> Self {
        let scroll_offset = bind(0);

        Self {
            elements: Vec::new(),
            scrollbar_size: None,
            scrollbar: ScrollBar::new(scroll_offset.clone(), 0, ScrollBarDirection::Vertical),
            scroll_offset,
        }
    }
}

impl<T: Element> Element for List<T> {
    fn layout(&mut self, ctx: &mut Context, constraints: BoxConstraints) -> Size {
        let elements_height =
            layout_list_elements(&mut self.elements, ctx, constraints.with_inf_height());

        let (scrollbar_size, scroll_offset) = if elements_height > constraints.max.height {
            // If the elements overflow, display the scrollbar
            let scrollbar_size = self.scrollbar.layout(ctx, constraints);

            layout_list_elements(
                &mut self.elements,
                ctx,
                constraints.shrink(scrollbar_size).with_inf_height(),
            );

            // Move up the list if we have room for elements from our current scroll offset
            let mut fill_height = 0.0;
            let mut scroll_offset = self.elements.len() - 1;
            for element in self.elements.iter().rev() {
                fill_height += element.size.unwrap().height;

                if fill_height > constraints.max.height {
                    break;
                }

                scroll_offset -= 1;
            }

            (Some(scrollbar_size), scroll_offset)
        } else {
            (None, 0)
        };

        self.scrollbar_size = scrollbar_size;
        self.scroll_offset.set(scroll_offset);
        constraints.max
    }

    fn draw(&mut self, ctx: &mut Context, fb: &mut KataFontBatch, rect: Rect) -> GameResult<()> {
        if let Some(scrollbar_size) = self.scrollbar_size {
            self.scrollbar.draw(
                ctx,
                fb,
                Rect::new(
                    rect.right() - scrollbar_size.width,
                    rect.y,
                    scrollbar_size.width,
                    scrollbar_size.height,
                ),
            )?;
        }

        let mut y: f32 = 0.0;

        for element in self.elements.iter_mut().skip(self.scroll_offset.get()) {
            if let Some(size) = element.size {
                let bottom = y + size.height;

                if bottom > rect.bottom() {
                    break;
                }

                element.element.draw(
                    ctx,
                    fb,
                    Rect::new(rect.x, rect.y + y, size.width, size.height),
                )?;

                y = bottom;
            }
        }

        Ok(())
    }
}

fn layout_list_elements<T: Element>(
    elements: &mut [ListElement<T>],
    ctx: &mut Context,
    constraints: BoxConstraints,
) -> f32 {
    let mut size = Size::new(0.0, 0.0);
    let mut total_height = 0.0;

    for element in elements {
        let element_size = element.element.layout(ctx, constraints);

        size.width = size.width.max(element_size.width);
        total_height += element_size.height;

        element.size = Some(element_size);
    }

    total_height
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

    fn draw(&mut self, ctx: &mut Context, fb: &mut KataFontBatch, rect: Rect) -> GameResult<()> {
        self.inner.draw(
            ctx,
            fb,
            Rect::new(
                rect.x + self.left,
                rect.y + self.top,
                (rect.w - (self.left + self.right)).max(0.0),
                (rect.h - (self.top + self.bottom)).max(0.0),
            ),
        )
    }
}

pub struct ScrollBar {
    pub scroll_pos: Binding<usize>,
    pub scroll_max: usize,
    pub direction: ScrollBarDirection,
}

pub enum ScrollBarDirection {
    Horizontal,
    Vertical,
}

impl ScrollBar {
    pub fn new(
        scroll_pos: Binding<usize>,
        scroll_max: usize,
        direction: ScrollBarDirection,
    ) -> Self {
        Self {
            scroll_pos,
            scroll_max,
            direction,
        }
    }
}

const PREFERRED_SCROLLBAR_THICKNESS: f32 = 10.0;

impl Element for ScrollBar {
    fn layout(&mut self, _ctx: &mut Context, constraints: BoxConstraints) -> Size {
        match self.direction {
            ScrollBarDirection::Horizontal => Size::new(
                constraints.max.width,
                PREFERRED_SCROLLBAR_THICKNESS
                    .max(constraints.min.height)
                    .min(constraints.max.height),
            ),
            ScrollBarDirection::Vertical => Size::new(
                PREFERRED_SCROLLBAR_THICKNESS
                    .max(constraints.min.width)
                    .min(constraints.max.width),
                constraints.max.height,
            ),
        }
    }

    fn draw(&mut self, ctx: &mut Context, fb: &mut KataFontBatch, rect: Rect) -> GameResult<()> {
        Ok(())
    }
}
