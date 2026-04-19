use rust_renderer::file::scene_file::SceneFile;
use rust_renderer::renderer::cpu::{MultiThreadRasterRenderer, SingleThreadRasterRenderer};
use rust_renderer::renderer::vulkan::VulkanRenderer;
use rust_renderer::renderer::wgsl::GpuRasterRenderer;
use rust_renderer::scenes::scene::Scene;
use std::time::{Duration, Instant};

const SCENE_PATH: &str = "assets/scene_defs/complex.json";
const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 600.0;
const WARMUP: Duration = Duration::from_millis(200);
const MEASURE: Duration = Duration::from_secs(1);

fn load_scene() -> Scene {
    SceneFile::from_file(SCENE_PATH, WIDTH, HEIGHT).expect("failed to load scene")
}

fn run(name: &str, scene: &mut Scene, mut render: impl FnMut(&mut Scene)) {
    // Warmup: let GPU drivers / JIT settle before we measure.
    let warmup_end = Instant::now() + WARMUP;
    while Instant::now() < warmup_end {
        render(scene);
    }

    let start = Instant::now();
    let mut frames = 0u32;
    while start.elapsed() < MEASURE {
        render(scene);
        frames += 1;
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!("{:<22} {:>5} fps  ({:.2}s)", name, frames, elapsed);
}

fn main() {
    println!(
        "Frame benchmark — {}ms warmup + {}s measure — {} @ {}×{}",
        WARMUP.as_millis(),
        MEASURE.as_secs(),
        SCENE_PATH,
        WIDTH as u32,
        HEIGHT as u32,
    );
    println!("{}", "─".repeat(52));

    run("single_thread", &mut load_scene(), |s| {
        s.render_scene(&SingleThreadRasterRenderer::new(32));
    });

    run("multi_thread", &mut load_scene(), |s| {
        s.render_scene(&MultiThreadRasterRenderer::new(32));
    });

    let gpu = GpuRasterRenderer::new();
    run("wgsl_gpu", &mut load_scene(), |s| {
        s.render_scene(&gpu);
    });

    match VulkanRenderer::new() {
        Ok(vk) => run("vulkan", &mut load_scene(), |s| {
            s.render_scene(&vk);
        }),
        Err(e) => println!("{:<22} unavailable: {e}", "vulkan"),
    }
}
