use crate::geometry::cube::Cube;
use crate::geometry::mesh::Mesh;
use crate::geometry::object::Object;
use crate::geometry::sphere::Sphere;
use crate::geometry::transform::Transform;
use crate::maths::vec3::Vec3;
use crate::scenes::lights::Light;
use crate::scenes::lights::pointlight::PointLight;
use crate::scenes::material::Material;
use crate::scenes::scene::Scene;
use proptest::prelude::*;
use std::f32::consts::TAU;
use std::sync::Arc;

prop_compose! {
  fn transform()(
      px in -10.0f32..10.0, py in -10.0f32..10.0, pz in -10.0f32..10.0,
      rx in 0.0f32..TAU, ry in 0.0f32..TAU, rz in 0.0f32..TAU,
      sx in 0.1f32..5.0, sy in 0.1f32..5.0, sz in 0.1f32..5.0,
  ) -> Transform {
      Transform { position: Vec3::new(px, py, pz), rotation: Vec3::new(rx, ry, rz), scale: Vec3::new(sx, sy, sz) }
  }
}

fn mesh() -> impl Strategy<Value = Mesh> {
    prop_oneof![
        (0.1f32..5.0).prop_map(|s| Cube::mesh(s)),
        (0.1f32..3.0, 4u32..8, 4u32..8).prop_map(|(r, st, sl)| Sphere::mesh(r, st, sl)),
    ]
}

fn material() -> impl Strategy<Value = Material> {
    (any::<u8>(), any::<u8>(), any::<u8>(), any::<u8>())
        .prop_map(|(r, g, b, a)| Material::Color([r, g, b, a]))
}

prop_compose! {
    fn object()(mesh in mesh(), transform in transform(), material in material()) -> Object {
        Object::new(mesh, transform, material)
    }
}

fn light() -> impl Strategy<Value = Arc<dyn Light>> {
    (-10.0f32..10.0, -10.0f32..10.0, -10.0f32..10.0, 0.1f32..5.0).prop_map(
        |(x, y, z, intensity)| {
            Arc::new(PointLight::new(
                Vec3::new(x, y, z),
                [1.0, 1.0, 1.0],
                intensity,
            )) as Arc<dyn Light>
        },
    )
}

fn scene() -> impl Strategy<Value = Scene> {
    (
        64u32..256,
        64u32..256,
        proptest::collection::vec(object(), 0..4),
        proptest::collection::vec(light(), 0..3),
        0.0f32..1.0,
    )
        .prop_map(|(width, height, objects, lights, ambient)| {
            Scene::new(width as f32, height as f32, objects, lights, ambient)
        })
        .prop_filter("camera must not start inside any object", |scene| {
            !scene.is_point_inside_any_object(&scene.camera.position)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::multi_thread_raster_renderer::MultiThreadRasterRenderer;
    use crate::renderer::single_thread_raster_renderer::SingleThreadRasterRenderer;

    proptest! {
        #[test]
        fn single_thread_renders_without_panic(mut scene in scene()) {
            let renderer = SingleThreadRasterRenderer::new(32);
            scene.render_scene(&renderer);
        }

        #[test]
        fn multi_thread_renders_without_panic(mut scene in scene()) {
            let renderer = MultiThreadRasterRenderer::new(32);
            scene.render_scene(&renderer);
        }

        #[test]
        fn single_and_multi_thread_produce_identical_output(mut scene in scene()) {
            let single = SingleThreadRasterRenderer::new(32);
            let multi = MultiThreadRasterRenderer::new(32);

            scene.render_scene(&single);
            let single_pixels = scene.framebuffer.as_bytes().to_vec();

            scene.render_scene(&multi);
            let multi_pixels = scene.framebuffer.as_bytes().to_vec();

            prop_assert_eq!(single_pixels, multi_pixels);
        }
    }
}
