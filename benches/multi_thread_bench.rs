use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use rust_renderer::renderer::multi_thread_raster_renderer::MultiThreadRasterRenderer;
use rust_renderer::{create_from_file, scenes::scene::Scene};
use std::path::PathBuf;
use std::sync::Arc;

const SIMPLE_SCENE_PATH: &str = "assets/scene_defs/simple.json";
const COMPLEX_SCENE_PATH: &str = "assets/scene_defs/complex.json";

fn simple_scene() -> Scene {
    create_from_file(
        PathBuf::from(SIMPLE_SCENE_PATH),
        Some(Arc::new(MultiThreadRasterRenderer)),
    )
    .expect("Failed to load simple scene")
}

fn complex_scene() -> Scene {
    create_from_file(
        PathBuf::from(COMPLEX_SCENE_PATH),
        Some(Arc::new(MultiThreadRasterRenderer)),
    )
    .expect("Failed to load complex scene")
}

fn add_scene_benches(group: &mut BenchmarkGroup<WallTime>, name: &str, scene: Scene) {
    let mut wireframe = scene.clone();
    wireframe.toggle_wire_frame_mode();

    group.bench_function(format!("{name}/solid"), |b| {
        b.iter_batched(
            || scene.clone(),
            |mut s| s.render_scene(),
            criterion::BatchSize::SmallInput,
        )
    });

    group.bench_function(format!("{name}/wireframe"), |b| {
        b.iter_batched(
            || wireframe.clone(),
            |mut s| s.render_scene(),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_multi_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_thread");
    add_scene_benches(&mut group, "simple", simple_scene());
    add_scene_benches(&mut group, "complex", complex_scene());
    group.finish();
}

#[cfg(target_os = "windows")]
criterion_group!(benches, bench_multi_thread);

#[cfg(not(target_os = "windows"))]
use pprof::criterion::{Output, PProfProfiler};

#[cfg(not(target_os = "windows"))]
criterion_group! {
    name = benches;
    config = Criterion::default()
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_multi_thread
}

criterion_main!(benches);
