#version 460

#include "../common.glsl"

layout(location = 0) in uint in_property_index;
layout(location = 1) in uint in_char_index;
layout(location = 2) in vec2 in_position;
layout(location = 3) in vec2 in_size;

layout(location = 0) out vec4 out_color;
layout(location = 1) out vec3 out_uvz;

void main() {
    const Property property = properties[in_property_index];

    const vec2 pos[4] = {
        vec2(0.0),
        vec2(0.0, in_size.y),
        vec2(in_size.x, 0.0),
        in_size
    };
    const float z_index = float(in_char_index);
    const vec2 uv[4] = {
        vec2(0.0),
        vec2(0.0, 1.0),
        vec2(1.0, 0.0),
        vec2(1.0)
    };
    const int vertex_index = gl_VertexIndex;

    gl_Position = vec4(remap(in_position + pos[vertex_index]), 0.0, 1.0);
    out_color = property.counter_text_color;
    out_uvz = vec3(uv[vertex_index], z_index);
}

