use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use rust_renderer::file::scene_file::SceneFile;
use rust_renderer::maths::vec3::Vec3;
use rust_renderer::renderer::gpu_raster_renderer::GpuRasterRenderer;
use rust_renderer::renderer::multi_thread_raster_renderer::MultiThreadRasterRenderer;
use rust_renderer::renderer::shade;
use rust_renderer::renderer::single_thread_raster_renderer::SingleThreadRasterRenderer;
use rust_renderer::scenes::lights::Light;
use rust_renderer::scenes::lights::pointlight::PointLight;
use rust_renderer::scenes::scene::Scene;
use std::sync::Arc;

const SIMPLE_SCENE_PATH: &str = "assets/scene_defs/simple.json";
const COMPLEX_SCENE_PATH: &str = "assets/scene_defs/complex.json";

fn simple_scene() -> Scene {
    SceneFile::from_file(SIMPLE_SCENE_PATH, 800.0, 600.0).unwrap()
}

fn complex_scene() -> Scene {
    SceneFile::from_file(COMPLEX_SCENE_PATH, 800.0, 600.0).unwrap()
}

fn add_single_thread_benches(group: &mut BenchmarkGroup<WallTime>, name: &str, scene: &mut Scene) {
    scene.settings.toggle_wire_frame_mode();

    group.bench_function(format!("{name}/solid"), |b| {
        b.iter_batched(
            || SingleThreadRasterRenderer::new(32),
            |r| scene.render_scene(&r),
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function(format!("{name}/wireframe"), |b| {
        b.iter_batched(
            || SingleThreadRasterRenderer::new(32),
            |r| scene.render_scene(&r),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn add_multi_thread_benches(group: &mut BenchmarkGroup<WallTime>, name: &str, scene: &mut Scene) {
    scene.settings.toggle_wire_frame_mode();

    group.bench_function(format!("{name}/solid"), |b| {
        b.iter_batched(
            || MultiThreadRasterRenderer::new(32),
            |r| scene.render_scene(&r),
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function(format!("{name}/wireframe"), |b| {
        b.iter_batched(
            || MultiThreadRasterRenderer::new(32),
            |r| scene.render_scene(&r),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn add_gpu_benches(group: &mut BenchmarkGroup<WallTime>, name: &str, scene: &mut Scene) {
    group.bench_function(format!("{name}/solid"), |b| {
        b.iter_batched(
            GpuRasterRenderer::new,
            |r| scene.render_scene(&r),
            criterion::BatchSize::SmallInput,
        );
    });

    scene.settings.toggle_wire_frame_mode();

    group.bench_function(format!("{name}/wireframe"), |b| {
        b.iter_batched(
            GpuRasterRenderer::new,
            |r| scene.render_scene(&r),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_single_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_thread");
    add_single_thread_benches(&mut group, "simple", &mut simple_scene());
    add_single_thread_benches(&mut group, "complex", &mut complex_scene());
    group.finish();
}

fn bench_multi_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_thread");
    add_multi_thread_benches(&mut group, "simple", &mut simple_scene());
    add_multi_thread_benches(&mut group, "complex", &mut complex_scene());
    group.finish();
}

fn bench_gpu(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu");
    add_gpu_benches(&mut group, "simple", &mut simple_scene());
    add_gpu_benches(&mut group, "complex", &mut complex_scene());
    group.finish();
}

fn bench_shade(c: &mut Criterion) {
    let normal = Vec3::new(0.0, 0.0, -1.0);
    let world_pos = Vec3::new(0.0, 0.0, -1.0);
    let view_dir = Vec3::new(0.0, 0.0, -1.0);
    let lights: Vec<Arc<dyn Light>> = vec![Arc::new(PointLight::new(
        Vec3::new(-10.0, 10.0, -10.0),
        [255.0, 255.0, 255.0],
        25.0,
    ))];
    let ambient = 32.0;

    let mut group = c.benchmark_group("shade");
    group.bench_function("shade", |b| {
        b.iter(|| shade(normal, world_pos, view_dir, &lights, ambient))
    });
}

criterion_group!(
    benches,
    bench_single_thread,
    bench_multi_thread,
    bench_gpu,
    bench_shade
);
criterion_main!(benches);
