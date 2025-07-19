#version 460

#include "../common.glsl"

layout(location = 0) out vec4 out_color;

void main() {
    const Property property = properties[gl_InstanceIndex];

    const vec2 key_position = property.key_position;
    const vec2 frame_size = vec2(property.width, property.height);
    const vec2 thickness = vec2(property.thickness);
    const vec2 inner_size = frame_size - thickness - thickness;

    const vec2 outer_left_top = key_position;
    const vec2 outer_left_bottom = outer_left_top + vec2(0.0, frame_size.y);
    const vec2 outer_right_top = outer_left_top + vec2(frame_size.x, 0.0);
    const vec2 outer_right_bottom = outer_left_top + frame_size;

    const vec2 inner_left_top = outer_left_top + thickness;
    const vec2 inner_left_bottom = inner_left_top + vec2(0.0, inner_size.y);
    const vec2 inner_right_top = inner_left_top + vec2(inner_size.x, 0.0);
    const vec2 inner_right_bottom = inner_left_top + inner_size;

    const vec2 vertices[10] = {
        outer_left_top,
        inner_left_top,
        outer_left_bottom,
        inner_left_bottom,
        outer_right_bottom,
        inner_right_bottom,
        outer_right_top,
        inner_right_top,
        outer_left_top,
        inner_left_top,
    };

    const vec2 position = remap(vertices[gl_VertexIndex]);

    gl_Position = vec4(position, 0.0, 1.0);
    out_color = property.frame_color;
}

