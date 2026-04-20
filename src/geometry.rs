use core::marker::PhantomData;

use crate::geometry::validity::{Unchecked, Valid};

#[derive(Debug, Clone)]
pub struct Position2<Validity> {
    pub x: u16,
    pub y: u16,
    validity: PhantomData<Validity>,
}

impl<T> Position2<T> {
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
}

impl Position2<Unchecked> {
    pub(crate) const fn unchecked_validate(self) -> Position2<Valid> {
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

impl<T> Size2<T> {
    pub const fn new(width: u16, height: u16) -> Size2<Unchecked> {
        Size2 {
            width,
            height,
            validity: PhantomData,
        }
    }
}

impl Size2<Unchecked> {
    pub(crate) const fn unchecked_validate(self) -> Size2<Valid> {
        Size2 {
            width: self.width,
            height: self.height,
            validity: PhantomData,
        }
    }
}

pub mod validity {
    pub struct Valid;
    pub struct Unchecked;
}
