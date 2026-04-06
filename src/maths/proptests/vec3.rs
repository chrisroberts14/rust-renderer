use crate::maths::vec3::Vec3;
use proptest::prelude::*;

// Generates Vec3 with components in [-10, 10] — large enough to be interesting,
// small enough that accumulated f32 error stays below our tolerances.
fn vec3() -> impl Strategy<Value = Vec3> {
    (-10.0f32..10.0, -10.0f32..10.0, -10.0f32..10.0).prop_map(|(x, y, z)| Vec3::new(x, y, z))
}

proptest! {
    // normalise() should always return a unit vector.
    // prop_assume! skips near-zero vectors where the result would be degenerate.
    #[test]
    fn normalise_has_unit_length(v in vec3()) {
        prop_assume!(v.length() > 1e-6);
        let n = v.normalise();
        prop_assert!((n.length() - 1.0).abs() < 1e-5, "length = {}", n.length());
    }

    // dot(a, b) == dot(b, a) holds exactly: f32 multiply is commutative and
    // both sides sum the same three products in the same order.
    #[test]
    fn dot_is_commutative(a in vec3(), b in vec3()) {
        prop_assert_eq!(a.dot(b), b.dot(a));
    }

    // v·v should equal |v|², but sqrt then squaring introduces rounding error.
    #[test]
    fn dot_self_equals_length_squared(v in vec3()) {
        let dot = v.dot(v);
        let len_sq = v.length() * v.length();
        prop_assert!((dot - len_sq).abs() < 1e-3, "dot={dot}, len_sq={len_sq}");
    }

    // The cross product must be perpendicular to both input vectors.
    // With components up to 10, accumulated f32 error is bounded by ~1e-3.
    #[test]
    fn cross_is_perpendicular_to_inputs(a in vec3(), b in vec3()) {
        let c = a.cross(b);
        prop_assert!(c.dot(a).abs() < 1e-2, "c·a = {}", c.dot(a));
        prop_assert!(c.dot(b).abs() < 1e-2, "c·b = {}", c.dot(b));
    }

    // a×b == -(b×a) holds exactly: each component is computed from the same
    // two products (f32 mul is commutative), just negated.
    #[test]
    fn cross_is_anti_commutative(a in vec3(), b in vec3()) {
        prop_assert_eq!(a.cross(b), -b.cross(a));
    }

    // Rotation is a length-preserving operation (isometry).
    #[test]
    fn rotation_preserves_length(
        v in vec3(),
        angle in -std::f32::consts::TAU..std::f32::consts::TAU,
    ) {
        let len = v.length();
        prop_assert!((v.rotate_x(angle).length() - len).abs() < 1e-4, "rotate_x changed length");
        prop_assert!((v.rotate_y(angle).length() - len).abs() < 1e-4, "rotate_y changed length");
        prop_assert!((v.rotate_z(angle).length() - len).abs() < 1e-4, "rotate_z changed length");
    }

    // a+b == b+a holds exactly: f32 addition is commutative.
    #[test]
    fn add_is_commutative(a in vec3(), b in vec3()) {
        prop_assert_eq!(a + b, b + a);
    }

    // v + (-v) == 0 holds exactly: x - x is always 0.0 in IEEE 754.
    #[test]
    fn neg_cancels_add(v in vec3()) {
        prop_assert_eq!(v + (-v), Vec3::ZERO);
    }
}
