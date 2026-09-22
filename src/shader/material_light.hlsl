---$include "geometry.hlsl"
Texture2D<float4> lightTexture : register(t0);
RWTexture2D<float4> diffuseOutput : register(u0);
RWTexture2D<float4> specularOutput : register(u1);

// extra[0..7]: material_ex::constants; extra[8]: damping, partition, image width, image height.
float attenuation(float distanceSquared) {
    float damping = extra[8].x;
    return damping == 0 ? 1 : min(1, damping * damping / max(distanceSquared, 1e-12));
}

float cone(float3 direction, float3 outward, float cosine) {
    float axial = -dot(direction, outward);
    if (cosine >= 0.999999) return axial >= 0.999999 ? 1 : 0;
    return smoothstep(cosine, 1, axial);
}

void addLight(float3 worldPosition, float3 n, float3 view, float3 position, float3 color,
              float factor, inout float3 diffuse, inout float3 specular) {
    float3 delta = position - worldPosition;
    float distanceSquared = dot(delta, delta);
    if (distanceSquared < 1e-12 || factor <= 0) return;
    float3 l = delta * rsqrt(distanceSquared);
    float lambert = max(0, dot(n, l));
    if (lambert <= 0) return;
    float rv = saturate(dot(reflect(-l, n), view));
    float highlight = rv > 0 ? pow(rv, extra[1].w) : 0;
    float3 intensity = color * attenuation(distanceSquared) * factor;
    diffuse += intensity * lambert;
    specular += intensity * extra[2].rgb * highlight;
}

[numthreads(8, 8, 1)]
void rikky_material_light(uint2 id : SV_DispatchThreadID) {
    uint width, height;
    diffuseOutput.GetDimensions(width, height);
    if (id.x >= width || id.y >= height) return;
    float partition = extra[8].y;
    float2 uv = 0.5;
    if (partition > 0) {
        float2 start = float2(id) * partition;
        float2 end = min(start + partition, extra[8].zw);
        uv = (start + end) * 0.5 / extra[8].zw;
    }
    float3 worldPosition = worldAt(uv);
    float3 n = shadingNormal();
    float3 toEye = eye.xyz - worldPosition;
    float3 view = dot(toEye, toEye) > 1e-12 ? normalize(toEye) : 0;
    float3 diffuse = 0, specular = 0;
    int kind = (int)extra[0].w;
    if (kind < 2) {
        float factor = 1;
        if (kind == 1) {
            float3 delta = extra[0].xyz - worldPosition;
            float3 l = dot(delta, delta) > 1e-12 ? normalize(delta) : 0;
            factor = cone(l, extra[3].xyz, extra[3].w);
            if (extra[2].w != 0) factor += cone(l, extra[4].xyz, extra[4].w);
        }
        addLight(worldPosition, n, view, extra[0].xyz, extra[1].rgb, factor, diffuse, specular);
    } else {
        float2 size = float2(extra[5].w, extra[6].w);
        uint2 cells = uint2(ceil(size / extra[7].x));
        // Area weights keep total source power independent of subdivision.
        for (uint y = 0; y < cells.y; ++y) {
            for (uint x = 0; x < cells.x; ++x) {
                float2 lo = float2(x, y) * extra[7].x;
                float2 hi = min(lo + extra[7].x, size);
                float2 offset = (lo + hi - size) * 0.5;
                float3 position = extra[0].xyz + extra[5].xyz * offset.x + extra[6].xyz * offset.y;
                float3 delta = position - worldPosition;
                if (dot(delta, delta) < 1e-12) continue;
                float3 l = normalize(delta);
                float cosine = -dot(l, extra[3].xyz);
                if (extra[2].w != 0) cosine = abs(cosine);
                float angular = extra[3].w >= 0.999999 ? (cosine >= 0.999999 ? 1 : 0)
                    : smoothstep(extra[3].w, 1, cosine);
                float3 color = extra[7].z != 0 ? lightTexture.Load(int3(x, y, 0)).rgb : extra[1].rgb;
                float weight = (hi.x-lo.x) * (hi.y-lo.y) / (size.x*size.y);
                addLight(worldPosition, n, view, position, color, max(0, cosine) * angular * weight, diffuse, specular);
            }
        }
    }
    diffuseOutput[id] = float4(diffuse, 1);
    specularOutput[id] = float4(specular, 1);
}
