struct Dims { width: u32, height: u32, _pad0: u32, _pad1: u32 }

@group(0) @binding(0) var<storage, read> labels: array<u32>;
@group(0) @binding(1) var out_tex: texture_storage_2d<rgba8uint, write>;
@group(0) @binding(2) var<uniform> dims: Dims;

fn convert(v: u32) -> vec4<u32> {
    let r =  (v >>  0u) & 0xFFu;
    let g =  (v >>  8u) & 0xFFu;
    let b =  (v >> 16u) & 0xFFu;
    let a =  (v >> 24u) & 0xFFu;
    return vec4<u32>(r, g, b, a);
}

@compute @workgroup_size(8, 8, 1)
fn label_to_rgba(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= dims.width || gid.y >= dims.height) { return; }
    let idx = gid.y * dims.width + gid.x;
    let v = labels[idx];
    let c = convert(v);
    textureStore(out_tex, vec2<i32>(i32(gid.x), i32(gid.y)), c);
}
