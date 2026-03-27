use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use rust_renderer::file::scene_file::SceneFile;
use rust_renderer::renderer::single_thread_raster_renderer::SingleThreadRasterRenderer;
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

fn add_scene_benches(group: &mut BenchmarkGroup<WallTime>, name: &str, scene: &mut Scene) {
    scene.toggle_wire_frame_mode();

    group.bench_function(format!("{name}/solid"), |b| {
        b.iter_batched(
            || (Arc::new(SingleThreadRasterRenderer)),
            |r| scene.render_scene(r),
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function(format!("{name}/wireframe"), |b| {
        b.iter_batched(
            || Arc::new(SingleThreadRasterRenderer),
            |r| scene.render_scene(r),
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_single_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_thread");
    add_scene_benches(&mut group, "simple", &mut simple_scene());
    add_scene_benches(&mut group, "complex", &mut complex_scene());
    group.finish();
}

#[cfg(target_os = "windows")]
criterion_group!(benches, bench_single_thread);

#[cfg(not(target_os = "windows"))]
use pprof::criterion::{Output, PProfProfiler};

#[cfg(not(target_os = "windows"))]
criterion_group! {
    name = benches;
    config = Criterion::default()
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_single_thread
}

criterion_main!(benches);
