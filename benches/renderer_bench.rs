use std::path::PathBuf;

/// To measure if any code improvements improved render time
use criterion::{Criterion, criterion_group, criterion_main};

use rust_renderer::{create_from_file, scenes::scene::Scene};

const SIMPLE_SCENE_PATH: &str = "assets/scene_defs/simple.json";
const COMPLEX_SCENE_PATH: &str = "assets/scene_defs/complex.json";

fn simple_scene() -> Scene {
    create_from_file(PathBuf::from(SIMPLE_SCENE_PATH)).expect("Failed to load simple scene")
}

fn complex_scene() -> Scene {
    create_from_file(PathBuf::from(COMPLEX_SCENE_PATH)).expect("Failed to load complex scene")
}

fn bench_scene(c: &mut Criterion, name: &str, mut scene: Scene, wireframe: bool) {
    if wireframe {
        scene.toggle_wire_frame_mode();
    }

    c.bench_function(name, |b| {
        b.iter_batched(
            || scene.clone(), // setup: fresh scene per iteration
            |mut s| s.render_scene(),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_render_simple_scene(c: &mut Criterion) {
    bench_scene(c, "render_simple", simple_scene(), false);
}

fn bench_render_simple_scene_wire_frame(c: &mut Criterion) {
    bench_scene(c, "render_simple_wireframe", simple_scene(), true);
}

fn bench_render_complex_scene(c: &mut Criterion) {
    bench_scene(c, "render_complex", complex_scene(), false);
}

fn bench_render_complex_scene_wire_frame(c: &mut Criterion) {
    bench_scene(c, "render_complex_wireframe", complex_scene(), true);
}

// Flame graphs production doesn't compile on windows
#[cfg(target_os = "windows")]
criterion_group!(
    benches,
    bench_render_simple_scene,
    bench_render_complex_scene,
    bench_render_simple_scene_wire_frame,
    bench_render_complex_scene_wire_frame
);

#[cfg(not(target_os = "windows"))]
use pprof::criterion::{Output, PProfProfiler};

#[cfg(not(target_os = "windows"))]
criterion_group! {
    name = benches;
    config = Criterion::default()
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_render_simple_scene, bench_render_complex_scene, bench_render_simple_scene_wire_frame, bench_render_complex_scene_wire_frame
}

criterion_main!(benches);
