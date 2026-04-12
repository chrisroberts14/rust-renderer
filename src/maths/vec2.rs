#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dot(&self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn length(&self) -> f32 {
        f32::sqrt(self.x * self.x + self.y * self.y)
    }

    pub fn normalise(&self) -> Vec2 {
        let len = self.length();
        Vec2 {
            x: self.x / len,
            y: self.y / len,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        assert_eq!(Vec2::new(1.0, 2.0), Vec2::new(1.0, 2.0));
    }

    #[test]
    fn test_add() {
        assert_eq!(
            Vec2::new(1.0, 2.0) + Vec2::new(3.0, 4.0),
            Vec2::new(4.0, 6.0)
        );
    }

    #[test]
    fn test_sub() {
        assert_eq!(
            Vec2::new(5.0, 3.0) - Vec2::new(2.0, 1.0),
            Vec2::new(3.0, 2.0)
        );
    }

    #[test]
    fn test_mul_scalar() {
        assert_eq!(Vec2::new(2.0, 3.0) * 4.0, Vec2::new(8.0, 12.0));
    }

    #[test]
    fn test_div_scalar() {
        assert_eq!(Vec2::new(6.0, 9.0) / 3.0, Vec2::new(2.0, 3.0));
    }

    #[test]
    fn test_neg() {
        assert_eq!(-Vec2::new(1.0, -2.0), Vec2::new(-1.0, 2.0));
    }

    #[test]
    fn test_dot_perpendicular() {
        assert_eq!(Vec2::new(1.0, 0.0).dot(Vec2::new(0.0, 1.0)), 0.0);
    }

    #[test]
    fn test_dot_value() {
        assert_eq!(Vec2::new(2.0, 3.0).dot(Vec2::new(4.0, 5.0)), 23.0); // 2*4 + 3*5
    }

    #[test]
    fn test_length() {
        // 3-4-5 right triangle gives an exact integer length
        assert_eq!(Vec2::new(3.0, 4.0).length(), 5.0);
    }

    #[test]
    fn test_normalise() {
        // (3, 4) / 5 = (0.6, 0.8); also verify the result is unit length
        let n = Vec2::new(3.0, 4.0).normalise();
        assert!((n.x - 0.6).abs() < 1e-5, "x = {}", n.x);
        assert!((n.y - 0.8).abs() < 1e-5, "y = {}", n.y);
        assert!((n.length() - 1.0).abs() < 1e-5, "length = {}", n.length());
    }
}
