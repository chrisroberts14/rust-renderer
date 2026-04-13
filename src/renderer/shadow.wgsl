/// Depth-only pass rendered from each light's point of view to produce a shadow map.
/// No fragment shader — the hardware writes depth automatically from clip-space position.

struct ShadowPassUniforms {
    light_vp: mat4x4<f32>,
    model:    mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> su: ShadowPassUniforms;

@vertex
fn vs_shadow(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return su.light_vp * su.model * vec4<f32>(position, 1.0);
}
