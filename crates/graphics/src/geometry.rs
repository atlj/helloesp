use core::{marker::PhantomData, ops::Div};

use crate::geometry::validity::{Unchecked, Valid};

#[derive(Debug, Clone)]
pub struct Position2<Validity> {
    pub x: u16,
    pub y: u16,
    validity: PhantomData<Validity>,
}

impl Position2<Unchecked> {
    pub const UPPER_LEFT: Position2<Valid> = Position2 {
        x: 0,
        y: 0,
        validity: PhantomData::<Valid>,
    };

    pub const fn new(x: u16, y: u16) -> Position2<Unchecked> {
        Position2 {
            x,
            y,
            validity: PhantomData,
        }
    }

    /// Converts to `Valid` without checking bounds.
    ///
    /// # Safety
    /// Caller must ensure the position lies within the target screen bounds.
    pub const fn unchecked_validate(self) -> Position2<Valid> {
        Position2 {
            x: self.x,
            y: self.y,
            validity: PhantomData,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Size2<Validity> {
    pub width: u16,
    pub height: u16,
    validity: PhantomData<Validity>,
}

impl<T> Div<u8> for Size2<T> {
    type Output = Size2<T>;

    fn div(self, rhs: u8) -> Self::Output {
        let Size2 { width, height, .. } = self;

        Size2 {
            width: width / rhs as u16,
            height: height / rhs as u16,
            validity: PhantomData,
        }
    }
}

impl<T> Size2<T> {
    pub const fn center_position(&self) -> Position2<T> {
        let x = self.width / 2;
        let y = self.height / 2;

        Position2 {
            x,
            y,
            validity: PhantomData,
        }
    }
}

impl Size2<Unchecked> {
    pub const fn new(width: u16, height: u16) -> Size2<Unchecked> {
        Size2 {
            width,
            height,
            validity: PhantomData,
        }
    }

    /// Converts to `Valid` without checking bounds.
    ///
    /// # Safety
    /// Caller must ensure the size fits within the target screen bounds.
    pub const fn unchecked_validate(self) -> Size2<Valid> {
        Size2 {
            width: self.width,
            height: self.height,
            validity: PhantomData,
        }
    }
}

pub mod validity {
    #[derive(Debug, Clone)]
    pub struct Valid;
    #[derive(Debug, Clone)]
    pub struct Unchecked;
}
