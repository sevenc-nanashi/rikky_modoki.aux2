---$include "geometry.hlsl"
Texture2D<float4> original : register(t0);

float4 rikky_material_base(float4 pos : SV_Position) : SV_Target {
    uint2 pixel = uint2(pos.xy);
    float4 source = original.Load(int3(pixel, 0));
    if (!affected(pixel)) return source;
    // extra[0]: ambient + emissive texture strength; extra[1]: emissive color.
    return float4(source.rgb * extra[0].rgb + extra[1].rgb, source.a);
}
