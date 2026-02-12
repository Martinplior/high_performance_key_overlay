#ifndef COMMON_FS
#define COMMON_FS

const float SDF_AA_STEP_EDGE = 0.9;

float aa_step(const float edge, const float value) {
    const float rcp_sqrt2 = 0.70710678; // 1 / sqrt(2)
    const float df = length(vec2(dFdx(value), dFdy(value))) * rcp_sqrt2;
    return smoothstep(edge - df, edge + df, value);
}

float mix_step(
    const float edge,
    const float smooth_step_diff,
    const vec2 dst_size,
    const vec2 src_size,
    const float value
) {
    const vec2 ratio = dst_size / src_size;
    const float ratio_max = max(ratio.x, ratio.y);
    const float a_min = smoothstep(edge - smooth_step_diff, edge, value);
    const float a_max = aa_step(edge, value);
    return mix(a_min, a_max, ratio_max);
}

#endif // COMMON_FS
