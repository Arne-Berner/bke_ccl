use pollster::FutureExt;

mod buffer_png;
use buffer_png::{BufferBundle, decode};

mod check_colors;
use check_colors::CheckColors;

mod check_array;
use check_array::CheckArrays;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    // let BufferBundle{buffer, width, height} = decode("./src/test_medium.png");
    // let state = CheckColors::new(&buffer[..buffer.len()], width, height).block_on()?;
    let state = CheckArrays::new().block_on()?;
    state.compute().block_on()?;

    Ok(())
}
