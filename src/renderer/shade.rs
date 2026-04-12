use crate::maths::vec3::Vec3;
use crate::scenes::lights::Light;
use std::sync::Arc;

pub(super) const SHININESS: i32 = 32;

/// Computes the Phong light multiplier [r, g, b] for a surface point.
/// Returns [1.0; 3] when there are no lights (unlit rendering).
pub fn shade(
    normal: Vec3,
    world_pos: Vec3,
    view_dir: Vec3,
    lights: &[Arc<dyn Light>],
    ambient: f32,
) -> [f32; 3] {
    if lights.is_empty() {
        return [1.0; 3];
    }
    let mut diffuse_rgb = [0.0f32; 3];
    let mut specular_rgb = [0.0f32; 3];
    for light in lights.iter() {
        let light_colour = light.colour_at(world_pos);
        let light_dir = light.direction_to(world_pos);

        let ndotl = normal.dot(light_dir).max(0.0);
        let diffuse = ndotl;

        let mut specular = 0.0;
        if ndotl > 0.0 {
            let reflect = normal * (2.0 * ndotl) - light_dir;
            specular = reflect.dot(view_dir).max(0.0).powi(SHININESS);
        }

        for i in 0..3 {
            diffuse_rgb[i] += diffuse * light_colour[i];
            specular_rgb[i] += specular * light_colour[i];
        }
    }
    let inv_ambient = 1.0 - ambient;
    [
        (ambient + inv_ambient * diffuse_rgb[0] + specular_rgb[0]).min(1.0),
        (ambient + inv_ambient * diffuse_rgb[1] + specular_rgb[1]).min(1.0),
        (ambient + inv_ambient * diffuse_rgb[2] + specular_rgb[2]).min(1.0),
    ]
}
