#ifndef COMMON_FS
#define COMMON_FS

const float SDF_AA_STEP_EDGE = 0.74;

float aa_step(const float edge, const float value) {
    const float rcp_sqrt2 = 0.70710678; // 1 / sqrt(2)
    const float df = length(vec2(dFdx(value), dFdy(value))) * rcp_sqrt2;
    return smoothstep(edge - df, edge + df, value);
}

#endif // COMMON_FS
