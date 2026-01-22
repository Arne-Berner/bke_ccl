struct Dimensions {
    columns: u32,
    rows: u32,
    _pad0: u32,
    _pad1: u32,
}

@group(0) @binding(0)
var<storage, read_write> labels: array<atomic<u32>>;
@group(0) @binding(1)
var<uniform> dims : Dimensions;

fn FindAndCompress(n: u32) -> u32 {
    var idx = n;
    loop {
        let parent = atomicLoad(&labels[idx]);
        if (parent == idx) {
            break;
        }
        idx = parent;
        atomicStore(&labels[n], idx);
    }
    return idx;
}

@compute
@workgroup_size(8, 8, 1)
fn compress(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
){
    // this needs to b *2 since it is supposed to be a 2x2 block
    let row = gid.y * 2u;
    let col = gid.x * 2u;
    let labels_idx = row * dims.columns + col;

    if row < dims.rows && col < dims.columns {
        FindAndCompress(labels_idx);
    }
}
