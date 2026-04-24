use core::marker::PhantomData;

use color_core::Color;

use crate::geometry::{
    Position2, Size2,
    validity::{Unchecked, Valid},
};

pub struct DrawCommand<Validity, I: Iterator<Item = Color>> {
    pub at: Position2<Validity>,
    pub size: Size2<Validity>,
    pub color_data: I,
    validity: PhantomData<Validity>,
}

impl<I: Iterator<Item = Color>> DrawCommand<Unchecked, I> {
    pub fn new(
        at: Position2<Unchecked>,
        size: Size2<Unchecked>,
        color_data: I,
    ) -> DrawCommand<Unchecked, I> {
        DrawCommand {
            at,
            size,
            color_data,
            validity: PhantomData,
        }
    }
}

pub trait Screen {
    const SIZE: Size2<Valid>;
    type Error: core::error::Error;

    fn set_brightness(&mut self, brightness: u8);

    fn get_brightness(&self) -> u8;

    fn draw<I: Iterator<Item = Color>>(
        &mut self,
        command: DrawCommand<Valid, I>,
    ) -> Result<(), Self::Error>;

    fn fill(
        &mut self,
        at: Position2<Valid>,
        size: Size2<Valid>,
        color: Color,
    ) -> Result<(), Self::Error> {
        let pixel_count = usize::from(size.width) * usize::from(size.height);
        let iter = (0..pixel_count).map(|_| color);

        let command = DrawCommand {
            at,
            size,
            color_data: iter,
            validity: PhantomData,
        };

        self.draw(command)
    }

    fn validate_draw_command<I: Iterator<Item = Color>>(
        DrawCommand {
            at,
            size,
            color_data,
            ..
        }: DrawCommand<Unchecked, I>,
    ) -> Option<DrawCommand<Valid, I>> {
        let at = Self::validate_position(at)?;
        let size = Self::validate_size(&at, size)?;

        Some(DrawCommand {
            at,
            size,
            color_data,
            validity: PhantomData,
        })
    }

    fn validate_position(position: Position2<Unchecked>) -> Option<Position2<Valid>> {
        if position.x < Self::SIZE.width && position.y < Self::SIZE.height {
            return Some(position.unchecked_validate());
        }

        None
    }

    fn validate_size(at: &Position2<Valid>, size: Size2<Unchecked>) -> Option<Size2<Valid>> {
        if size.height == 0 || size.width == 0 {
            return None;
        }

        let remaining_width = Self::SIZE.width.checked_sub(at.x)?;
        let remaining_height = Self::SIZE.height.checked_sub(at.y)?;
        if size.width > remaining_width || size.height > remaining_height {
            return None;
        }

        Some(size.unchecked_validate())
    }
}
