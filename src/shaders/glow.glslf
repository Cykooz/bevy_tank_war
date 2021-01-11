#version 150 core

uniform sampler2D t_Texture;
in vec2 v_Uv;
out vec4 Target0;

layout (std140) uniform GlowParams {
    vec3 glow_color;
    float glow_intensity;
};

void main() {
    Target0 = texture(t_Texture, v_Uv);
    if (Target0.a < 1.0) {
        ivec2 size = textureSize(t_Texture, 0);

        float uv_x = v_Uv.x * size.x;
        float uv_y = v_Uv.y * size.y;

        float sum = 0.0;
        for (int n = 0; n < 9; ++n) {
            uv_y = (v_Uv.y * size.y) + float(n - 4.5);
            float h_sum = 0.0;
            h_sum += texelFetch(t_Texture, ivec2(uv_x - 4.0, uv_y), 0).a;
            h_sum += texelFetch(t_Texture, ivec2(uv_x - 3.0, uv_y), 0).a;
            h_sum += texelFetch(t_Texture, ivec2(uv_x - 2.0, uv_y), 0).a;
            h_sum += texelFetch(t_Texture, ivec2(uv_x - 1.0, uv_y), 0).a;
            h_sum += texelFetch(t_Texture, ivec2(uv_x, uv_y), 0).a;
            h_sum += texelFetch(t_Texture, ivec2(uv_x + 1.0, uv_y), 0).a;
            h_sum += texelFetch(t_Texture, ivec2(uv_x + 2.0, uv_y), 0).a;
            h_sum += texelFetch(t_Texture, ivec2(uv_x + 3.0, uv_y), 0).a;
            h_sum += texelFetch(t_Texture, ivec2(uv_x + 4.0, uv_y), 0).a;
            sum += h_sum / 9.0;
        }

        float dst_alpha = (sum / 9.0) * glow_intensity;
        float res_alpha = Target0.a + dst_alpha * (1.0 - Target0.a);
        vec3 res_rgb = Target0.rgb * Target0.a + glow_color * dst_alpha * (1.0 - Target0.a);
        Target0 = vec4(res_rgb, res_alpha);
    }
}
