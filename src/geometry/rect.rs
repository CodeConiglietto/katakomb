use std::ops::{Index, IndexMut, Range};

pub use ggez::graphics::Rect as FRect;

use crate::ui::{LayoutDirection, Size};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct IRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl IRect {
    /// Create a new `Rect`.
    pub const fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    /// Create a new `Rect` with all values zero.
    pub const fn zero() -> Self {
        Self::new(0, 0, 0, 0)
    }

    /// Creates a new `Rect` at `0,0` with width and height 1.
    pub const fn one() -> Self {
        Self::new(0, 0, 1, 1)
    }

    /// Gets the `Rect`'s x and y coordinates as a `Point2`.
    pub const fn point(&self) -> mint::Point2<u32> {
        mint::Point2 {
            x: self.x,
            y: self.y,
        }
    }

    /// Returns the left edge of the `Rect`
    pub const fn left(&self) -> u32 {
        self.x
    }

    /// Returns the right edge of the `Rect`
    pub fn right(&self) -> u32 {
        self.x + self.w
    }

    /// Returns the top edge of the `Rect`
    pub const fn top(&self) -> u32 {
        self.y
    }

    /// Returns the bottom edge of the `Rect`
    pub fn bottom(&self) -> u32 {
        self.y + self.h
    }

    /// Checks whether the `Rect` contains a `Point`
    pub fn contains<P>(&self, point: P) -> bool
    where
        P: Into<mint::Point2<u32>>,
    {
        let point = point.into();
        point.x >= self.left()
            && point.x < self.right()
            && point.y < self.bottom()
            && point.y >= self.top()
    }

    /// Checks whether the `Rect` overlaps another `Rect`
    pub fn overlaps(&self, other: &Self) -> bool {
        self.left() < other.right()
            && self.right() > other.left()
            && self.top() < other.bottom()
            && self.bottom() > other.top()
    }

    /// Translates the `Rect` by an offset of (x, y)
    pub fn translate<V>(&mut self, offset: V)
    where
        V: Into<mint::Vector2<u32>>,
    {
        let offset = offset.into();
        self.x += offset.x;
        self.y += offset.y;
    }

    /// Moves the `Rect`'s origin to (x, y)
    pub fn move_to<P>(&mut self, destination: P)
    where
        P: Into<mint::Point2<u32>>,
    {
        let destination = destination.into();
        self.x = destination.x;
        self.y = destination.y;
    }

    /// Returns a new `Rect` that includes all points of these two `Rect`s.
    pub fn combine_with(self, other: Self) -> Self {
        let x = u32::min(self.x, other.x);
        let y = u32::min(self.y, other.y);
        let w = u32::max(self.right(), other.right()) - x;
        let h = u32::max(self.bottom(), other.bottom()) - y;
        Self { x, y, w, h }
    }

    pub fn to_f_rect(self) -> FRect {
        FRect {
            x: self.x as f32,
            y: self.y as f32,
            w: self.w as f32,
            h: self.h as f32,
        }
    }

    pub fn size(self) -> Size {
        Size::new(self.w, self.h)
    }

    pub fn slice_dir(self, direction: LayoutDirection, range: Range<u32>) -> Self {
        let mut r = self;

        match direction {
            LayoutDirection::Horizontal => {
                r.x += range.start;
                r.w = range.end - range.start;
            }
            LayoutDirection::Vertical => {
                r.y += range.start;
                r.h = range.end - range.start;
            }
        }

        r
    }

    pub fn dir_start(self, direction: LayoutDirection) -> u32 {
        match direction {
            LayoutDirection::Horizontal => self.x,
            LayoutDirection::Vertical => self.y,
        }
    }

    pub fn dir_end(self, direction: LayoutDirection) -> u32 {
        match direction {
            LayoutDirection::Horizontal => self.x + self.w,
            LayoutDirection::Vertical => self.y + self.h,
        }
    }

    pub fn dir(self, direction: LayoutDirection) -> Range<u32> {
        self.dir_start(direction)..self.dir_end(direction)
    }

    pub fn points(self) -> Points {
        Points { rect: self, i: 0 }
    }
}

pub struct Points {
    rect: IRect,
    i: usize,
}

impl Iterator for Points {
    type Item = mint::Point2<u32>;

    fn next(&mut self) -> Option<Self::Item> {
        if (self.rect.w * self.rect.h) as usize == self.i {
            None
        } else {
            let pos = mint::Point2::from([
                self.rect.x + self.i as u32 % self.rect.w,
                self.rect.y + self.i as u32 / self.rect.w,
            ]);
            self.i += 1;
            Some(pos)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.rect.w * self.rect.h) as usize - self.i;
        (n, Some(n))
    }
}

impl ExactSizeIterator for Points {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_points() {
        assert_points(
            IRect::new(0, 0, 2, 3),
            &[[0, 0], [1, 0], [0, 1], [1, 1], [0, 2], [1, 2]],
        );
        assert_points(IRect::new(0, 0, 0, 0), &[]);
        assert_points(IRect::new(20, 50, 3, 1), &[[20, 50], [21, 50], [22, 50]]);
        assert_points(IRect::new(10, 10, 1, 1), &[[10, 10]]);
    }

    fn assert_points(rect: IRect, expected: &[[u32; 2]]) {
        let actual: Vec<_> = rect.points().collect();
        let expected: Vec<mint::Point2<u32>> =
            expected.iter().cloned().map(mint::Point2::from).collect();
        assert_eq!(actual, expected);
    }
}
