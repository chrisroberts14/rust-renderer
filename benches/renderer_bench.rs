/// To measure if any code improvements improved render time

use criterion::{criterion_group, criterion_main, Criterion};

use rust_renderer::create_scene;

fn bench_render(c: &mut Criterion) {
    let (mut scene, _update_handle, _update_running) = create_scene().unwrap();

    c.bench_function("render", |b| {
        b.iter(|| {
           scene.render_scene();
        })
    });
}

criterion_group!(benches, bench_render);
criterion_main!(benches);