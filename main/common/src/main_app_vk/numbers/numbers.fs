#version 460

layout(location = 0) in vec4 in_color;
layout(location = 1) in vec3 in_uvz;

layout(location = 0) out vec4 out_color;

layout(set = 1, binding = 0) uniform sampler2D font_samplers[10];

void main() {
    const float a = texture(font_samplers[int(in_uvz.z)], in_uvz.xy).r;
    out_color = in_color * a;
}
