use std::cmp::Ordering;

use crate::geometry::rect::MyRect;
use bevy::prelude::*;

pub struct Circle {
    pub center: Vec2,
    radius: f32,
}

impl Circle {
    pub fn new<P>(center: P, radius: f32) -> Self
    where
        P: Into<Vec2>,
    {
        assert!(radius >= 0.0, "Radius of circle is negative number");
        Self {
            center: center.into(),
            radius,
        }
    }

    /// http://mathworld.wolfram.com/Circle-LineIntersection.html
    pub fn line_intersection<P>(&self, point1: P, point2: P) -> Vec<Vec2>
    where
        P: Into<Vec2>,
    {
        // Translate the line into the coordinate system relative to the center of the circle.
        let point1: Vec2 = point1.into() - self.center;
        let point2: Vec2 = point2.into() - self.center;

        let line_vector: Vec2 = point2 - point1;
        let dr2 = line_vector.dot(line_vector);
        let d = point1.perp_dot(point2);
        let discriminant = self.radius * self.radius * dr2 - d * d;

        match discriminant.partial_cmp(&0.0) {
            Some(Ordering::Greater) => {
                // Two intersections
                let discr_sqrt = discriminant.sqrt();
                let mut result = Vec::with_capacity(2);

                let dx = -d * line_vector.x;
                let dy = d * line_vector.y;
                let x_discr = line_vector.y.signum() * line_vector.x * discr_sqrt;
                let y_dyscr = line_vector.y.abs() * discr_sqrt;

                let x = (dy - x_discr) / dr2 + self.center.x;
                let y = (dx - y_dyscr) / dr2 + self.center.y;
                result.push(Vec2::new(x, y));

                let x = (dy + x_discr) / dr2 + self.center.x;
                let y = (dx + y_dyscr) / dr2 + self.center.y;
                result.push(Vec2::new(x, y));

                result
            }
            Some(Ordering::Equal) => {
                // One intersection (tangent)
                let x = d * line_vector.y / dr2 + self.center.x;
                let y = -d * line_vector.x / dr2 + self.center.y;
                vec![Vec2::new(x, y)]
            }
            _ => {
                // No intersections
                vec![]
            }
        }
    }

    pub fn segment_intersection<P>(&self, point1: P, point2: P) -> Vec<Vec2>
    where
        P: Into<Vec2>,
    {
        let point1 = point1.into();
        let point2 = point2.into();
        let result = self.line_intersection(point1, point2);

        if result.is_empty() {
            return result;
        }

        let segment_vector = point2 - point1;
        let segment_len2 = segment_vector.dot(segment_vector);
        result
            .into_iter()
            .filter(|&res_point| {
                let res_vector: Vec2 = res_point - point1;
                let dot = segment_vector.dot(res_vector);
                dot >= 0.0 && dot <= segment_len2
            })
            .collect()
    }

    pub fn area_of_rect_intersection(&self, mut rect: MyRect) -> f32 {
        rect.left -= self.center.x;
        rect.right -= self.center.x;
        rect.top -= self.center.y;
        rect.bottom -= self.center.y;

        self.area_of_normalized_rect_intersection(rect)
    }

    fn area_of_normalized_rect_intersection(&self, mut rect: MyRect) -> f32 {
        if rect.bottom < 0.0 {
            if rect.top < 0.0 {
                // the rect is completely under, just flip it above //and try again
                rect = MyRect {
                    top: -rect.bottom,
                    bottom: -rect.top,
                    ..rect
                };
            } else {
                // the rect is both above and below, divide it to two rects and go again
                let top_rect_part = MyRect {
                    top: rect.top,
                    bottom: 0.0,
                    ..rect
                };
                let bottom_rect_part = MyRect {
                    top: -rect.bottom,
                    bottom: 0.0,
                    ..rect
                };
                return self.area_of_normalized_rect_intersection(top_rect_part)
                    + self.area_of_normalized_rect_intersection(bottom_rect_part);
            }
        }

        // area of the lower box minus area of the higher box
        let x0 = rect.left;
        let x1 = rect.right;
        let y0 = rect.bottom.abs();
        let y1 = rect.top.abs();
        self.area_of_intersection_of_infinity_tall_rect(x0, x1, y0)
            - self.area_of_intersection_of_infinity_tall_rect(x0, x1, y1)
    }

    /// Area of intersection of an infinitely tall rect with left edge at x0,
    /// right edge at x1, bottom edge at h and top edge at infinity,
    /// with circle centered at the origin.
    #[inline]
    fn area_of_intersection_of_infinity_tall_rect(&self, x0: f32, x1: f32, h: f32) -> f32 {
        // assert!(x0 > x1);
        let s = self.section(h);
        // integrate the area
        self.g(s.min(x1).max(-s), h) - self.g(s.min(x0).max(-s), h)
    }

    /// Returns the positive root of intersection of line y = h
    /// with circle centered at the origin.
    /// https://www.wolframalpha.com/input/?i=r+*+sin%28acos%28x+%2F+r%29%29+%3D+h
    #[inline]
    fn section(&self, h: f32) -> f32 {
        if h < self.radius {
            (self.radius * self.radius - h * h).sqrt()
        } else {
            0.0
        }
    }

    /// Indefinite integral of circle segment.
    /// https://www.wolframalpha.com/input/?i=r+*+sin%28acos%28x+%2F+r%29%29+-+h
    #[inline]
    fn g(&self, x: f32, h: f32) -> f32 {
        let r2 = self.radius * self.radius;
        let frac_x_r = x / self.radius;

        0.5 * ((1.0 - frac_x_r * frac_x_r).sqrt() * x * self.radius + r2 * frac_x_r.asin()
            - 2.0 * h * x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    #[test]
    fn test_circle_line_no_intersections() {
        let circle = Circle::new([1.0, 2.0], 5.0);

        // Line above circle
        let res = circle.line_intersection([0.0, 8.0], [1.0, 8.0]);
        assert_eq!(res.len(), 0);

        // Line under circle
        let res = circle.line_intersection([0.0, -4.0], [1.0, -4.0]);
        assert_eq!(res.len(), 0);

        // Line before circle
        let res = circle.line_intersection([-5.0, 0.0], [-5.0, 1.0]);
        assert_eq!(res.len(), 0);

        // Line after circle
        let res = circle.line_intersection([7.0, 0.0], [7.0, 1.0]);
        assert_eq!(res.len(), 0);
    }

    #[test]
    fn test_circle_line_one_intersection() {
        let circle = Circle::new([1.0, 2.0], 5.0);

        // Line is tangent to up of circle
        let res = circle.line_intersection([0.0, 7.0], [1.0, 7.0]);
        assert_eq!(res.len(), 1);
        assert_eq!(res, vec![Vec2::new(1.0, 7.0)]);

        // Line is tangent to bottom of circle
        let res = circle.line_intersection([0.0, -3.0], [1.0, -3.0]);
        assert_eq!(res.len(), 1);
        assert_eq!(res, vec![Vec2::new(1.0, -3.0)]);

        // Line is tangent to left of circle
        let res = circle.line_intersection([-4.0, 0.0], [-4.0, 1.0]);
        assert_eq!(res.len(), 1);
        assert_eq!(res, vec![Vec2::new(-4.0, 2.0)]);

        // Line is tangent to right of circle
        let res = circle.line_intersection([6.0, 0.0], [6.0, 1.0]);
        assert_eq!(res.len(), 1);
        assert_eq!(res, vec![Vec2::new(6.0, 2.0)]);
    }

    #[test]
    fn test_circle_line_two_intersections() {
        let circle = Circle::new([1.0, 2.0], 5.0);

        // Line intersect upper half of circle
        let res = circle.line_intersection([0.0, 6.0], [1.0, 6.0]);
        assert_eq!(res.len(), 2);
        assert_eq!(res, vec![Vec2::new(-2.0, 6.0), Vec2::new(4.0, 6.0)]);

        // Line intersect lower half of circle
        let res = circle.line_intersection([0.0, -2.0], [1.0, -2.0]);
        assert_eq!(res.len(), 2);
        assert_eq!(res, vec![Vec2::new(-2.0, -2.0), Vec2::new(4.0, -2.0)]);

        // Line intersect left half of circle
        let res = circle.line_intersection([-3.0, 0.0], [-3.0, 1.0]);
        assert_eq!(res.len(), 2);
        assert_eq!(res, vec![Vec2::new(-3.0, -1.0), Vec2::new(-3.0, 5.0)]);

        // Line intersect right half of circle
        let res = circle.line_intersection([5.0, 0.0], [5.0, 1.0]);
        assert_eq!(res.len(), 2);
        assert_eq!(res, vec![Vec2::new(5.0, -1.0), Vec2::new(5.0, 5.0)]);

        // Line intersect center of circle
        let res = circle.line_intersection([0.0, 2.0], [1.0, 2.0]);
        assert_eq!(res.len(), 2);
        assert_eq!(res, vec![Vec2::new(-4.0, 2.0), Vec2::new(6.0, 2.0)]);
    }

    #[test]
    fn test_circle_segment_no_intersections() {
        let circle = Circle::new([1.0, 2.0], 5.0);

        // Line of segment is tangent to up of circle
        let res = circle.segment_intersection([3.0, 7.0], [4.0, 7.0]);
        assert_eq!(res.len(), 0);

        // Line of segment is intersect of circle
        let res = circle.segment_intersection([1.0, 9.0], [1.0, 8.0]);
        assert_eq!(res.len(), 0);

        // Segment inside of circle
        let res = circle.segment_intersection([3.0, 2.0], [4.0, 2.0]);
        assert_eq!(res.len(), 0);
    }

    #[test]
    fn test_circle_segment_has_intersections() {
        let circle = Circle::new([1.0, 2.0], 5.0);

        // Segment is tangent to up of circle
        let res = circle.segment_intersection([1.0, 7.0], [4.0, 7.0]);
        assert_eq!(res.len(), 1);
        assert_eq!(res, vec![Vec2::new(1.0, 7.0)]);

        // Segment is intersect of circle in one point
        let res = circle.segment_intersection([3.0, 2.0], [7.0, 2.0]);
        assert_eq!(res.len(), 1);
        assert_eq!(res, vec![Vec2::new(6.0, 2.0)]);

        // Segment is intersect of circle in two points
        let res = circle.segment_intersection([-4.0, 2.0], [7.0, 2.0]);
        assert_eq!(res.len(), 2);
        assert_eq!(res, vec![Vec2::new(-4.0, 2.0), Vec2::new(6.0, 2.0)]);
    }

    #[test]
    fn test_area_of_rect_intersection() {
        let circle = Circle::new((0.0, 0.0), 1.0);

        // unit circle completely inside a huge rect, area of intersection is pi
        let area = circle.area_of_rect_intersection(MyRect {
            left: -10.0,
            right: 10.0,
            top: 10.0,
            bottom: -10.0,
        });
        assert_eq!(area, PI);

        // half of unit circle inside a large box, area of intersection is pi/2
        let area = circle.area_of_rect_intersection(MyRect {
            left: -10.0,
            right: 0.0,
            top: 10.0,
            bottom: -10.0,
        });
        assert_eq!(area, FRAC_PI_2);

        // half of unit circle inside a large box, area of intersection is pi/2
        let area = circle.area_of_rect_intersection(MyRect {
            left: 0.0,
            right: 10.0,
            top: 10.0,
            bottom: -10.0,
        });
        assert_eq!(area, FRAC_PI_2);

        // half of unit circle inside a large box, area of intersection is pi/2
        let area = circle.area_of_rect_intersection(MyRect {
            left: -10.0,
            right: 10.0,
            top: 0.0,
            bottom: -10.0,
        });
        assert_eq!(area, FRAC_PI_2);

        // half of unit circle inside a large box, area of intersection is pi/2
        let area = circle.area_of_rect_intersection(MyRect {
            left: -10.0,
            right: 10.0,
            top: 10.0,
            bottom: 0.0,
        });
        assert_eq!(area, FRAC_PI_2);

        // unit box covering one quadrant of the circle, area of intersection is pi/4
        let area = circle.area_of_rect_intersection(MyRect {
            left: 0.0,
            right: 1.0,
            top: 1.0,
            bottom: 0.0,
        });
        assert_eq!(area, FRAC_PI_4);

        // unit box covering one quadrant of the circle, area of intersection is pi/4
        let area = circle.area_of_rect_intersection(MyRect {
            left: -1.0,
            right: 0.0,
            top: 1.0,
            bottom: 0.0,
        });
        assert_eq!(area, FRAC_PI_4);

        // unit box covering one quadrant of the circle, area of intersection is pi/4
        let area = circle.area_of_rect_intersection(MyRect {
            left: -1.0,
            right: 0.0,
            top: 0.0,
            bottom: -1.0,
        });
        assert_eq!(area, FRAC_PI_4);

        // unit box covering one quadrant of the circle, area of intersection is pi/4
        let area = circle.area_of_rect_intersection(MyRect {
            left: 0.0,
            right: 1.0,
            top: 0.0,
            bottom: -1.0,
        });
        assert_eq!(area, FRAC_PI_4);

        // huge box completely outside a circle (left), area of intersection is 0
        let area = circle.area_of_rect_intersection(MyRect {
            left: -20.0,
            right: -10.0,
            top: 10.0,
            bottom: -10.0,
        });
        assert_eq!(area, 0.0);

        // huge box completely outside a circle (right), area of intersection is 0
        let area = circle.area_of_rect_intersection(MyRect {
            left: 10.0,
            right: 20.0,
            top: 10.0,
            bottom: -10.0,
        });
        assert_eq!(area, 0.0);

        // huge box completely outside a circle (below), area of intersection is 0
        let area = circle.area_of_rect_intersection(MyRect {
            left: -10.0,
            right: 10.0,
            top: -10.0,
            bottom: -20.0,
        });
        assert_eq!(area, 0.0);

        // huge box completely outside a circle (above), area of intersection is 0
        let area = circle.area_of_rect_intersection(MyRect {
            left: -10.0,
            right: 10.0,
            top: 20.0,
            bottom: 10.0,
        });
        assert_eq!(area, 0.0);

        // unit box completely inside a huge circle, area of intersection is 1
        let circle = Circle::new((0.0, 0.0), 10.0);
        let area = circle.area_of_rect_intersection(MyRect {
            left: -0.5,
            right: 0.5,
            top: 0.5,
            bottom: -0.5,
        });
        assert_eq!(area, 1.0);

        let circle = Circle::new((87.0, 489.0), 50.0);
        let area = circle.area_of_rect_intersection(MyRect {
            left: 100.0,
            right: 141.0,
            top: 507.0,
            bottom: 466.0,
        });
        assert!(area > 0.0);
    }
}
