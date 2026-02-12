struct ScreenSize {
    width: f32,
    height: f32,
}

struct Direction {
    v: u32,
}

struct Property {
    pressed_color: vec4<f32>,
    key_position: vec2<f32>,
    width: f32,
    height: f32,
    /// 0: up, 1: down, 2: left, 3: right
    direction: Direction,
    bar_speed: f32,
    max_distance: f32,
    fade_length: f32,
    has_max_distance: u32,
    has_fade: u32,
}

struct BarRect {
    @location(0) property_index: u32,
    @location(1) begin_duration_secs: f32,
    @location(2) end_duration_secs: f32,
}

struct Uniforms {
    screen_size: ScreenSize,
}

struct Rangef {
    min: f32,
    max: f32,
}

struct Rect {
    min: vec2<f32>,
    max: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<storage, read> properties: array<Property>;

fn remap(pos: vec2<f32>) -> vec2<f32> {
    let screen_size_vec = vec2<f32>(uniforms.screen_size.width, uniforms.screen_size.height);
    let factor = vec2<f32>(2.0);
    let shift = vec2<f32>(-1.0);
    let r = pos / screen_size_vec * factor + shift;
    return vec2<f32>(r.x, -r.y);
}

fn head(bar_rect: BarRect, bar_speed: f32) -> f32 {
    return bar_rect.begin_duration_secs * bar_speed;
}

fn tail(bar_rect: BarRect, bar_speed: f32) -> f32 {
    return bar_rect.end_duration_secs * bar_speed;
}

fn up_down_x_range(property: Property) -> Rangef {
    let min = property.key_position.x;
    let max = min + property.width;
    return Rangef(min, max);
}

fn left_right_y_range(property: Property) -> Rangef {
    let min = property.key_position.y;
    let max = min + property.height;
    return Rangef(min, max);
}

fn Rect_from_x_y_ranges(x_range: Rangef, y_range: Rangef) -> Rect {
    let min = vec2<f32>(x_range.min, y_range.min);
    let max = vec2<f32>(x_range.max, y_range.max);
    return Rect(min, max);
}

fn Rect_from_BarRect(head: f32, tail: f32, property: Property, up_down_x_range: Rangef, left_right_y_range: Rangef) -> Rect {
    let key_position = property.key_position;
    let base_vec = vec4<f32>(key_position.y, key_position.y + property.height, key_position.x, key_position.x + property.width);
    let src_head_vec = vec4<f32>(-head, head, -head, head);
    let src_tail_vec = vec4<f32>(-tail, tail, -tail, tail);

    let head_vec = base_vec + src_head_vec;
    let tail_vec = base_vec + src_tail_vec;

    let up_head = head_vec.x;
    let down_head = head_vec.y;
    let left_head = head_vec.z;
    let right_head = head_vec.w;
    let up_tail = tail_vec.x;
    let down_tail = tail_vec.y;
    let left_tail = tail_vec.z;
    let right_tail = tail_vec.w;

    let up_rect = Rect_from_x_y_ranges(up_down_x_range, Rangef(up_head, up_tail));
    let down_rect = Rect_from_x_y_ranges(up_down_x_range, Rangef(down_tail, down_head));
    let left_rect = Rect_from_x_y_ranges(Rangef(left_head, left_tail), left_right_y_range);
    let right_rect = Rect_from_x_y_ranges(Rangef(right_tail, right_head), left_right_y_range);

    let rect_arr = array<Rect, 4>(up_rect, down_rect, left_rect, right_rect);

    return rect_arr[property.direction.v];
}

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
}

struct VertexOuptut {
    @builtin(position) position: vec4<f32>,
    @location(0) property_index: u32,
}

@vertex
fn vs_main(input: VertexInput, bar_rect: BarRect) -> VertexOuptut {
    let property = properties[bar_rect.property_index];
    let up_down_x_range = up_down_x_range(property);
    let left_right_y_range = left_right_y_range(property);
    let head = head(bar_rect, property.bar_speed);
    let tail = min(tail(bar_rect, property.bar_speed), head - 1.0);
    let rect = Rect_from_BarRect(head, tail, property, up_down_x_range, left_right_y_range);
    let vertexes = array<vec2<f32>, 4>(rect.min, vec2<f32>(rect.max.x, rect.min.y), vec2<f32>(rect.min.x, rect.max.y), rect.max);
    let position = remap(vertexes[input.vertex_index]);
    return VertexOuptut(vec4<f32>(position, 0.0, 1.0), bar_rect.property_index);
}

struct FragmentInput {
    @builtin(position) coord: vec4<f32>,
    @location(0) property_index: u32,
}

fn calc_distance(frag_coord: vec2<f32>, property: Property) -> f32 {
    let key_position = property.key_position;
    let has_max_distance = bool(property.has_max_distance);
    let max_distance = property.max_distance;

    let src_clip_vec = vec4<f32>(max_distance - key_position.y, key_position.y + property.height + max_distance, max_distance - key_position.x, key_position.x + property.width + max_distance);
    let has_max_distance_vec = vec4<bool>(has_max_distance);
    let screen_size = uniforms.screen_size;
    let fallback_vec = vec4<f32>(0.0, screen_size.height, 0.0, screen_size.width);
    let clip_vec = select(fallback_vec, src_clip_vec, has_max_distance_vec);
    let frag_coord_vec = vec4<f32>(frag_coord.y, -frag_coord.y, frag_coord.x, -frag_coord.x);
    let distance_vec = clip_vec + frag_coord_vec;
    return distance_vec[property.direction.v];
}

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    let frag_coord = input.coord.xy;
    let property = properties[input.property_index];
    let has_fade = bool(property.has_fade);
    let fade_length = property.fade_length;
    let transparent = vec4<f32>(0.0);
    let color = property.pressed_color;
    let distance = calc_distance(frag_coord, property);
    let fade_factor = select(1.0, clamp(distance / fade_length, 0.0, 1.0), has_fade);
    return mix(transparent, color, fade_factor);
}
