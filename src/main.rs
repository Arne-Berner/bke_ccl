use std::iter;
use flume::bounded;
use bke_ccl::*;
use pollster::FutureExt;
use image::ImageBuffer;


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

        let (device, queue) = adapter
            .request_device(&Default::default())
            .await
            .unwrap();

        let image_bytes = include_bytes!("./tobacco.png");
        let texture_bundle =
            texture::TextureUInt::from_bytes(&device, &queue, image_bytes, "in_texture").unwrap();

        Ok(Self {
            texture_bundle,
            device,
            queue,
        })
    }


    fn get_num_bytes_storage(&mut self) -> anyhow::Result<u64> {
        let texture_size = self.texture_bundle.texture.size();
        let width = texture_size.width;
        let height = texture_size.height;
        let num_pixels = width as u64 * height as u64;
        let num_bytes_storage = num_pixels
            .checked_mul(4)
            .expect("The image was too big to create a storage buffer");
        Ok(num_bytes_storage)
    }

    async fn compute(&mut self) -> anyhow::Result<()> {
        let mut encoder = self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let ccl = CCLState::new(&self.device, &self.queue, &self.texture_bundle).unwrap();
        let output_buffer = ccl.compute(&mut encoder)?;
        self.queue.submit(iter::once(encoder.finish()));
        self.check_if_correct(&output_buffer).await?;

        Ok(())
    }

    async fn check_if_correct(&self, output_buffer: &wgpu::Buffer ) -> anyhow::Result<()> {
        let num_bytes_storage = self.get_num_bytes_storage().unwrap();
        // I get the buffer from here
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
            let expected: [u32; 256] = [
                1, 0, 3, 0, 5, 0, 7, 0, 9, 0, 11, 0, 13, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 33, 0, 35, 0, 37, 0, 39, 0, 41, 0, 43, 0, 45, 0, 47, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 65, 0, 67, 0, 69, 0, 71, 0, 73, 0, 75, 0, 77,
                0, 79, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 97, 0, 99, 0, 101, 0,
                103, 0, 105, 0, 107, 0, 109, 0, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 129, 0, 131, 0, 133, 0, 135, 0, 137, 0, 139, 0, 141, 0, 143, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 161, 0, 163, 0, 165, 0, 167, 0, 169, 0, 171, 0,
                173, 0, 175, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 193, 0, 195, 0,
                197, 0, 199, 0, 201, 0, 203, 0, 205, 0, 207, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 225, 0, 227, 0, 229, 0, 231, 0, 233, 0, 235, 0, 237, 0, 239, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ];
            println!("output_data:{:?}", output_data);
            assert_eq!(output_data, expected);
        }

        // We need to unmap the buffer to be able to use it again
        temp_buffer.unmap();

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut state = WGPUState::new().block_on()?;
    state.compute().block_on()?;

    Ok(())
}
