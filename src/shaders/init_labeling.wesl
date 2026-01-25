import super::util;
// TODO using input_img and labels opens up opportunities:
// 1. I could use the previous columns shifted by 8 bits, to store info for odd numbers
// this works, because I could output the final image in the input_img
// 2. I could make the labels buffer 1/4 smaller (not sure if this helps, since the copying is elsewhere)
// 3. It is agnostic to the input type (I create a buffer from the texture anyway it seems)

// questions: Would it be slower to make input_img read write?
// questions: Is textureDimensions faster than creating and using an Uniform buffer?

// I could make it work in one buffer, if I added 2 extra fields for each array, or if any array
// would only have even rows/cols. The extra fields would be the last info+parent
// since the parent info can't overwrite the label info in this step like before


const BIT_P: u32 = 1u;
const BIT_Q1: u32 = 1u << 1u;
const BIT_Q2: u32 = 1u << 2u;
const BIT_R: u32 = 1u << 3u;
const BIT_S1: u32 = 1u << 4u;
const BIT_S2: u32 = 1u << 8u;
const BIT_X1: u32 = 1u << 5u;
const BIT_X2: u32 = 1u << 6u;
const BIT_X3: u32 = 1u << 9u;
const BIT_X4: u32 = 1u << 10u;

@group(0) @binding(0) var<storage, read> 
input_img: array<u32>;

@group(0) @binding(1) var<storage, read_write> 
labels: array<u32>;

@group(0) @binding(2) var<uniform> 
dims : Dimensions;

struct Dimensions {
    columns: u32,
    rows: u32,
    _pad0: u32,
    _pad1: u32,
}

fn none_texel(idx:u32) -> bool {
    return 0u == input_img[idx];
}
fn some_texel(idx:u32) -> bool {
    return 0u != input_img[idx];
}

@compute
@workgroup_size(8, 8, 1)
fn init_labeling(
    @builtin(global_invocation_id)
    gid: vec3<u32>,
){
    let row = gid.y * 2u;
    let col = gid.x * 2u;
    // let img_size = textureDimensions(in_image);
    let labels_idx = row * dims.columns + col;

    if row < dims.rows && col < dims.columns {

        //      | COL1  | COL2  | COL3  | COL4
        // ROW1 | P     | Q_1   | Q_2   | R
        // ROW2 | S_1   | X_1   | X_2   | 0
        // ROW3 | S_2   | X_3   | X_4   | 0
        //
        // Row 1: bits 0,1,2,3 correspond to col 1,2,3 and 4
        // Row 2: bits 4,5,6,7 correspond to col 1,2,3 and 4
        // Row 3: bits 8,9,10,11 correspond to col 1,2,3 and 4
        // 
        // if a is there: set p, q1, q2, s1, s2
        // if b is there: set q1, q2, r
        // if c is there: set s1, s2
        //
        // if col == 0 -> there can't be a col1
        // if row == 0 -> there can't be a row1
        // if col == last -> there can't be col3 and col4
        var available_pixel = 0xFFFFu;
        var potential_parents = 0x0000u;


        // Bitmask representing two kinds of information
        // Bits 0, 1, 2, 3 are set if pixel a, b, c, d are foreground, respectively
        // Bits 4, 5, 6, 7 are set if block P, Q, R, S need to be merged to X in Merge phase // TODO what is X? The above pixel, I guess.
        // bits 0, 1, 2, 3 are set at the beginning and removed if necessary
        var info = 0x0000u;

        // row/col start at 0 and util.rows/columns start at 1
        // set available_pixel
        // if there is no right row or bottom column, remove the set info
        if dims.rows <= row + 1u{
            available_pixel = util::RemoveBits(available_pixel, (util::ROW_3));
        }
        if dims.columns <= col + 1u{
            available_pixel = util::RemoveBits(available_pixel, (util::COL_3 | util::COL_4));
        }
        // first row, so no row above it
        if row == 0u {
            available_pixel = util::RemoveBits(available_pixel, util::ROW_1);
        }
        // first column in a row, so no row to the left
        if col == 0u {
            available_pixel = util::RemoveBits(available_pixel, util::COL_1);
        }

        // set potential parents and info
        // at this point its just potentially an a
        if util::HasBits(available_pixel, util::X1) {
            if some_texel(labels_idx) {
                info = util::SetBits(info, (util::A));
                potential_parents = util::SetBits(potential_parents, util::A_BLOCK);
            }
        }
        if util::HasBits(available_pixel, util::X2) {
            if some_texel(labels_idx+1) {
                info = util::SetBits(info, (util::B));
                potential_parents = util::SetBits(potential_parents, util::B_BLOCK);
            }
        }
        if util::HasBits(available_pixel, util::X3) {
            if some_texel(labels_idx+dims.columns) {
                info = util::SetBits(info, (util::C));
                potential_parents = util::SetBits(potential_parents, util::C_BLOCK);
            }
        }
        if util::HasBits(available_pixel, util::X4) {
            if some_texel(labels_idx+dims.columns+1) {
                info = util::SetBits(info, (util::D));
            }
        }

        // erase any potential parents that are not possible
        potential_parents = potential_parents & available_pixel;

        var parent_idx = labels_idx;
        var parent_flag = false;

        // the 2x2 block "P"
        let p_idx = labels_idx - dims.columns - 1u;
        if util::HasBits(potential_parents, BIT_P) && some_texel(p_idx) {
            // uses P as a parent
            parent_idx = labels_idx - (2u * dims.columns + 2u);
            parent_flag = true;
        }
        
        // the 2x2 block "Q"
        let q1_idx = labels_idx - dims.columns;
        let q2_idx = labels_idx - dims.columns + 1u;
        // TODO the parenthesis are not necessary because && > ||
        if (util::HasBits(potential_parents, BIT_Q1) && some_texel(q1_idx)) || (util::HasBits(potential_parents, BIT_Q2) && some_texel(q2_idx)) {
            if parent_flag {
                info = util::SetBits(info, util::Q);
            } else {
                // 2x2 to the top
                parent_idx = labels_idx - (2u * dims.columns);
                parent_flag = true;
            }
        }

        // the 2x2 block "R"
        let r_idx = labels_idx - dims.columns + 2u;
        // P BIT_R should only be set, if there is b
        if util::HasBits(potential_parents, BIT_R) && some_texel(r_idx) {
            if parent_flag {
                info = util::SetBits(info, util::R);
            } else {
                // 2x2 to the top-right
                parent_idx = labels_idx - (2u * dims.columns) + 2u;
                parent_flag = true;
            }
        }

        // the 2x2 block "S"
        let s1_idx = labels_idx - 1u;
        let s2_idx = labels_idx + dims.columns - 1u;
        // potential partents BIT_S1+BIT_S2 should only be set, if there is a or c
        if (util::HasBits(potential_parents, BIT_S1) && some_texel(s1_idx)) || (util::HasBits(potential_parents, BIT_S2) && some_texel(s2_idx)) {
            if parent_flag {
                info = util::SetBits(info, util::S);
            } else {
                parent_idx = labels_idx - 2u;
                parent_flag = true;
            }
        }
        
        labels[labels_idx] = parent_idx;
        if util::HasBits(available_pixel, BIT_X2) {
            // put into X2
            labels[labels_idx+1] = info;
        } else if util::HasBits(available_pixel, BIT_X3) {
            // put into X3
            labels[labels_idx+ dims.columns + 1] = info;
        } else {
            // using the extra pixel added in lib
            labels[labels_idx+1] = info;
        }
    }
}
