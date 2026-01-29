import super::util;

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
fn Find(n: u32) -> u32 {
    var idx = n;

    loop {
        let parent = atomicLoad(&labels[idx]);
        if (parent == idx) {
            break;
        }
        idx = parent;

    }
    return idx;
}

fn Union(a_in: u32, b_in: u32) {
    var a: u32 = a_in;
    var b: u32 = b_in;
    var done: bool = false;

    loop {
        // Find current roots
        a = Find(a);
        b = Find(b);

        if (a < b) {
            // atomicMin takes i32 and returns the previous i32 value
            let old = atomicMin(&labels[b], a);
            if old == b {
                break;
            }
            b = old;
        } else if (b < a) {
            let old = atomicMin(&labels[a], b);
            if old == a {
                break;
            }
            a = old;
        } else {
            break;
        }
    }
}

@compute
@workgroup_size(8, 8, 1)
fn merge(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
){
    // this needs to b *2 since it is supposed to be a 2x2 block
    let row = gid.y * 2u;
    let col = gid.x * 2u;
    let labels_idx = row * dims.columns + col;

    if row < dims.rows && col < dims.columns {
        var info = 0u;
        if col < dims.columns - 1 {
            info = labels[labels_idx + 1];
        } else if row < dims.rows - 1 {
            info = labels[labels_idx + dims.columns];
        } else {
            info = labels[labels_idx + 1];
        }

        if util::HasBits(info, util::Q) {
            Union(labels_idx, labels_idx - (dims.columns * 2u));
        }

        if util::HasBits(info, util::R) {
            Union(labels_idx, labels_idx - (dims.columns * 2u) + 2u);
        }

        if util::HasBits(info, util::S) {
            Union(labels_idx, labels_idx - 2u);
        }
    }
}
