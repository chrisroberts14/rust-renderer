use criterion::{Criterion, criterion_group, criterion_main};
use rust_renderer::geometry::triangle::Triangle;
use rust_renderer::maths::vec2::Vec2;
use std::hint::black_box;

fn bench_contains_point(c: &mut Criterion) {
    let tri = Triangle::screen_triangle(
        Vec2::new(0.0, 0.0),
        Vec2::new(100.0, 0.0),
        Vec2::new(50.0, 100.0),
    );

    let mut group = c.benchmark_group("triangle_contains_point");

    group.bench_function("inside", |b| {
        b.iter(|| tri.contains_point(black_box(50.0), black_box(40.0)));
    });

    group.bench_function("outside", |b| {
        b.iter(|| tri.contains_point(black_box(200.0), black_box(200.0)));
    });

    group.finish();
}

criterion_group!(benches, bench_contains_point);
criterion_main!(benches);
