use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use rust_renderer::file::scene_file::SceneFile;
use rust_renderer::renderer::gpu_raster_renderer::GpuRasterRenderer;
use rust_renderer::scenes::scene::Scene;

const SIMPLE_SCENE_PATH: &str = "assets/scene_defs/simple.json";
const COMPLEX_SCENE_PATH: &str = "assets/scene_defs/complex.json";

fn simple_scene() -> Scene {
    SceneFile::from_file(SIMPLE_SCENE_PATH, 800.0, 600.0).unwrap()
}

fn complex_scene() -> Scene {
    SceneFile::from_file(COMPLEX_SCENE_PATH, 800.0, 600.0).unwrap()
}

fn add_scene_benches(group: &mut BenchmarkGroup<WallTime>, name: &str, scene: &mut Scene) {
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

fn bench_gpu(c: &mut Criterion) {
    let mut group = c.benchmark_group("gpu");
    add_scene_benches(&mut group, "simple", &mut simple_scene());
    add_scene_benches(&mut group, "complex", &mut complex_scene());
    group.finish();
}

criterion_group!(benches, bench_gpu);
criterion_main!(benches);
