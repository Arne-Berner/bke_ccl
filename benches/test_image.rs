use bke_ccl::*;
use criterion::{criterion_group, criterion_main, Criterion};
use pollster::FutureExt;

// This is a struct that tells Criterion.rs to use the "futures" crate's current-thread executor
use criterion::async_executor::FuturesExecutor;

pub struct WGPUState {
    texture_bundle: texture::TextureUInt,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl WGPUState {
    async fn new(image_bytes: &[u8]) -> anyhow::Result<WGPUState> {
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

        let texture_bundle =
            texture::TextureUInt::from_bytes(&device, &queue, image_bytes, "in_texture").unwrap();

        Ok(Self {
            texture_bundle,
            device,
            queue,
        })
    }

    async fn compute(&self) -> anyhow::Result<()> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let ccl = CCLState::new(&self.device, &self.queue, &self.texture_bundle).unwrap();
        let _output_buffer = ccl.compute(&mut encoder)?;

        Ok(())
    }
}

async fn create_wgpu_state() -> Result<(), Box<dyn std::error::Error>> {
    let image_bytes = include_bytes!("test_8connectivity_maxlabelEE.png");
    let _state = WGPUState::new(image_bytes).await?;
    Ok(())
}

async fn compute_wgpu_state() -> Result<(), Box<dyn std::error::Error>> {
    let image_bytes = include_bytes!("test_8connectivity_maxlabelEE.png");
    let state = WGPUState::new(image_bytes).await?;
    state.compute().await?;
    Ok(())
}

async fn create_wgpu_state_finger() -> Result<(), Box<dyn std::error::Error>> {
    let image_bytes = include_bytes!("fingerprint.png");
    let _state = WGPUState::new(image_bytes).await?;
    Ok(())
}

async fn compute_wgpu_state_finger() -> Result<(), Box<dyn std::error::Error>> {
    let image_bytes = include_bytes!("fingerprint.png");
    let state = WGPUState::new(image_bytes).await?;
    state.compute().await?;
    Ok(())
}

fn simple_image(c: &mut Criterion) {
    let mut group = c.benchmark_group("WGPU Benchmarks");

    group.bench_function("test only setup", |b| {
        b.to_async(FuturesExecutor).iter(|| create_wgpu_state()); // Benchmarks WGPUState::new()
    });

    group.bench_function("test with setup", |b| {
        b.to_async(FuturesExecutor).iter(|| compute_wgpu_state()); // Benchmarks state.compute()
    });

    let image_bytes = include_bytes!("test_8connectivity_maxlabelEE.png");
    let state = WGPUState::new(image_bytes).block_on().expect("could not create state");
    group.bench_function("test only compute", |b| {
        b.to_async(FuturesExecutor).iter(|| {
            state.compute() // Measure only the `compute()` function of WGPUState
        });
    });

    group.bench_function("finger only setup", |b| {
        b.to_async(FuturesExecutor).iter(|| create_wgpu_state_finger()); // Benchmarks WGPUState::new()
    });

    group.bench_function("finger with setup", |b| {
        b.to_async(FuturesExecutor).iter(|| compute_wgpu_state_finger()); // Benchmarks state.compute()
    });

    let image_bytes = include_bytes!("fingerprint.png");
    let state = WGPUState::new(image_bytes).block_on().expect("could not create state");
    group.bench_function("finger only compute", |b| {
        b.to_async(FuturesExecutor).iter(|| {
            state.compute() // Measure only the `compute()` function of WGPUState
        });
    });

    let image_bytes = include_bytes!("tobacco.png");
    let state = WGPUState::new(image_bytes).block_on().expect("could not create state");
    group.bench_function("tobacco only compute", |b| {
        b.to_async(FuturesExecutor).iter(|| {
            state.compute() // Measure only the `compute()` function of WGPUState
        });
    });

    group.finish();
}

criterion_group!(benches, simple_image);
criterion_main!(benches);
