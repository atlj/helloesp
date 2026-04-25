use core::cmp::{max, min};

use color_core::Color;

use crate::{
    DrawCommand,
    geometry::{Position2, Size2, validity::Valid},
};

pub trait Shape<Validity> {
    fn to_draw_commands(
        &self,
    ) -> impl Iterator<Item = DrawCommand<Validity, impl Iterator<Item = Color>>>;
}

#[derive(Clone, Debug)]
pub struct Rectangle {
    pub position: Position2<Valid>,
    pub size: Size2<Valid>,
    pub corner_radius: u16,
    pub fill: Color,
}

impl Shape<Valid> for Rectangle {
    fn to_draw_commands(
        &self,
    ) -> impl Iterator<Item = DrawCommand<Valid, impl Iterator<Item = Color>>> {
        let smaller_dimension = min(self.size.width, self.size.height);
        let normalized_corner_radius = min(self.corner_radius, smaller_dimension / 2);

        let center_y = self.size.height / 2;
        let top_corner_y = min(normalized_corner_radius, center_y);

        let bottom_corner_y = max(
            self.size.height.saturating_sub(normalized_corner_radius),
            center_y,
        );

        (0..self.size.height).map(move |y| {
            // 1. Get the inset
            // TODO branchless
            let row_inset = if y >= top_corner_y && y <= bottom_corner_y {
                0
            } else {
                // a. Find the closest corner center
                // TODO branchless
                let closest_corner_y = if y < center_y {
                    top_corner_y
                } else {
                    bottom_corner_y
                };

                let distance_to_closest_corner_y = closest_corner_y.abs_diff(y);

                // b. Calculate the inset, since our corners are always the same, only use the same
                // corner
                normalized_corner_radius.saturating_sub(
                    (normalized_corner_radius
                        .pow(2)
                        .abs_diff(distance_to_closest_corner_y.pow(2)))
                    .isqrt(),
                )
            };

            let total_inset = row_inset * 2;
            let row_width = self
                .size
                .width
                .checked_sub(total_inset)
                .unwrap_or(self.size.width);

            let colors = (0..row_width).map(move |_| self.fill);

            let position = Position2::new(row_inset + self.position.x, y + self.position.y);
            let size = Size2::new(row_width, 1);

            DrawCommand::new(position, size, colors).unchecked_validate()
        })
    }
}
