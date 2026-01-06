pub mod texture;

use wgpu::{BufferDescriptor, util::{BufferInitDescriptor, DeviceExt}};
use wesl::include_wesl;


#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Dimensions {
    columns: u32,
    rows:    u32,
    _pad0:   u32,
    _pad1:   u32,
}

pub struct CCLState {
    width: u32,
    height: u32,
    init_pipeline: wgpu::ComputePipeline,
    init_bind_group: wgpu::BindGroup,
    compress_pipeline: wgpu::ComputePipeline,
    compress_bind_group: wgpu::BindGroup,
    merge_pipeline: wgpu::ComputePipeline,
    merge_bind_group: wgpu::BindGroup,
    final_labeling_pipeline: wgpu::ComputePipeline,
    label_to_rgba_pipeline: wgpu::ComputePipeline,
    label_to_rgba_bind_group: wgpu::BindGroup,
    labels_buffer: wgpu::Buffer,
}

impl CCLState {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, texture_bundle: &texture::TextureUInt) -> anyhow::Result<CCLState> {
        let texture_size = texture_bundle.texture.size();
        let width = texture_size.width;
        let height = texture_size.height;
        let dims = Dimensions {columns: width, rows: height, _pad0: 0, _pad1: 0};
        let dims_buffer = device.create_buffer_init(&BufferInitDescriptor{
            label: Some("Dimensions Uniform"),
            contents: bytemuck::cast_slice(&[dims]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let num_pixels = width as u64 * height as u64;
        // every pixel is now rgba<u8> so 32bit
        let num_bytes_storage = num_pixels
            .checked_mul(4)
            .expect("The image was too big to create a storage buffer");
        let labels_buffer = device.create_buffer(&BufferDescriptor{ 
            label: Some("Labels Buffer"),
            size: num_bytes_storage,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false, });
        let info_buffer = device.create_buffer(&BufferDescriptor{ 
            label: Some("Labels Buffer"),
            size: num_bytes_storage,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false, });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Encoder"),
        });
        encoder.clear_buffer(&labels_buffer, 0, None);
        queue.submit(std::iter::once(encoder.finish()));

        let init_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("init_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture { 
                            access: wgpu::StorageTextureAccess::ReadOnly, 
                            format: wgpu::TextureFormat::Rgba8Uint, 
                            view_dimension: wgpu::TextureViewDimension::D2
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None, // or Some(NonZeroU64::new(labels_size).unwrap())
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None, // or Some(NonZeroU64::new(labels_size).unwrap())
                        },
                        count: None,
                    },
                ],
            });

        let compress_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("compress_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None, // or Some(NonZeroU64::new(labels_size).unwrap())
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            // 16 bytes is a safe minimum for two u32s + padding
                            min_binding_size: Some(std::num::NonZeroU64::new(16).unwrap()),
                        },
                        count: None,
                    },
                ],
            });

        let merge_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("merge_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None, // or Some(NonZeroU64::new(labels_size).unwrap())
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None, // or Some(NonZeroU64::new(labels_size).unwrap())
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            // 16 bytes is a safe minimum for two u32s + padding
                            min_binding_size: Some(std::num::NonZeroU64::new(16).unwrap()),
                        },
                        count: None,
                    },
                ],
            });

        let label_to_rgba_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("merge_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None, 
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Uint,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            // 16 bytes is a safe minimum for two u32s + padding
                            min_binding_size: Some(std::num::NonZeroU64::new(16).unwrap()),
                        },
                        count: None,
                    },
                ],
            });

        let init_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("init_bind_group"),
            layout: &init_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_bundle.view)
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: labels_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: info_buffer.as_entire_binding(),
                },
            ],
        });

        let compress_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compress_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: labels_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: dims_buffer.as_entire_binding(),
                },
            ],
            label: Some("compress_bind_group"),
        });
        
        let merge_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &merge_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: labels_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: info_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: dims_buffer.as_entire_binding(),
                },
            ],
            label: Some("merge_bind_group"),
        });

        let label_to_rgba_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &label_to_rgba_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: labels_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_bundle.view)
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: dims_buffer.as_entire_binding(),
                },
            ],
            label: Some("label_to_rgba_bind_group"),
        });


        let shader_string = include_wesl!("init_labeling");
        let shader_source = wgpu::ShaderSource::Wgsl(shader_string.into());

        let init_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Init Shader"),
            source: shader_source,
        });

        let shader_string = include_wesl!("compress");
        let shader_source = wgpu::ShaderSource::Wgsl(shader_string.into());

        let compress_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compress Shader"),
            source: shader_source,
        });

        let shader_string = include_wesl!("merge");
        let shader_source = wgpu::ShaderSource::Wgsl(shader_string.into());

        let merge_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Merge Shader"),
            source: shader_source,
        });

        let shader_string = include_wesl!("final_labeling");
        let shader_source = wgpu::ShaderSource::Wgsl(shader_string.into());

        let final_labeling_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Final Labeling Shader"),
            source: shader_source,
        });

        let shader_string = include_wesl!("label_to_rgba");
        let shader_source = wgpu::ShaderSource::Wgsl(shader_string.into());

        let label_to_rgba_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Label to RGBA Shader"),
            source: shader_source,
        });

        let init_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Init pipeline layout"),
                bind_group_layouts: &[ &init_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compress_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("compress pipeline layout"),
                bind_group_layouts: &[ &compress_bind_group_layout],
                push_constant_ranges: &[],
            });

        let merge_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("merge pipeline layout"),
                bind_group_layouts: &[ &merge_bind_group_layout],
                push_constant_ranges: &[],
            });

        let label_to_rgba_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("label to rgba pipeline layout"),
                bind_group_layouts: &[ &label_to_rgba_bind_group_layout],
                push_constant_ranges: &[],
            });

        let init_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("init Pipeline"),
            layout: Some(&init_pipeline_layout),
            module: &init_shader,
            entry_point: "init_labeling".into(),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        let compress_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compress Pipeline"),
            layout: Some(&compress_pipeline_layout),
            module: &compress_shader,
            entry_point: "compress".into(),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        let merge_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("merge Pipeline"),
            layout: Some(&merge_pipeline_layout),
            module: &merge_shader,
            entry_point: "merge".into(),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        let final_labeling_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("final labeling Pipeline"),
            layout: Some(&merge_pipeline_layout),
            module: &final_labeling_shader,
            entry_point: "final_labeling".into(),
            compilation_options: Default::default(),
            cache: Default::default(),
        });

        let label_to_rgba_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Label to RGBA Pipeline"),
            layout: Some(&label_to_rgba_pipeline_layout),
            module: &label_to_rgba_shader,
            entry_point: "label_to_rgba".into(),
            compilation_options: Default::default(),
            cache: Default::default(),
        });




        Ok(Self {
            width,
            height,
            init_pipeline,
            init_bind_group,
            compress_pipeline,
            compress_bind_group,
            merge_pipeline,
            merge_bind_group,
            final_labeling_pipeline,
            label_to_rgba_pipeline,
            label_to_rgba_bind_group,
            labels_buffer,
        })
    }

    pub fn compute(self, encoder: &mut wgpu::CommandEncoder) -> Result<wgpu::Buffer, wgpu::SurfaceError> {
        // this needs to run every frame, otherwise the buffer will be cleared
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.init_pipeline);
            compute_pass.set_bind_group(0, &self.init_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                // workgroup / 2 because it is checking 2x2 blocks
                self.width/16 + self.width % 16, 
                self.height/16 + self.height % 16, 
                1
            );

            compute_pass.set_pipeline(&self.compress_pipeline);
            compute_pass.set_bind_group(0, &self.compress_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                self.width/16 + self.width % 16, 
                self.height/16 + self.height % 16, 
                1
            );

            compute_pass.set_pipeline(&self.merge_pipeline);
            compute_pass.set_bind_group(0, &self.merge_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                self.width/16 + self.width % 16, 
                self.height/16 + self.height % 16, 
                1
            );

            compute_pass.set_pipeline(&self.compress_pipeline);
            compute_pass.set_bind_group(0, &self.compress_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                self.width/16 + self.width % 16, 
                self.height/16 + self.height % 16, 
                1
            );
            compute_pass.set_pipeline(&self.final_labeling_pipeline);
            compute_pass.set_bind_group(0, &self.merge_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                self.width/16 + self.width % 16, 
                self.height/16 + self.height % 16, 
                1
            );
            compute_pass.set_pipeline(&self.label_to_rgba_pipeline);
            compute_pass.set_bind_group(0, &self.label_to_rgba_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                self.width/8 + self.width % 8, 
                self.height/8 + self.width % 8, 
                1
            );
        }

        Ok(self.labels_buffer)
    }
}

