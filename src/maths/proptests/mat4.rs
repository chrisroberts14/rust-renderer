use crate::maths::mat4::Mat4;
use proptest::prelude::*;

/// Generates a Mat4 with components in [-10, 10]
fn mat4() -> impl Strategy<Value = Mat4> {
    prop::array::uniform4(prop::array::uniform4(-10.0f32..10.0)).prop_map(|m| Mat4 { m })
}

proptest! {
    // transposing twice should always return the same starting mat4
    #[test]
    fn double_transpose(m in mat4()){
        prop_assert_eq!(m.transpose().transpose(), m)
    }
}
