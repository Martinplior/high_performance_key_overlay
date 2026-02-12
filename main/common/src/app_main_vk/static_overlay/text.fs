#version 460

#extension GL_EXT_nonuniform_qualifier : enable

#include "../common.fs"

layout(location = 0) in vec4 in_color;
layout(location = 1) in vec3 in_uvz;

layout(location = 0) out vec4 out_color;

layout(set = 1, binding = 0) uniform sampler2D font_samplers[];

void main() {
    const float dist = texture(font_samplers[int(in_uvz.z)], in_uvz.xy).r;
    const float a = aa_step(SDF_AA_STEP_EDGE, dist);
    out_color = in_color * a;
}
