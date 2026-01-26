// TODO output buffer ist _viel_ größer
use std::fs::File;
use std::io::BufReader;
use png::{Decoder, ColorType, BitDepth};

pub struct BufferBundle {
    pub buffer: Vec<u32>,
    pub width: u32,
    pub height: u32,
}
pub fn decode(path: &str) -> BufferBundle {
    let decoder = png::Decoder::new(BufReader::new(File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();

    // Allocate the output buffer.
    let mut buffer = vec![0; reader.output_buffer_size().unwrap()];

    // Read the next frame. An APNG might contain multiple frames.
    let info = reader.next_frame(&mut buffer).unwrap();
    let width = info.width;
    let height = info.height;

    // Grab the bytes of the image.
    buffer.truncate(info.buffer_size());

    // let bytes = &buffer[..info.buffer_size()];
    println!("len: {:?}, width: {:?}, height: {:?}, info: {:?}", buffer.len(), info.width, info.height, info);

     let buffer: Vec<u32> = match info.color_type {
        ColorType::Rgba => {
            println!("RGBA");
            // Combine 4 u8s (RGBA) into a single u32 for each pixel
            buffer.chunks_exact(4)
                .map(|chunk| {
                    (chunk[0] as u32) << 24 | // Red
                    (chunk[1] as u32) << 16 | // Green
                    (chunk[2] as u32) << 8  | // Blue
                    (chunk[3] as u32)        // Alpha
                })
                .collect()
        }
        ColorType::Rgb => {
            // Combine 3 u8s (RGB) into a single u32 for each pixel (0xRRGGBB)
            println!("RGB");
            buffer.chunks_exact(3)
                .map(|chunk| {
                    (chunk[0] as u32) << 16 | // Red
                    (chunk[1] as u32) << 8  | // Green
                    (chunk[2] as u32)        // Blue
                })
                .collect()
        }
        ColorType::Grayscale => {

            if info.bit_depth == BitDepth::One {
                let byte_width = (info.width + 7) / 8; // Each row is padded to the nearest whole byte
                let mut unpacked_pixels = Vec::with_capacity((info.width * height) as usize);
                // Treat each pixel (1-byte grayscale) as a 32-bit scalar (0x00G000FF)
                for row in 0..height {
                    let row_start = (row * byte_width) as usize;
                    for col in 0..info.width {
                        // Extract the bit corresponding to the current column
                        let byte = buffer[row_start + (col / 8) as usize];
                        let bit = 7 - (col % 8); // Bit position within the byte (MSB to LSB)
                        let value = ((byte >> bit) & 1) as u32; // Extract the bit as 0 or 1
                        unpacked_pixels.push(value);
                    }
                }
                unpacked_pixels
            } else {
                buffer.iter()
                    .map(|&gray| gray as u32)
                    .collect()
            }
        }
        _ => panic!("Unsupported color type for u32 conversion."),
    };
    println!("{:?}", buffer.len());
    return BufferBundle {
        buffer,
        width,
        height,
    };
}
