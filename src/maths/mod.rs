pub mod mat4;
pub mod vec2;
pub mod vec3;
pub mod vec4;

use vec2::Vec2;
use vec3::Vec3;
use vec4::Vec4;

macro_rules! impl_vec_ops {
    ($Type:ty, $($field:ident),+) => {
        impl ::std::ops::Add for $Type {
            type Output = $Type;
            fn add(self, other: $Type) -> $Type {
                Self { $($field: self.$field + other.$field),+ }
            }
        }
        impl ::std::ops::Sub for $Type {
            type Output = $Type;
            fn sub(self, other: $Type) -> $Type {
                Self { $($field: self.$field - other.$field),+ }
            }
        }
        impl ::std::ops::Mul<f32> for $Type {
            type Output = $Type;
            fn mul(self, factor: f32) -> $Type {
                Self { $($field: self.$field * factor),+ }
            }
        }
        impl ::std::ops::Div<f32> for $Type {
            type Output = $Type;
            fn div(self, factor: f32) -> $Type {
                Self { $($field: self.$field / factor),+ }
            }
        }
        impl ::std::ops::Neg for $Type {
            type Output = $Type;
            fn neg(self) -> $Type {
                Self { $($field: -self.$field),+ }
            }
        }
    };
}

impl_vec_ops!(Vec2, x, y);
impl_vec_ops!(Vec3, x, y, z);
impl_vec_ops!(Vec4, x, y, z, w);

#[cfg(test)]
mod proptests;
