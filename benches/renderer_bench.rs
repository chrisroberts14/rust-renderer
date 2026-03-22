/// To measure if any code improvements improved render time
use criterion::{Criterion, criterion_group, criterion_main};

use rust_renderer::{create_complex_scene, create_simple_scene};

fn bench_render_simple_scene(c: &mut Criterion) {
    let scene_create_return = create_simple_scene().unwrap();
    let mut scene = scene_create_return.scene;

    c.bench_function("render_simple", |b| {
        b.iter(|| {
            scene.render_scene();
        })
    });
}

fn bench_render_complex_scene(c: &mut Criterion) {
    let scene_create_return = create_complex_scene().unwrap();
    let mut scene = scene_create_return.scene;

    c.bench_function("render_complex", |b| {
        b.iter(|| {
            scene.render_scene();
        })
    });
}

fn bench_render_simple_scene_wire_frame(c: &mut Criterion) {
    let scene_create_return = create_simple_scene().unwrap();
    let mut scene = scene_create_return.scene;
    scene.settings.toggle_wire_frame_mode();

    c.bench_function("render_simple_wireframe", |b| {
        b.iter(|| {
            scene.render_scene();
        })
    });
}

fn bench_render_complex_scene_wire_frame(c: &mut Criterion) {
    let scene_create_return = create_complex_scene().unwrap();
    let mut scene = scene_create_return.scene;
    scene.settings.toggle_wire_frame_mode();

    c.bench_function("render_complex_wireframe", |b| {
        b.iter(|| {
            scene.render_scene();
        })
    });
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
criterion_group! {
    name = benches;
    config = Criterion::default()
        .with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench_render_simple_scene, bench_render_complex_scene, bench_render_simple_scene_wire_frame, bench_render_complex_scene_wire_frame
}

criterion_main!(benches);
