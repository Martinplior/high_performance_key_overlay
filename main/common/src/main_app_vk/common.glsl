#ifndef COMMON_GLSL
#define COMMON_GLSL

struct ScreenSize {
    float width;
    float height;
};

struct Direction {
    uint v;
};

struct Property {
    vec4 pressed_color;
    vec4 frame_color;
    vec4 text_color;

    vec2 key_position;
    float width;
    float height;

    float thickness;
    float bar_speed;
    uint has_max_distance;
    float max_distance;

    uint has_fade;
    float fade_length;
    /// 0: up, 1: down, 2: left, 3: right
    Direction direction;
    float font_size;

    vec4 counter_text_color;

    uvec3 _padding;
    float counter_font_size;
};

layout(set = 0, binding = 0) uniform readonly Uniforms {
    ScreenSize screen_size;
} uniforms;

layout(set = 0, binding = 1) buffer readonly Buffer {
    Property properties[];
};

vec2 remap(const vec2 pos) {
    const vec2 screen_size_vec = vec2(uniforms.screen_size.width, uniforms.screen_size.height);
    const vec2 factor = vec2(2.0);
    const vec2 shift = vec2(-1.0);
    const vec2 r = pos / screen_size_vec * factor + shift;
    return r;
}

#endif // COMMON_GLSL
