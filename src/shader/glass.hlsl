---$include "geometry.hlsl"
Texture2D<float4> original : register(t0);
Texture2D<float4> background : register(t1);
SamplerState backgroundSampler : register(s0);

// extra[0]: transmission RGB, unused
// extra[1]: refraction strength, virtual background distance, inverse zoom, lens
// extra[2]: horizontal sign, vertical sign, unused, unused
bool projectPoint(float3 worldPosition, out float2 xy) {
    float3 delta = worldPosition - eye.xyz;
    float depth = dot(delta, cameraForward.xyz);
    xy = 0;
    if (depth <= 0.0001) return false;
    float2 p = float2(dot(delta, cameraRight.xyz), -dot(delta, cameraUp.xyz)) * projection.x / depth;
    xy = float2(p.x * projection.y - p.y * projection.z, p.x * projection.z + p.y * projection.y);
    return true;
}

float2 lensUV(float2 uv) {
    const float pi = 3.141592653589793;
    float2 warped;
    if (extra[1].w == 1) {
        // Convex: compress source coordinates near the center to magnify it.
        warped = 0.5 * sin(pi * uv);
        warped = float2(uv.x > 0.5 ? 1 - warped.x : warped.x,
                        uv.y > 0.5 ? 1 - warped.y : warped.y);
    } else if (extra[1].w == 2) {
        // Concave: expand source coordinates near the center to shrink it.
        warped = 0.5 * (1 - cos(pi * uv));
    } else {
        return uv;
    }
    // Match the original lens profile and keep all four image edges fixed.
    return uv + (warped - uv) * sin(pi * uv.yx);
}

float4 rikky_glass(float4 pos : SV_Position, float2 uv : TEXCOORD) : SV_Target {
    float alpha = original.Load(int3(int2(pos.xy), 0)).a;
    uv = lensUV(uv);
    float3 worldPosition = worldAt(uv);
    if (extra[1].x > 0 && extra[1].y > 0) {
        float3 delta = worldPosition - eye.xyz;
        if (dot(delta, delta) < 1e-12) return 0;
        float3 incident = normalize(delta);
        float3 n = surface.xyz;
        if (dot(incident, n) > 0) n = -n;
        // Air -> glass. The existing 0..1 control maps to IOR 1..2.
        float3 ray = refract(incident, n, 1 / (1 + extra[1].x));
        float axial = dot(ray, cameraForward.xyz);
        if (axial <= 0.0001) return 0;
        worldPosition += ray * (extra[1].y / axial);
    }
    float2 xy, center;
    if (!projectPoint(worldPosition, xy) || !projectPoint(worldAt(0.5), center)) return 0;
    xy = center + (xy - center) * extra[1].z * extra[2].xy;
    uint width, height;
    background.GetDimensions(width, height);
    float3 rgb = background.Sample(backgroundSampler, xy / float2(width, height) + 0.5).rgb;
    // AviUtl2 buffers use premultiplied alpha, including fully transparent pixels.
    return float4(rgb * extra[0].rgb * alpha, alpha);
}
