#version 150 core
precision highp float;

uniform sampler2D t_Texture;
in vec2 v_Uv;
out vec4 Target0;

layout (std140) uniform HueParams {
    float hue_offset;
};

vec3 rgb2hsv(vec3 c)
{
    vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));

    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

vec3 hsv2rgb(vec3 c)
{
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

void main() {
    vec4 v_TexelColor = texture(t_Texture, v_Uv);
    vec3 v_Rgb = v_TexelColor.rgb;
    vec3 v_Hsv = rgb2hsv(v_Rgb).xyz;
    v_Hsv.x = mod(v_Hsv.x + hue_offset, 1.0);
    v_Rgb = hsv2rgb(v_Hsv);
    Target0 = vec4(v_Rgb, v_TexelColor.w);
}
