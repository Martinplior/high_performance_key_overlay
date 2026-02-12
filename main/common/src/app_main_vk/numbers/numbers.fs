#version 460

#include "../common.fs"

layout(location = 0) in vec4 in_color;
layout(location = 1) in vec2 in_size;
layout(location = 2) in vec3 in_uvz;

layout(location = 0) out vec4 out_color;

layout(set = 1, binding = 0) uniform sampler2D font_samplers[10];

void main() {
    const int font_index = int(in_uvz.z);
    const float dist = texture(font_samplers[font_index], in_uvz.xy).r;
    const float a = mix_step(
        SDF_AA_STEP_EDGE,
        0.1,
        in_size,
        textureSize(font_samplers[font_index], 0),
        dist
    );
    out_color = in_color * a;
}
