/// To measure if any code improvements improved render time
use criterion::{Criterion, criterion_group, criterion_main};

use rust_renderer::{create_complex_scene, create_simple_scene};

fn bench_render_simple_scene(c: &mut Criterion) {
    let (mut scene, _update_handle, _update_running) = create_simple_scene().unwrap();

    c.bench_function("render_simple", |b| {
        b.iter(|| {
            scene.render_scene();
        })
    });
}

fn bench_render_complex_scene(c: &mut Criterion) {
    let (mut scene, _update_handle, _update_running) = create_complex_scene().unwrap();

    c.bench_function("render_complex", |b| {
        b.iter(|| {
            scene.render_scene();
        })
    });
}

fn bench_render_simple_scene_wire_frame(c: &mut Criterion) {
    let (mut scene, _update_handle, _update_running) = create_simple_scene().unwrap();
    scene.settings.toggle_wire_frame_mode();

    c.bench_function("render_simple_wireframe", |b| {
        b.iter(|| {
            scene.render_scene();
        })
    });
}

fn bench_render_complex_scene_wire_frame(c: &mut Criterion) {
    let (mut scene, _update_handle, _update_running) = create_complex_scene().unwrap();
    scene.settings.toggle_wire_frame_mode();

    c.bench_function("render_complex_wireframe", |b| {
        b.iter(|| {
            scene.render_scene();
        })
    });
}

criterion_group!(
    benches,
    bench_render_simple_scene,
    bench_render_complex_scene,
    bench_render_simple_scene_wire_frame,
    bench_render_complex_scene_wire_frame
);
criterion_main!(benches);
