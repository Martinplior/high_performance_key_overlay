#version 460

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0, rgba32f) uniform image2D overlay_image;

void main() {
    const ivec2 image_size = imageSize(overlay_image);
    const ivec2 coord = ivec2(gl_FragCoord.xy);
    if (any(greaterThanEqual(coord, image_size))) {
        discard;
    }
    out_color = imageLoad(overlay_image, coord);
}
