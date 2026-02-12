#version 460

#include "../common.glsl"

layout(location = 0) in uint in_property_index;

layout(location = 0) out vec4 out_color;

void main() {
    const Property property = properties[in_property_index];

    const vec2 key_position = property.key_position;
    const vec2 frame_size = vec2(property.width, property.height);
    const vec2 thickness = vec2(property.thickness);
    const vec2 inner_size = frame_size - thickness - thickness;

    const vec2 left_top = key_position + thickness;
    const vec2 left_bottom = left_top + vec2(0.0, inner_size.y);
    const vec2 right_top = left_top + vec2(inner_size.x, 0.0);
    const vec2 right_bottom = left_top + inner_size;
    const vec2 vertices[4] = { left_top, left_bottom, right_top, right_bottom };

    const vec2 position = remap(vertices[gl_VertexIndex]);

    gl_Position = vec4(position, 0.0, 1.0);
    out_color = property.pressed_color;
}

