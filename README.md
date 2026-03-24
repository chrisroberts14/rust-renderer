# rust-renderer

A CPU-based software rasterizer written in Rust, supporting Phong shading, texture mapping, and a tile-based rendering pipeline with both single-threaded and multi-threaded backends.

## Running

```bash
cargo run --release
```

By default the multi-threaded renderer is used. To select a renderer explicitly:

```bash
cargo run --release -- --renderer single-thread-raster
cargo run --release -- --renderer multi-thread-raster
```

## Benchmarks

```bash
cargo bench --bench single_thread_bench
cargo bench --bench multi_thread_bench
```