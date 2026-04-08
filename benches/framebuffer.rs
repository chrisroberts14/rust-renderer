use criterion::{Criterion, criterion_group, criterion_main};

use rust_renderer::framebuffer::Framebuffer;
use rust_renderer::scenes::camera::Camera;
use rust_renderer::scenes::texture::Texture;
use std::hint::black_box;

fn bench_draw_skybox(c: &mut Criterion) {
    let framebuffer = Framebuffer::new(800, 600);
    let texture = Texture::new(800, 600, [255, 255, 255, 255]);
    let camera = Camera::new(800.0, 600.0);

    let mut group = c.benchmark_group("draw_skybox");

    group.bench_function("draw_skybox", |b| {
        b.iter(|| black_box(framebuffer.draw_skybox(&texture, &camera)));
    });
}

criterion_group!(benches, bench_draw_skybox);
criterion_main!(benches);
