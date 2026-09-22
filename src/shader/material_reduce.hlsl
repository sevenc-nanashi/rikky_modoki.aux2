cbuffer Constants : register(b0) { float4 options; }; // width, height, partition, layer alpha
Texture2D<float4> source : register(t0);
RWTexture2D<float4> cells : register(u0);

[numthreads(8, 8, 1)]
void rikky_material_reduce(uint2 id : SV_DispatchThreadID) {
    uint2 size = uint2(options.xy);
    uint partition = (uint)options.z;
    uint2 start = id * partition;
    if (any(start >= size)) return;
    uint2 end = min(start + partition, size);
    float3 sum = 0;
    for (uint y = start.y; y < end.y; ++y) {
        for (uint x = start.x; x < end.x; ++x) {
            float4 pixel = source.Load(int3(x, y, 0));
            sum += pixel.rgb * pixel.a;
        }
    }
    cells[id] = float4(sum * options.w / ((end.x-start.x)*(end.y-start.y)), 1);
}
