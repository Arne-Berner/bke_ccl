use bke_ccl::*;
use flume::bounded;
use pollster::FutureExt;
use std::iter;
use image::{ImageBuffer, RgbaImage};

pub struct WGPUState {
    texture_bundle: texture::TextureUInt,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl WGPUState {
    async fn new() -> anyhow::Result<WGPUState> {
        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();

        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();

        let image_bytes = include_bytes!("./test.png");
        let texture_bundle =
            texture::TextureUInt::from_bytes(&device, &queue, image_bytes, "in_texture").unwrap();

        Ok(Self {
            texture_bundle,
            device,
            queue,
        })
    }

    fn get_num_bytes_storage(&self) -> anyhow::Result<u64> {
        let texture_size = self.texture_bundle.texture.size();
        let width = texture_size.width;
        let height = texture_size.height;
        let num_pixels = width as u64 * height as u64;
        let num_bytes_storage = num_pixels
            .checked_mul(4)
            .expect("The image was too big to create a storage buffer");
        Ok(num_bytes_storage)
    }

    async fn compute(&self) -> anyhow::Result<()> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let ccl = CCLState::new(&self.device, &self.queue, &self.texture_bundle).unwrap();
        let output_buffer = ccl.compute(&mut encoder)?;
        self.queue.submit(iter::once(encoder.finish()));
        self.check_colors(&output_buffer).await?;

        Ok(())
    }

    async fn check_colors(&self, output_buffer: &wgpu::Buffer ) -> anyhow::Result<()> {
        let num_bytes_storage = self.get_num_bytes_storage().unwrap();
        let temp_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("temp"),
            size: num_bytes_storage,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());

        encoder.copy_buffer_to_buffer(output_buffer, 0, &temp_buffer, 0, output_buffer.size());

        self.queue.submit([encoder.finish()]);

        {
            // The mapping process is async, so we'll need to create a channel to get
            // the success flag for our mapping
            let (tx, rx) = bounded(1);

            // We send the success or failure of our mapping via a callback
            temp_buffer.map_async(wgpu::MapMode::Read, .., move |result| {
                tx.send(result).unwrap()
            });

            // The callback we submitted to map async will only get called after the
            // device is polled or the queue submitted
            self.device.poll(wgpu::PollType::wait_indefinitely())?;

            // We check if the mapping was successful here
            rx.recv_async().await??;

            // We then get the bytes that were stored in the buffer
            let output_buffer_view = temp_buffer.get_mapped_range(..);

            // Now we have the data on the CPU we can do what ever we want to with it
            let output_data = bytemuck::cast_slice::<_, u32>(&output_buffer_view);

            // using f64 to accomodate the bigger u32 range
            let normalized = 255.0 / self.texture_bundle.texture.size().width as f64 / self.texture_bundle.texture.size().height as f64;
            let mut rgba_data = Vec::with_capacity(output_data.len() * 4);
            for &label in output_data {
                // grey scale
                let r = (label as f64 * normalized) as u8;
                let g = (label as f64 * normalized) as u8;
                let b = (label as f64 * normalized) as u8;
                let a = 255u8; 

                rgba_data.push(r); 
                rgba_data.push(g);
                rgba_data.push(b);
                rgba_data.push(a);
            }
            let img: RgbaImage = ImageBuffer::from_raw(256, 256, rgba_data).unwrap();

            // Save the image
            img.save("output.png")?;
        }

        // We need to unmap the buffer to be able to use it again
        temp_buffer.unmap();
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let state = WGPUState::new().block_on()?;
    state.compute().block_on()?;

    Ok(())
}
