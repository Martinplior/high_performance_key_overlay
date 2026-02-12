#version 460

#include "../common.glsl"

struct BarRect {
    uint property_index;
    float begin_duration_secs;
    float end_duration_secs;
};

struct Rangef {
    float min;
    float max;
};

struct Rect {
    vec2 min;
    vec2 max;
};

float head(const BarRect bar_rect, const float bar_speed) {
    return bar_rect.begin_duration_secs * bar_speed;
}

float tail(const BarRect bar_rect, const float bar_speed)  {
    return bar_rect.end_duration_secs * bar_speed;
}

Rangef up_down_x_range(const Property property) {
    float min = property.key_position.x;
    float max = min + property.width;
    return Rangef(min, max);
}

Rangef left_right_y_range(const Property property) {
    float min = property.key_position.y;
    float max = min + property.height;
    return Rangef(min, max);
}

Rect Rect_from_x_y_ranges(const Rangef x_range, const Rangef y_range) {
    const vec2 min = vec2(x_range.min, y_range.min);
    const vec2 max = vec2(x_range.max, y_range.max);
    return Rect(min, max);
}

Rect Rect_from_BarRect(
    const float head,
    const float tail,
    const Property property,
    const Rangef up_down_x_range,
    const Rangef left_right_y_range
) {
    const vec2 key_position = property.key_position;
    const vec4 base_vec = vec4(
        key_position.y,
        key_position.y + property.height,
        key_position.x,
        key_position.x + property.width
    );
    const vec4 src_head_vec = vec4(-head, head, -head, head);
    const vec4 src_tail_vec = vec4(-tail, tail, -tail, tail);

    const vec4 head_vec = base_vec + src_head_vec;
    const vec4 tail_vec = base_vec + src_tail_vec;

    const float up_head = head_vec.x;
    const float down_head = head_vec.y;
    const float left_head = head_vec.z;
    const float right_head = head_vec.w;
    const float up_tail = tail_vec.x;
    const float down_tail = tail_vec.y;
    const float left_tail = tail_vec.z;
    const float right_tail = tail_vec.w;

    const Rect up_rect = Rect_from_x_y_ranges(up_down_x_range, Rangef(up_head, up_tail));
    const Rect down_rect = Rect_from_x_y_ranges(up_down_x_range, Rangef(down_tail, down_head));
    const Rect left_rect = Rect_from_x_y_ranges(Rangef(left_head, left_tail), left_right_y_range);
    const Rect right_rect = Rect_from_x_y_ranges(
        Rangef(right_tail, right_head),
        left_right_y_range
    );

    const Rect rect_arr[4] = { up_rect, down_rect, left_rect, right_rect };

    return rect_arr[property.direction.v];
}

layout(location = 0) in uint in_property_index;
layout(location = 1) in float in_begin_duration_secs;
layout(location = 2) in float in_end_duration_secs;

layout(location = 0) out uint out_property_index;

void main() {
    const BarRect bar_rect = BarRect(
        in_property_index,
        in_begin_duration_secs,
        in_end_duration_secs
    );
    const Property property = properties[bar_rect.property_index];
    const Rangef up_down_x_range = up_down_x_range(property);
    const Rangef left_right_y_range = left_right_y_range(property);
    const float head = head(bar_rect, property.bar_speed);
    const float tail = min(tail(bar_rect, property.bar_speed), head - 1.0);
    const Rect rect = Rect_from_BarRect(head, tail, property, up_down_x_range, left_right_y_range);
    const vec2 vertexes[4] = {
        rect.min,
        vec2(rect.max.x, rect.min.y),
        vec2(rect.min.x, rect.max.y),
        rect.max
    };
    const vec2 position = remap(vertexes[gl_VertexIndex]);

    gl_Position = vec4(position, 0.0, 1.0);
    out_property_index = bar_rect.property_index;
}

