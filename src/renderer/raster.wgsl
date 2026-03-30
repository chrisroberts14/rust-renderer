struct Uniforms {
    model: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    normal_mat: mat4x4<f32>,
    cam_pos: vec4<f32>,
    ambient: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

struct Light {
    position:  vec4<f32>,  // xyz = world position, w = intensity
    colour:    vec4<f32>,  // xyz = rgb
    direction: vec4<f32>,  // xyz = spot direction, w = cone_angle (0 = point light)
    falloff:   vec4<f32>,  // x = falloff_angle
};

struct LightBlock {
    lights: array<Light, 8>,
    light_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<uniform> lights: LightBlock;
@group(0) @binding(2) var tex: texture_2d<f32>;
@group(0) @binding(3) var tex_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) colour: vec4<f32>
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) colour: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world = u.model * vec4<f32>(in.position, 1.0);
    out.clip_pos = u.proj * u.view * world;
    out.world_pos = world.xyz;
    out.normal = normalize((u.normal_mat * vec4<f32>(in.normal, 0.0)).xyz);
    out.uv = in.uv;
    out.colour = in.colour;
    return out;
}

@fragment
fn fs_wireframe(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}

const SHININESS: f32 = 16.0;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSample(tex, tex_sampler, in.uv) * in.colour;

    if lights.light_count == 0u {
      return base;
    }

    let view_dir = normalize(u.cam_pos.xyz - in.world_pos);
    var diffuse  = vec3<f32>(0.0);
    var specular = vec3<f32>(0.0);

    for (var i = 0u; i < lights.light_count; i++) {
      let lpos       = lights.lights[i].position.xyz;
      let intensity  = lights.lights[i].position.w;
      let diff       = lpos - in.world_pos;
      let dist_sq    = dot(diff, diff);
      let dist_atten = intensity / (1.0 + dist_sq);

      // Cone attenuation — cone_angle == 0 means point light, skip cone test.
      var cone_atten = 1.0;
      let cone_angle = lights.lights[i].direction.w;
      if cone_angle > 0.0 {
          let spot_dir     = lights.lights[i].direction.xyz;
          let falloff_angle = lights.lights[i].falloff.x;
          let to_point     = normalize(in.world_pos - lpos);
          let angle        = acos(clamp(dot(spot_dir, to_point), -1.0, 1.0));
          if angle > cone_angle {
              cone_atten = 0.0;
          } else {
              let inner_angle = cone_angle - falloff_angle;
              if angle > inner_angle {
                  let t = (angle - inner_angle) / falloff_angle;
                  cone_atten = 1.0 - t * t * (3.0 - 2.0 * t);
              }
          }
      }

      let lcol  = lights.lights[i].colour.rgb * (dist_atten * cone_atten);
      let ldir  = normalize(diff);
      let ndotl = max(dot(in.normal, ldir), 0.0);
      diffuse  += ndotl * lcol;
      if ndotl > 0.0 {
          let refl  = reflect(-ldir, in.normal);
          specular += pow(max(dot(refl, view_dir), 0.0), SHININESS) * lcol;
      }
    }

    let inv_amb = 1.0 - u.ambient;
    let lit = clamp(u.ambient + inv_amb * diffuse + specular, vec3(0.0), vec3(1.0));
    return vec4<f32>(base.rgb * lit, base.a);
}
