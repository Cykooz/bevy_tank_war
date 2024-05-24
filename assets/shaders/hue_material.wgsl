#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> hue_offset: f32;
@group(2) @binding(1) var color_texture: texture_2d<f32>;
@group(2) @binding(2) var color_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(color_texture, color_sampler, mesh.uv);
    return rotate_hue(texel, hue_offset);
}

fn rotate_hue(rgba: vec4f, offset: f32) -> vec4f {
    var hsv = rgb2hsv(rgba.rgb);
    hsv.x = (hsv.x + offset) % 1.0;
    return vec4f(hsv2rgb(hsv), rgba.a);
}

fn rgb2hsv(c: vec3f) -> vec3f {
    let K = vec4f(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4f(c.bg, K.wz), vec4f(c.gb, K.xy), step(c.b, c.g));
    let q = mix(vec4f(p.xyw, c.r), vec4f(c.r, p.yzx), step(p.x, c.r));

    let d: f32 = q.x - min(q.w, q.y);
    let e: f32 = 1.0e-10;
    return vec3f(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

fn hsv2rgb(c: vec3f) -> vec3f {
    let K = vec4f(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p: vec3f = abs(fract(c.rrr + K.xyz) * 6.0 - K.www);
    return c.b * mix(K.xxx, clamp(p - K.xxx, vec3f(0.0), vec3f(1.0)), c.g);
}
