//! The GLSL shaders used in the vulkan renderer
//!
//! These could be defined in separate files but the vulkano `shader!` macro
//! means we can define them easily in the Rust code like this.

/// Vertex shader
/// Runs once per vertex.
/// Main job is to figure out where a vertex should go in screen space
pub(crate) mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450

            // Inputs
            // This must match the `VulkanVertex` Struct
            layout(location = 0) in vec3 position;
            layout(location = 1) in vec3 normal;
            layout(location = 2) in vec4 color;

            // Outputs
            // These are what are passed to the fragment shader
            layout(location = 0) out vec3 frag_world_pos;
            layout(location = 1) out vec3 frag_normal;
            layout(location = 2) out vec4 frag_color;

            // This is global data that comes from the Rust code
            layout(set = 0, binding = 0) uniform Uniforms {
                mat4 model;
                mat4 view;
                mat4 proj;
                mat4 normal_mat;
                vec4 cam_pos;
                float ambient;
            };

            void main() {
                // Transform the position of the vertex to world space
                vec4 world_pos = model * vec4(position, 1.0);
                frag_world_pos = world_pos.xyz;
                frag_normal = normalize((normal_mat * vec4(normal, 0.0)).xyz);
                frag_color = color;
                // Final screen position
                gl_Position = proj * view * world_pos;
                // Vulkan NDC has Y+ pointing down; flip to match scene conventions.
                gl_Position.y = -gl_Position.y;
            }
        ",
    }
}

/// Fragment shader
/// This runs once per pixel of every triangle
/// Job is to figure out what colour a given pixel should be
pub(crate) mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450

            // Inputs
            // Come from the vertex shader
            layout(location = 0) in vec3 frag_world_pos;
            layout(location = 1) in vec3 frag_normal;
            layout(location = 2) in vec4 frag_color;

            // Output
            // This the final colour the pixel will be on the screen
            layout(location = 0) out vec4 out_color;

            // Same as in the vertex shader
            // This is data from the Rust code
            layout(set = 0, binding = 0) uniform Uniforms {
                mat4 model;
                mat4 view;
                mat4 proj;
                mat4 normal_mat;
                vec4 cam_pos;
                float ambient;
            };

            struct Light {
                vec4 position;   // xyz = world pos, w = intensity
                vec4 color;      // xyz = rgb, w = unused
                vec4 direction;  // xyz = spot dir, w = cone_angle (0 = point)
                vec4 falloff;    // x = falloff_angle
            };

            // An array of lights
            // At the moment this is capped at 8 as it needs to have a set length at compile time
            layout(set = 0, binding = 1) uniform LightBlock {
                Light lights[8];
                uint light_count;
            };

            void main() {
                vec3 n = normalize(frag_normal);
                vec4 base = frag_color;

                // Short circuit for if there are no lights
                if (light_count == 0u) {
                    out_color = base;
                    return;
                }

                vec3 view_dir = normalize(cam_pos.xyz - frag_world_pos);
                vec3 diffuse  = vec3(0.0);
                vec3 specular = vec3(0.0);

                for (uint i = 0u; i < light_count; i++) {
                    vec3  lpos        = lights[i].position.xyz;
                    float intensity   = lights[i].position.w;
                    vec3  diff_vec    = lpos - frag_world_pos;
                    float dist_sq     = dot(diff_vec, diff_vec);
                    float dist_atten  = intensity / (1.0 + dist_sq);

                    float cone_atten  = 1.0;
                    float cone_angle  = lights[i].direction.w;
                    if (cone_angle > 0.0) {
                        vec3  spot_dir      = lights[i].direction.xyz;
                        float falloff_angle = lights[i].falloff.x;
                        vec3  to_point      = normalize(frag_world_pos - lpos);
                        float angle         = acos(clamp(dot(spot_dir, to_point), -1.0, 1.0));
                        if (angle > cone_angle) {
                            cone_atten = 0.0;
                        } else {
                            float inner_angle = cone_angle - falloff_angle;
                            if (angle > inner_angle) {
                                float t = (angle - inner_angle) / falloff_angle;
                                cone_atten = 1.0 - t * t * (3.0 - 2.0 * t);
                            }
                        }
                    }

                    vec3  lcol  = lights[i].color.xyz * (dist_atten * cone_atten);
                    vec3  ldir  = normalize(diff_vec);
                    float ndotl = max(dot(n, ldir), 0.0);
                    diffuse    += ndotl * lcol;
                    if (ndotl > 0.0) {
                        vec3 refl = reflect(-ldir, n);
                        // The 32 here is the shininess
                        // TODO: Change this to a property
                        specular += pow(max(dot(refl, view_dir), 0.0), 32.0) * lcol;
                    }
                }

                float inv_amb = 1.0 - ambient;
                vec3  lit = clamp(vec3(ambient) + inv_amb * diffuse + specular, 0.0, 1.0);
                out_color = vec4(base.rgb * lit, base.a);
            }
        ",
    }
}
