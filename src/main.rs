use pollster::FutureExt;

// mod check_colors;
// use check_colors::CheckColors;

mod check_array;
use check_array::CheckArrays;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    // let image_bytes = include_bytes!("./test.png");
    // let state = CheckColors::new(image_bytes).block_on()?;
    let state = CheckArrays::new().block_on()?;
    state.compute().block_on()?;

    Ok(())
}
