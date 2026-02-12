#version 460

#include "../common.glsl"

float calc_distance(const vec2 frag_coord, const Property property) {
    const vec2 key_position = property.key_position;
    const bool has_max_distance = bool(property.has_max_distance);
    const float max_distance = property.max_distance;

    const vec4 src_clip_vec = vec4(
        max_distance - key_position.y,
        key_position.y + property.height + max_distance,
        max_distance - key_position.x,
        key_position.x + property.width + max_distance
    );
    const bvec4 has_max_distance_vec = bvec4(has_max_distance);
    const ScreenSize screen_size = uniforms.screen_size;
    const vec4 fallback_vec = vec4(0.0, screen_size.height, 0.0, screen_size.width);
    const vec4 clip_vec = mix(fallback_vec, src_clip_vec, has_max_distance_vec);
    const vec4 frag_coord_vec = vec4(frag_coord.y, -frag_coord.y, frag_coord.x, -frag_coord.x);
    const vec4 distance_vec = clip_vec + frag_coord_vec;
    return distance_vec[property.direction.v];
}

layout(location = 0) flat in uint in_property_index;

layout(location = 0) out vec4 out_color;

void main() {
    const vec2 frag_coord = gl_FragCoord.xy;
    const Property property = properties[in_property_index];
    const bool has_fade = bool(property.has_fade);
    const float fade_length = property.fade_length;
    const vec4 transparent = vec4(0.0);
    const vec4 color = property.pressed_color;
    const float distance = calc_distance(frag_coord, property);
    const float fade_factor = mix(1.0, clamp(distance / fade_length, 0.0, 1.0), has_fade);

    out_color = mix(transparent, color, fade_factor);
}
