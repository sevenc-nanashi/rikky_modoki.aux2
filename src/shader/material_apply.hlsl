---$include "geometry.hlsl"
Texture2D<float4> original : register(t0);
Texture2D<float4> previous : register(t1);
Texture2D<float4> diffuseInput : register(t2);
Texture2D<float4> specularInput : register(t3);

float4 rikky_material_apply(float4 pos : SV_Position) : SV_Target {
    uint2 pixel = uint2(pos.xy);
    float4 source = original.Load(int3(pixel, 0));
    if (!affected(pixel)) return source;
    // extra[0].x: HQ partition (0 means one coefficient for the entire image).
    uint2 cell = extra[0].x > 0 ? uint2(float2(pixel) / extra[0].x) : uint2(0, 0);
    float3 diffuse = diffuseInput.Load(int3(cell, 0)).rgb;
    float3 specular = specularInput.Load(int3(cell, 0)).rgb;
    return float4(previous.Load(int3(pixel, 0)).rgb + source.rgb * diffuse + specular, source.a);
}
