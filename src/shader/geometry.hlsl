// Rust glass::constants: 11 float4, followed by up to 16 shader-specific float4.
cbuffer Constants : register(b0) {
    float4 corners[4];
    float4 surface;
    float4 eye;
    float4 cameraRight;
    float4 cameraUp;
    float4 cameraForward;
    float4 projection;
    float4 bounds;
    float4 extra[16];
};

float3 worldAt(float2 uv) {
    return lerp(lerp(corners[0].xyz, corners[1].xyz, uv.x),
                lerp(corners[3].xyz, corners[2].xyz, uv.x), uv.y);
}

bool affected(uint2 pixel) {
    return all(float2(pixel) >= bounds.xy) && all(float2(pixel) < bounds.zw);
}

float3 shadingNormal() {
    float3 n = surface.xyz;
    return dot(n, eye.xyz - worldAt(0.5)) < 0 ? -n : n;
}
