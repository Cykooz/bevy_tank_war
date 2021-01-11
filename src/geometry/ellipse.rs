use bevy::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Ellipse {
    pub center: Vec2,
    a: f32,
    b: f32,
    a2: f32,
    b2: f32,
}

impl Ellipse {
    pub fn new<P: Into<Vec2>>(center: P, a: f32, b: f32) -> Self {
        assert!(a >= 0.0, "'a' radius of ellipse is negative number");
        assert!(b >= 0.0, "'b' radius of ellipse is negative number");
        Ellipse {
            center: center.into(),
            a,
            b,
            a2: a * a,
            b2: b * b,
        }
    }

    pub fn point_position<P: Into<Vec2>>(&self, point: P) -> f32 {
        let point = point.into() - self.center;
        if self.a == 0. || self.b == 0. {
            let x_len = point.x.abs() - self.a;
            let y_len = point.y.abs() - self.b;
            x_len.max(y_len).max(0.)
        } else {
            point.x * point.x / self.a2 + point.y * point.y / self.b2 - 1.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_position() {
        let ellipse = Ellipse::new((10., -12.), 5., 2.);

        assert!(ellipse.point_position((10., -12.)) < 0.);
        assert_eq!(ellipse.point_position((15., -12.)), 0.);
        assert_eq!(ellipse.point_position((10., -10.)), 0.);
        assert!(ellipse.point_position((16., -12.)) > 0.);
    }

    #[test]
    fn test_point_position_zero_a() {
        let ellipse = Ellipse::new((10., -12.), 0., 2.);

        assert_eq!(ellipse.point_position((10., -12.)), 0.);
        assert_eq!(ellipse.point_position((10., -10.)), 0.);
        assert_eq!(ellipse.point_position((10., -11.)), 0.);
        assert!(ellipse.point_position((10., -9.)) > 0.);
        assert!(ellipse.point_position((11., -12.)) > 0.);
    }

    #[test]
    fn test_point_position_zero_b() {
        let ellipse = Ellipse::new((10., -12.), 5., 0.);

        assert_eq!(ellipse.point_position((10., -12.)), 0.);
        assert_eq!(ellipse.point_position((15., -12.)), 0.);
        assert_eq!(ellipse.point_position((13., -12.)), 0.);
        assert!(ellipse.point_position((16., -12.)) > 0.);
        assert!(ellipse.point_position((10., -11.)) > 0.);
    }

    #[test]
    fn test_point_position_zero_a_and_b() {
        let ellipse = Ellipse::new((10., -12.), 0., 0.);

        assert_eq!(ellipse.point_position((10., -12.)), 0.);
        assert!(ellipse.point_position((11., -12.)) > 0.);
        assert!(ellipse.point_position((10., -11.)) > 0.);
    }
}
