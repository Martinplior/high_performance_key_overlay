struct ScreenSize {
    width: f32,
    height: f32,
}

struct Direction {
    v: u32,
}

const Direction_UP: Direction = Direction(0);
const Direction_DOWN: Direction = Direction(1);
const Direction_LEFT: Direction = Direction(2);
const Direction_RIGHT: Direction = Direction(3);

struct Property {
    pressed_color: vec4<f32>,
    key_position: vec2<f32>,
    width: f32,
    height: f32,
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

fn bar_pos_remap_up(pos: f32, property: Property) -> f32 {
    return property.key_position.y + pos;
}

fn bar_pos_remap_down(pos: f32, property: Property) -> f32 {
    return property.key_position.y + property.height + pos;
}

fn bar_pos_remap_left(pos: f32, property: Property) -> f32 {
    return property.key_position.x + pos;
}

fn bar_pos_remap_right(pos: f32, property: Property) -> f32 {
    return property.key_position.x + property.width + pos;
}

fn Rect_from_x_y_ranges(x_range: Rangef, y_range: Rangef) -> Rect {
    let min = vec2<f32>(x_range.min, y_range.min);
    let max = vec2<f32>(x_range.max, y_range.max);
    return Rect(min, max);
}

fn Rect_from_BarRect(
    head: f32,
    tail: f32,
    property: Property,
    up_down_x_range: Rangef,
    left_right_y_range: Rangef
) -> Rect {
    let up_head = bar_pos_remap_up(-head, property);
    let up_tail = bar_pos_remap_up(-tail, property);
    let down_head = bar_pos_remap_down(head, property);
    let down_tail = bar_pos_remap_down(tail, property);
    let left_head = bar_pos_remap_left(-head, property);
    let left_tail = bar_pos_remap_left(-tail, property);
    let right_head = bar_pos_remap_right(head, property);
    let right_tail = bar_pos_remap_right(tail, property);

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
    let rect = Rect_from_BarRect(
        head,
        tail,
        property,
        up_down_x_range,
        left_right_y_range
    );
    let vertexes = array<vec2<f32>, 4>(
        vec2<f32>(rect.min.x, rect.min.y),
        vec2<f32>(rect.max.x, rect.min.y),
        vec2<f32>(rect.min.x, rect.max.y),
        vec2<f32>(rect.max.x, rect.max.y)
    );
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

    let clip_up = select(0.0, key_position.y - max_distance, has_max_distance);
    let clip_down = select(
        uniforms.screen_size.height,
        key_position.y + property.height + max_distance,
        has_max_distance
    );
    let clip_left = select(0.0, key_position.x - max_distance, has_max_distance);
    let clip_right = select(
        uniforms.screen_size.width,
        key_position.x + property.width + max_distance,
        has_max_distance
    );

    let up_distance = frag_coord.y - clip_up;
    let down_distance = clip_down - frag_coord.y;
    let left_distance = frag_coord.x - clip_left;
    let right_distance = clip_right - frag_coord.x;

    let distance_arr = array<f32, 4>(up_distance, down_distance, left_distance, right_distance);

    return distance_arr[property.direction.v];
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
