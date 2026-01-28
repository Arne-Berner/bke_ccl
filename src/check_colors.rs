// TODO using png instead: https://docs.rs/png/latest/png/
// That way I can check arrays.
use bke_ccl::*;
use flume::bounded;
use std::iter;
use image::{ImageBuffer, RgbaImage};
use wgpu::{Buffer, util::{BufferInitDescriptor, DeviceExt}};

pub struct CheckColors {
    input_buffer: Buffer,
    width: u32,
    height: u32,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl CheckColors {
    pub async fn new(image_bytes:&[u32], width:u32, height:u32 ) -> anyhow::Result<CheckColors> {
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

        let input_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("input"),
            contents: bytemuck::cast_slice(&image_bytes),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        Ok(Self {
            input_buffer,
            width,
            height,
            device,
            queue,
        })
    }

    pub async fn compute(&self) -> anyhow::Result<()> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let ccl = CCLState::new(&self.device, &self.queue, &self.input_buffer, self.width, self.height).unwrap();
        let output_buffer = ccl.compute(&mut encoder)?;
        self.queue.submit(iter::once(encoder.finish()));
        self.check_colors(&output_buffer).await?;

        Ok(())
    }

    async fn check_colors(&self, output_buffer: &wgpu::Buffer ) -> anyhow::Result<()> {
        let temp_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("temp"),
            size: self.input_buffer.size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&Default::default());

        encoder.copy_buffer_to_buffer(output_buffer, 0, &temp_buffer, 0, output_buffer.size()-4u64);

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
            println!("{:?}", output_data.len());

            // using f64 to accomodate the bigger u32 range
            // 255 / (width*height) would be normalized
            // 254 will not create any 0 values for last label
            let normalized = 254.0 / self.width as f64 / self.height as f64;
            let mut rgba_data = Vec::with_capacity(output_data.len() * 4);
            for &label in output_data {
                // grey scale
                let mut val = 255u8 - ((label as f64 * normalized) as u8);
                if label == 0 {
                    val = 0
                }

                let r = val;
                let g = val;
                let b = val;
                let a = 255u8; 

                rgba_data.push(r); 
                rgba_data.push(g);
                rgba_data.push(b);
                rgba_data.push(a);
            }
            let img: RgbaImage = ImageBuffer::from_raw(self.width, self.height, rgba_data).unwrap();

            // Save the image
            img.save("output.png")?;
        }

        // We need to unmap the buffer to be able to use it again
        temp_buffer.unmap();
        Ok(())
    }
}
