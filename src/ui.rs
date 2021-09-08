use std::{
    cmp::Ordering,
    ops::{Deref, DerefMut, Index, IndexMut, Range},
};

use flo_binding::{bind, Binding, Bound, MutableBound};
use ggez::{
    input::mouse::{self, MouseButton},
    mint, Context,
};
use log::trace;

use crate::{
    geometry::rect::IRect,
    rendering::{
        color::{self, Color},
        font::KataFontBatch,
        voxel::Voxel2,
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const ZERO: Self = Self {
        width: 0,
        height: 0,
    };

    pub fn shrink(&self, size: Size) -> Self {
        Self {
            width: self.width.saturating_sub(size.width),
            height: self.height.saturating_sub(size.height),
        }
    }

    pub fn dir(&self, direction: LayoutDirection) -> &u32 {
        match direction {
            LayoutDirection::Horizontal => &self.width,
            LayoutDirection::Vertical => &self.height,
        }
    }

    pub fn dir_mut(&mut self, direction: LayoutDirection) -> &mut u32 {
        match direction {
            LayoutDirection::Horizontal => &mut self.width,
            LayoutDirection::Vertical => &mut self.height,
        }
    }

    pub fn with_dir(self, direction: LayoutDirection, new_size: u32) -> Self {
        let mut it = self;
        it[direction] = new_size;
        it
    }

    pub fn min(self, rhs: Self) -> Self {
        Self {
            width: self.width.min(rhs.width),
            height: self.height.min(rhs.height),
        }
    }

    pub fn max(self, rhs: Self) -> Self {
        Self {
            width: self.width.max(rhs.width),
            height: self.height.max(rhs.height),
        }
    }
}

impl Default for Size {
    fn default() -> Self {
        Self::ZERO
    }
}

pub trait PointExt {
    fn dir(&self, direction: LayoutDirection) -> &u32;
}

impl PointExt for mint::Point2<u32> {
    fn dir(&self, direction: LayoutDirection) -> &u32 {
        match direction {
            LayoutDirection::Horizontal => &self.x,
            LayoutDirection::Vertical => &self.y,
        }
    }
}

impl Index<LayoutDirection> for Size {
    type Output = u32;

    fn index(&self, index: LayoutDirection) -> &Self::Output {
        self.dir(index)
    }
}

impl IndexMut<LayoutDirection> for Size {
    fn index_mut(&mut self, index: LayoutDirection) -> &mut Self::Output {
        self.dir_mut(index)
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

    pub fn exact(size: Size) -> Self {
        Self::new(size, size)
    }

    pub fn shrink(&self, size: Size) -> Self {
        Self {
            min: self.min.shrink(size),
            max: self.max.shrink(size),
        }
    }

    pub fn with_inf_height(&self) -> Self {
        Self {
            min: Size::new(self.min.width, u32::max_value()),
            max: Size::new(self.max.width, u32::max_value()),
        }
    }

    pub fn constrain(&self, size: Size) -> Size {
        size.min(self.max).max(self.min)
    }
}

pub struct UiContext {
    pub relayout: bool,
    pub batch: KataFontBatch,
}

impl UiContext {
    pub fn new(batch: KataFontBatch) -> Self {
        Self {
            relayout: true,
            batch,
        }
    }

    pub fn mouse_pos(&self, ctx: &Context) -> mint::Point2<u32> {
        let p = mouse::position(ctx);
        mint::Point2::from([
            (p.x / self.batch.tile_width()) as u32,
            (p.y / self.batch.tile_height()) as u32,
        ])
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
    Mouse {
        pos: mint::Point2<u32>,
        e: MouseEvent,
    },

    Draw,
}

impl Event {
    pub fn cull(self, bounds: IRect) -> Option<Self> {
        let keep = match self {
            Event::Mouse { pos, e } => match e {
                MouseEvent::ButtonDrag { start_pos, .. } => bounds.contains(start_pos),
                _ => bounds.contains(pos),
            },

            Event::Draw => true,
        };

        if keep {
            Some(self)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum MouseEvent {
    ButtonDown {
        button: MouseButton,
    },
    ButtonUp {
        button: MouseButton,
    },
    ButtonDrag {
        button: MouseButton,
        start_pos: mint::Point2<u32>,
    },

    WheelUp,
    WheelDown,
}

pub struct Continue;
pub struct Stop;
pub type EventResult = Result<Continue, Stop>;

pub trait Element {
    fn layout(&mut self, constraints: BoxConstraints) -> Size;
    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult;
}

pub trait ElementExt: Element + Sized {
    fn with_events<F>(self, handler: F) -> WithEvents<Self, F>
    where
        F: FnMut(&mut Self, &mut UiContext, Event, IRect) -> EventResult;
}

impl<T: Element + Sized> ElementExt for T {
    fn with_events<F>(self, handler: F) -> WithEvents<Self, F>
    where
        F: FnMut(&mut Self, &mut UiContext, Event, IRect) -> EventResult,
    {
        WithEvents {
            element: self,
            handler,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

impl LayoutDirection {
    pub fn other(self) -> Self {
        match self {
            LayoutDirection::Horizontal => LayoutDirection::Vertical,
            LayoutDirection::Vertical => LayoutDirection::Horizontal,
        }
    }
}

pub struct List {
    pub elements: Vec<ListElement>,
    pub scrollbar: ScrollBar,
    pub scrollbar_size: Option<Size>,
}

impl List {
    pub fn new() -> Self {
        Self::from_vec(Vec::new())
    }

    pub fn from_vec(elements: Vec<ListElement>) -> Self {
        Self {
            elements,
            scrollbar_size: None,
            scrollbar: ScrollBar::new(bind(0), bind(0), LayoutDirection::Vertical),
        }
    }
}

impl Element for List {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        trace!("List relayout");

        let elements_size = layout_list_elements(
            &mut self.elements,
            BoxConstraints::new(
                Size::ZERO,
                Size::new(constraints.max.width, u32::max_value()),
            ),
        );

        let (width, scrollbar_size, scroll_pos, scroll_max) =
            if elements_size.height > constraints.max.height {
                trace!("List overflow");
                // If the elements overflow, display the scrollbar
                let scrollbar_size = self
                    .scrollbar
                    .layout(BoxConstraints::exact(Size::new(1, constraints.max.height)));

                let elements_size = layout_list_elements(
                    &mut self.elements,
                    BoxConstraints::new(
                        Size::ZERO,
                        Size::new(
                            constraints.max.width - scrollbar_size.width,
                            u32::max_value(),
                        ),
                    ),
                );

                // Move up the list if we have room for elements from our current scroll offset
                let mut fill_height = 0;
                let mut scroll_max = (self.elements.len() - 1) as u32;
                for element in self.elements.iter().rev() {
                    fill_height += element.size.unwrap().height;

                    if fill_height > constraints.max.height {
                        break;
                    }

                    scroll_max -= 1;
                }

                (
                    elements_size.width + 1,
                    Some(scrollbar_size),
                    self.scrollbar.scroll_pos.get().min(scroll_max),
                    scroll_max,
                )
            } else {
                (elements_size.width, None, 0, 0)
            };

        self.scrollbar_size = scrollbar_size;
        self.scrollbar.scroll_pos.set(scroll_pos);
        self.scrollbar.scroll_max.set(scroll_max);

        Size::new(width, constraints.max.height)
    }

    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult {
        match event.cull(bounds) {
            Some(Event::Mouse { e, .. }) => match e {
                MouseEvent::WheelUp => {
                    self.scrollbar
                        .scroll_pos
                        .set(self.scrollbar.scroll_pos.get().saturating_sub(1));
                    return Err(Stop);
                }

                MouseEvent::WheelDown => {
                    self.scrollbar.scroll_pos.set(
                        (self.scrollbar.scroll_pos.get() + 1).min(self.scrollbar.scroll_max.get()),
                    );
                    return Err(Stop);
                }

                _ => {}
            },

            _ => {}
        }

        if let Some(scrollbar_size) = self.scrollbar_size {
            self.scrollbar.handle_event(
                ctx,
                event,
                IRect::new(
                    bounds.right() - scrollbar_size.width,
                    bounds.y,
                    scrollbar_size.width,
                    scrollbar_size.height,
                ),
            )?;
        }

        let mut y = 0;

        for element in self
            .elements
            .iter_mut()
            .skip(self.scrollbar.scroll_pos.get() as usize)
        {
            if let Some(size) = element.size {
                let bottom = y + size.height;

                if bottom > bounds.bottom() {
                    break;
                }

                element.element.handle_event(
                    ctx,
                    event,
                    IRect::new(bounds.x, bounds.y + y, size.width, size.height),
                )?;

                y = bottom;
            }
        }

        Ok(Continue)
    }
}

fn layout_list_elements(elements: &mut [ListElement], constraints: BoxConstraints) -> Size {
    let mut size = Size::new(0, 0);

    for element in elements {
        let element_size = element.element.layout(constraints);

        size.width = size.width.max(element_size.width);
        size.height += element_size.height;

        element.size = Some(element_size);
    }

    size
}

pub struct ListElement {
    pub element: Box<dyn Element>,
    size: Option<Size>,
}

impl ListElement {
    pub fn new(element: Box<dyn Element>) -> Self {
        Self {
            element,
            size: None,
        }
    }
}

impl From<Box<dyn Element>> for ListElement {
    fn from(element: Box<dyn Element>) -> Self {
        Self::new(element)
    }
}

pub struct Padding<T> {
    inner: T,
    top: u32,
    right: u32,
    bottom: u32,
    left: u32,
}

impl<T: Element> Padding<T> {
    pub fn new(inner: T, top: u32, right: u32, bottom: u32, left: u32) -> Self {
        Self {
            inner,
            top,
            right,
            bottom,
            left,
        }
    }
}

impl<T: Element> Element for Padding<T> {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        trace!("Padding relayout");

        self.inner
            .layout(constraints.shrink(Size::new(self.right + self.left, self.top + self.bottom)))
    }

    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult {
        self.inner.handle_event(
            ctx,
            event,
            IRect::new(
                bounds.x + self.left,
                bounds.y + self.top,
                bounds.w.saturating_sub(self.left + self.right),
                bounds.h.saturating_sub(self.top + self.bottom),
            ),
        )
    }
}

pub struct Centered<T> {
    inner: T,
    inner_size: Option<Size>,
}

impl<T: Element> Centered<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            inner_size: None,
        }
    }
}

impl<T: Element> Element for Centered<T> {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        trace!("Centered relayout");

        self.inner_size = dbg!(Some(
            self.inner
                .layout(BoxConstraints::new(Size::new(0, 0), constraints.max))
        ));
        constraints.max
    }

    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult {
        let inner_size = self.inner_size.unwrap();

        self.inner.handle_event(
            ctx,
            event,
            IRect::new(
                bounds.x + (bounds.w - inner_size.width) / 2,
                bounds.y + (bounds.h - inner_size.height) / 2,
                inner_size.width,
                inner_size.height,
            ),
        )
    }
}

pub struct ScrollBar {
    pub scroll_pos: Binding<u32>,
    pub scroll_max: Binding<u32>,
    pub direction: LayoutDirection,
}

impl ScrollBar {
    pub fn new(
        scroll_pos: Binding<u32>,
        scroll_max: Binding<u32>,
        direction: LayoutDirection,
    ) -> Self {
        Self {
            scroll_pos,
            scroll_max,
            direction,
        }
    }

    pub fn scroll_up(&mut self, ctx: &mut UiContext) {
        self.scroll_to(ctx, self.scroll_pos.get().saturating_sub(1));
    }

    pub fn scroll_down(&mut self, ctx: &mut UiContext) {
        self.scroll_to(ctx, self.scroll_pos.get() + 1);
    }

    pub fn scroll_to(&mut self, ctx: &mut UiContext, new_pos: u32) {
        let old_pos = self.scroll_pos.get();
        if old_pos != new_pos {
            self.scroll_pos.set(new_pos.min(self.scroll_max.get()));
            ctx.relayout = true;
        }
    }

    fn caret_pos(&self, size: Size) -> u32 {
        let scroll_r = self.scroll_pos.get() as f32 / self.scroll_max.get() as f32;
        (scroll_r * (size.dir(self.direction).saturating_sub(3)) as f32).round() as u32
    }
}

impl Element for ScrollBar {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        trace!("ScrollBar relayout");

        match self.direction {
            LayoutDirection::Horizontal => {
                Size::new(constraints.max.width, constraints.min.height.max(1))
            }

            LayoutDirection::Vertical => {
                Size::new(constraints.min.width.max(1), constraints.max.height)
            }
        }
    }

    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult {
        let bar_bounds = match self.direction {
            LayoutDirection::Horizontal => {
                IRect::new(bounds.x + 1, bounds.y, bounds.w.saturating_sub(2), bounds.h)
            }
            LayoutDirection::Vertical => {
                IRect::new(bounds.x, bounds.y + 1, bounds.w, bounds.h.saturating_sub(2))
            }
        };

        match event.cull(bounds) {
            Some(Event::Mouse { pos, e }) => match e {
                MouseEvent::WheelUp => {
                    self.scroll_up(ctx);
                    Err(Stop)
                }

                MouseEvent::WheelDown => {
                    self.scroll_down(ctx);
                    Err(Stop)
                }

                MouseEvent::ButtonDown { button } if button == MouseButton::Left => {
                    let scrollbar_pos =
                        pos.dir(self.direction) - bounds.point().dir(self.direction);

                    if scrollbar_pos == 0 {
                        self.scroll_up(ctx);
                    } else if scrollbar_pos == bounds.dir_end(self.direction) - 1 {
                        self.scroll_down(ctx);
                    } else if scrollbar_pos != self.caret_pos(bounds.size()) {
                        let scroll_pos = ((scrollbar_pos - 1) as f32
                            / (bar_bounds.size().dir(self.direction).saturating_sub(1)) as f32
                            * self.scroll_max.get() as f32)
                            .round() as u32;

                        self.scroll_to(ctx, scroll_pos);
                    }

                    Err(Stop)
                }

                MouseEvent::ButtonDrag { button, start_pos }
                    if button == MouseButton::Left && bar_bounds.contains(start_pos) =>
                {
                    let scroll_pos = (pos
                        .dir(self.direction)
                        .saturating_sub(*bar_bounds.point().dir(self.direction))
                        as f32
                        / (bar_bounds.size().dir(self.direction).saturating_sub(1)) as f32
                        * self.scroll_max.get() as f32)
                        .round() as u32;

                    self.scroll_to(ctx, scroll_pos);

                    Err(Stop)
                }

                _ => Ok(Continue),
            },

            Some(Event::Draw) => {
                let caret = Voxel2::new(0x2EC).background(Some(color::GRAY));
                let bg = Voxel2::new(0).background(Some(color::DARK_GRAY));

                match self.direction {
                    LayoutDirection::Horizontal => {
                        let left_arrow = Voxel2::new(0x11).background(Some(color::GRAY));
                        let right_arrow = Voxel2::new(0x10).background(Some(color::GRAY));

                        let caret_x = bounds.left() + 1 + self.caret_pos(bounds.size());

                        for y in bounds.top()..bounds.bottom() {
                            ctx.batch.add(&left_arrow, [bounds.left(), y]);

                            for x in (bounds.left() + 1)..caret_x {
                                ctx.batch.add(&bg, [x, y]);
                            }

                            ctx.batch.add(&caret, [caret_x, y]);

                            for x in (caret_x + 1)..(bounds.right() - 1) {
                                ctx.batch.add(&bg, [x, y]);
                            }

                            ctx.batch.add(&right_arrow, [bounds.right() - 1, y]);
                        }
                    }

                    LayoutDirection::Vertical => {
                        let top_arrow = Voxel2::new(0x1E).background(Some(color::GRAY));
                        let bottom_arrow = Voxel2::new(0x1F).background(Some(color::GRAY));

                        let caret_y = bounds.top() + 1 + self.caret_pos(bounds.size());

                        for x in bounds.left()..bounds.right() {
                            ctx.batch.add(&top_arrow, [x, bounds.top()]);

                            for y in (bounds.top() + 1)..caret_y {
                                ctx.batch.add(&bg, [x, y]);
                            }

                            ctx.batch.add(&caret, [x, caret_y]);

                            for y in (caret_y + 1)..(bounds.bottom() - 1) {
                                ctx.batch.add(&bg, [x, y]);
                            }

                            ctx.batch.add(&bottom_arrow, [x, bounds.bottom() - 1]);
                        }
                    }
                }

                Ok(Continue)
            }

            _ => Ok(Continue),
        }
    }
}

pub struct KataText {
    pub voxels: Vec<Voxel2>,
}

impl KataText {
    pub fn from_voxels(voxels: Vec<Voxel2>) -> Self {
        Self { voxels }
    }

    pub fn from_colored_str(s: &str, color: Color) -> Self {
        Self::from_voxels(
            s.char_indices()
                .map(|(i, c)| {
                    Voxel2::new(if c.is_ascii() {
                        u16::from(s.as_bytes()[i])
                    } else {
                        0x082D // Square
                    })
                    .foreground(color)
                })
                .collect(),
        )
    }

    pub fn from_str(s: &str) -> Self {
        Self::from_colored_str(s, color::WHITE)
    }
}

impl From<&str> for KataText {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl Element for KataText {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        trace!("Text relayout");

        let n = self.voxels.len() as u32;

        if constraints.max.width == 0 {
            Size::new(0, 0)
        } else {
            Size::new(
                n.min(constraints.max.width).max(constraints.min.width),
                n / constraints.max.width + if n % constraints.max.width > 0 { 1 } else { 0 },
            )
        }
    }

    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult {
        match event {
            Event::Draw if bounds.w > 0 => {
                for (i, voxel) in self.voxels.iter().enumerate() {
                    ctx.batch.add(
                        voxel,
                        [
                            bounds.x + i as u32 % bounds.w,
                            bounds.y + i as u32 / bounds.w,
                        ],
                    );
                }
                Ok(Continue)
            }

            _ => Ok(Continue),
        }
    }
}

pub struct StackedLayout {
    elements: Vec<StackedElement>,
    direction: LayoutDirection,
    dividers: bool,
}

impl StackedLayout {
    pub fn empty(direction: LayoutDirection) -> Self {
        Self::from_vec(direction, Vec::new())
    }

    pub fn from_vec(direction: LayoutDirection, elements: Vec<StackedElement>) -> Self {
        Self {
            elements,
            direction,
            dividers: false,
        }
    }

    pub fn horizontal(elements: Vec<StackedElement>) -> Self {
        Self::from_vec(LayoutDirection::Horizontal, elements)
    }

    pub fn vertical(elements: Vec<StackedElement>) -> Self {
        Self::from_vec(LayoutDirection::Vertical, elements)
    }

    pub fn with_dividers(self) -> Self {
        Self {
            dividers: true,
            ..self
        }
    }

    fn scan_sizes(&self) -> (u32, usize) {
        let mut total_size = 0;
        let mut free = 0;

        for element in self.elements.iter() {
            if let Some(size) = element.size {
                total_size += size;
            } else {
                free += 1;
            }
        }

        (total_size, free)
    }
}

impl Element for StackedLayout {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        trace!("StackedLayout relayout");

        let (total_size, free) = self.scan_sizes();
        let n = self.elements.len() as u32;

        let mut size_allowance = constraints.max[self.direction];
        if self.dividers {
            size_allowance = size_allowance.saturating_sub(n.saturating_sub(1));
        }

        let fullness = total_size.cmp(&size_allowance);

        match (fullness, free) {
            (Ordering::Equal, 0) => {
                // We're all good
                trace!("StackedLayout children-only relayout");

                for element in self.elements.iter_mut() {
                    element.element.layout(BoxConstraints::exact(
                        constraints
                            .max
                            .with_dir(self.direction, element.size.unwrap()),
                    ));
                }
            }

            (Ordering::Greater, _) | (Ordering::Equal, _) => {
                // Overfull - Full relayout
                trace!("StackedLayout full relayout");
                for (i, element) in self.elements.iter_mut().enumerate() {
                    let s = spread(i as u32, size_allowance, n);

                    element.size = Some(s);
                    element.element.layout(BoxConstraints::exact(
                        constraints.max.with_dir(self.direction, s),
                    ));
                }
            }

            (Ordering::Less, 0) => {
                // Add to existing elements in equal parts
                trace!("StackedLayout adding to existing elements");
                for (i, element) in self.elements.iter_mut().enumerate() {
                    let s =
                        element.size.unwrap() + spread(i as u32, size_allowance - total_size, n);

                    element.size = Some(s);
                    element.element.layout(BoxConstraints::exact(
                        constraints.max.with_dir(self.direction, s),
                    ));
                }
            }

            (Ordering::Less, _) => {
                // Spread to free elements in equal parts
                trace!("StackedLayout adding to new elements");
                for (i, element) in self
                    .elements
                    .iter_mut()
                    .filter(|e| e.size.is_none())
                    .enumerate()
                {
                    let s = element.size.unwrap_or_else(|| {
                        spread(i as u32, size_allowance - total_size, free as u32)
                    });

                    element.size = Some(s);

                    element.element.layout(BoxConstraints::exact(
                        constraints.max.with_dir(self.direction, s),
                    ));
                }
            }
        };

        constraints.max
    }

    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult {
        let mut offset = 0;

        for (i, element) in self.elements.iter_mut().enumerate() {
            if self.dividers && i > 0 {
                let div_bounds = bounds.slice_dir(self.direction, offset..(offset + 1));

                match event {
                    Event::Draw => {
                        let div_voxel = Voxel2::new(match self.direction {
                            LayoutDirection::Horizontal => 0x266,
                            LayoutDirection::Vertical => 0x265,
                        });

                        for p in div_bounds.points() {
                            ctx.batch.add(&div_voxel, p);
                        }
                    }

                    _ => {}
                }

                offset += 1;
            }

            let element_size = element.size.unwrap();
            element.element.handle_event(
                ctx,
                event,
                bounds.slice_dir(self.direction, offset..(offset + element_size)),
            )?;
            offset += element_size;
        }

        Ok(Continue)
    }
}

#[inline(always)]
fn spread(i: u32, total: u32, n: u32) -> u32 {
    total / n + if i < total % n { 1 } else { 0 }
}

pub struct StackedElement {
    element: Box<dyn Element>,
    size: Option<u32>,
}

impl StackedElement {
    pub fn new(element: Box<dyn Element>) -> Self {
        Self {
            element,
            size: None,
        }
    }
}

impl From<Box<dyn Element>> for StackedElement {
    fn from(element: Box<dyn Element>) -> Self {
        Self::new(element)
    }
}

pub struct FlexLayout {
    elements: Vec<FlexElement>,
    direction: LayoutDirection,
}

impl FlexLayout {
    pub fn empty(direction: LayoutDirection) -> Self {
        Self::from_vec(direction, Vec::new())
    }

    pub fn from_vec(direction: LayoutDirection, elements: Vec<FlexElement>) -> Self {
        Self {
            elements,
            direction,
        }
    }

    pub fn horizontal(elements: Vec<FlexElement>) -> Self {
        Self::from_vec(LayoutDirection::Horizontal, elements)
    }

    pub fn vertical(elements: Vec<FlexElement>) -> Self {
        Self::from_vec(LayoutDirection::Vertical, elements)
    }
}

impl Element for FlexLayout {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        trace!("FlexLayout relayout");

        let mut free = constraints.max;
        let mut max_other = 0;

        for fixed_element in self.elements.iter_mut().filter(|e| e.flex == 0) {
            let element_size = fixed_element
                .element
                .layout(dbg!(BoxConstraints::new(Size::ZERO, free)));

            free = dbg!(
                free.shrink(Size::ZERO.with_dir(self.direction, element_size[self.direction]))
            );
            fixed_element.size = Some(element_size[self.direction]);
            max_other = max_other.max(element_size[self.direction.other()]);
        }

        let total_flex: u32 = self.elements.iter().map(|e| e.flex).sum();

        let mut start_flex = 0;
        for flex_element in self.elements.iter_mut().filter(|e| e.flex > 0) {
            let end_flex = start_flex + flex_element.flex;

            let element_size = spread_flex(start_flex..end_flex, free[self.direction], total_flex);

            flex_element.element.layout(BoxConstraints::exact(
                free.with_dir(self.direction, element_size),
            ));

            flex_element.size = Some(element_size);
            start_flex = end_flex;
        }

        if total_flex > 0 {
            constraints.max
        } else {
            Size::default().with_dir(
                self.direction,
                constraints.max.dir(self.direction) - free.dir(self.direction),
            )
        }
        .with_dir(self.direction.other(), max_other)
    }

    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult {
        let mut offset = 0;

        for element in self.elements.iter_mut() {
            let element_size = element.size.unwrap();
            element.element.handle_event(
                ctx,
                event,
                bounds.slice_dir(self.direction, offset..(offset + element_size)),
            )?;
            offset += element_size;
        }

        Ok(Continue)
    }
}

#[inline(always)]
fn spread_flex(flex_range: Range<u32>, total: u32, total_flex: u32) -> u32 {
    let flex = flex_range.end - flex_range.start;
    let threshold = total % total_flex;

    (total / total_flex * flex)
        + if flex_range.start < threshold {
            (threshold - flex_range.start).min(flex)
        } else {
            0
        }
}

pub struct FlexElement {
    pub element: Box<dyn Element>,
    pub flex: u32,
    size: Option<u32>,
}

impl FlexElement {
    pub fn flex(element: Box<dyn Element>, flex: u32) -> Self {
        Self {
            element,
            flex,
            size: None,
        }
    }

    pub fn fixed(element: Box<dyn Element>) -> Self {
        Self {
            element,
            flex: 0,
            size: None,
        }
    }
}

pub struct VoxelDisplay<B> {
    pub voxel: B,
}

impl<B: Bound<Voxel2>> VoxelDisplay<B> {
    pub fn new(voxel: B) -> Self {
        Self { voxel }
    }
}

impl<B: Bound<Voxel2>> Element for VoxelDisplay<B> {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        Size::new(constraints.min.width.max(1), constraints.min.height.max(1))
    }

    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult {
        match event {
            Event::Draw => ctx.batch.add(&self.voxel.get(), bounds.point()),
            _ => {}
        }

        Ok(Continue)
    }
}

pub struct Placeholder<F> {
    voxel: Voxel2,
    size_fn: F,
}

impl<F> Placeholder<F>
where
    F: Fn(BoxConstraints) -> Size,
{
    pub fn new(voxel: Voxel2, size_fn: F) -> Self {
        Self { voxel, size_fn }
    }
}

impl<F> Element for Placeholder<F>
where
    F: Fn(BoxConstraints) -> Size,
{
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        (self.size_fn)(constraints)
    }

    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult {
        match event {
            Event::Draw => {
                for p in bounds.points() {
                    ctx.batch.add(&self.voxel, p);
                }
            }
            _ => {}
        }

        Ok(Continue)
    }
}

pub struct Filling {
    voxel: Voxel2,
}

impl Filling {
    pub fn new(voxel: Voxel2) -> Self {
        Self { voxel }
    }

    pub fn blank() -> Self {
        Self::new(Voxel2::default())
    }
}

impl Element for Filling {
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        constraints.max
    }

    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult {
        match event {
            Event::Draw => {
                for p in bounds.points() {
                    ctx.batch.add(&self.voxel, p);
                }
            }
            _ => {}
        }

        Ok(Continue)
    }
}

pub struct WithEvents<T, F> {
    element: T,
    handler: F,
}

impl<T, F> Deref for WithEvents<T, F> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.element
    }
}

impl<T, F> DerefMut for WithEvents<T, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.element
    }
}

impl<T, F> Element for WithEvents<T, F>
where
    T: Element,
    F: FnMut(&mut T, &mut UiContext, Event, IRect) -> EventResult,
{
    fn layout(&mut self, constraints: BoxConstraints) -> Size {
        self.element.layout(constraints)
    }

    fn handle_event(&mut self, ctx: &mut UiContext, event: Event, bounds: IRect) -> EventResult {
        (self.handler)(&mut self.element, ctx, event, bounds)?;
        self.element.handle_event(ctx, event, bounds)?;

        Ok(Continue)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_spread() {
        for n in 1..100 {
            for total in 1..100 {
                assert_eq!((0..n).map(|i| spread(i, total, n)).sum::<u32>(), total);
            }
        }
    }

    #[test]
    fn text_flex_spread() {
        for total_flex in 1..10 {
            for total in 1..100 {
                for flex_1 in 0..total_flex {
                    for flex_2 in flex_1..total_flex {
                        let flex_range_1 = 0..flex_1;
                        let flex_range_2 = flex_1..flex_2;
                        let flex_range_3 = flex_2..total_flex;

                        assert_eq!(
                            spread_flex(flex_range_1, total, total_flex)
                                + spread_flex(flex_range_2, total, total_flex)
                                + spread_flex(flex_range_3, total, total_flex),
                            total
                        );
                    }
                }
            }
        }
    }
}
