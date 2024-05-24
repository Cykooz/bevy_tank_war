#import bevy_sprite::mesh2d_vertex_output::VertexOutput
// we can import items from shader modules in the assets folder with a quoted path
//#import "shaders/custom_material_import.wgsl"::COLOR_MULTIPLIER

@group(2) @binding(0) var<uniform> glow_color: vec4<f32>;
@group(2) @binding(1) var<uniform> glow_intensity: f32;
@group(2) @binding(2) var color_texture: texture_2d<f32>;
@group(2) @binding(3) var color_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(color_texture, color_sampler, mesh.uv);
    if (texel.a < 1.0) {
		let size: vec2f = vec2f(textureDimensions(color_texture, 0));
		let uv: vec2i = vec2i(mesh.uv * size - vec2f(0., 4.5));
		let base_x: i32 = uv.x;
		let base_y: i32 = uv.y;
		var sum: f32 = 0.;

		for (var n = 0; n < 9; n++) {
		    let uv_y = base_y + n;
			var h_sum: f32 = 0.;
            h_sum += textureLoad(color_texture, vec2i(base_x - 4, uv_y), 0).a;
            h_sum += textureLoad(color_texture, vec2i(base_x - 3, uv_y), 0).a;
            h_sum += textureLoad(color_texture, vec2i(base_x - 2, uv_y), 0).a;
            h_sum += textureLoad(color_texture, vec2i(base_x - 1, uv_y), 0).a;
            h_sum += textureLoad(color_texture, vec2i(base_x, uv_y), 0).a;
            h_sum += textureLoad(color_texture, vec2i(base_x + 1, uv_y), 0).a;
            h_sum += textureLoad(color_texture, vec2i(base_x + 2, uv_y), 0).a;
            h_sum += textureLoad(color_texture, vec2i(base_x + 3, uv_y), 0).a;
            h_sum += textureLoad(color_texture, vec2i(base_x + 4, uv_y), 0).a;
			sum += h_sum / 9.0;
		}

        let dst_alpha: f32 = (sum / 9.0) * glow_intensity;
        let res_alpha: f32 = texel.a + dst_alpha * (1.0 - texel.a);
        let res_rgb: vec3f = texel.rgb * texel.a + glow_color.rgb * dst_alpha * (1.0 - texel.a);
        return vec4f(res_rgb, res_alpha);
    }
    return texel;
}