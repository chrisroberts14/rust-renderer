@group(0) @binding(0) var t_frame: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;

struct VertOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> }

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertOut {
  // Full-screen triangle trick
  var uv = vec2<f32>(f32((idx << 1u) & 2u), f32(idx & 2u));
  return VertOut(vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0), vec2<f32>(uv.x, 1.0 - uv.y));
}

@fragment
fn fs_main(in: VertOut) -> @location(0) vec4<f32> {
  return textureSample(t_frame, s, in.uv);
}