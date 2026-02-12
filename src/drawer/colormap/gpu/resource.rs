use super::*;

#[derive(Debug)]
pub struct RenderResources {
    pub(crate) uniforms: Uniforms,
    // compute stuff
    pub(crate) compute_pipeline_layout: wgpu::PipelineLayout,
    pub(crate) compute_pipeline_raw_reduce: wgpu::ComputePipeline,
    pub(crate) compute_pipeline_raw: wgpu::ComputePipeline,
    pub(crate) compute_pipeline_rf_transpose: wgpu::ComputePipeline,
    pub(crate) compute_pipeline_rf_stage1: wgpu::ComputePipeline,
    pub(crate) compute_pipeline_rf_reduce_global: wgpu::ComputePipeline,
    pub(crate) compute_pipeline_rf_stage2: wgpu::ComputePipeline,
    pub(crate) compute_shader: wgpu::ShaderModule,
    pub(crate) raw_data_buffer: wgpu::Buffer,
    pub(crate) rf_input_buffer: wgpu::Buffer,
    pub(crate) rf_fft_state_buffer: wgpu::Buffer,
    pub(crate) rf_value_buffer: wgpu::Buffer,
    pub(crate) rf_minmax_buffer: wgpu::Buffer,
    pub(crate) cache_buffer: wgpu::Buffer,
    pub(crate) compute_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) compute_bind_group: wgpu::BindGroup,
    pub(crate) fft_cfg: FftRuntimeConfig,
    // render stuff
    pub(crate) render_pipeline_layout: wgpu::PipelineLayout,
    pub(crate) render_pipeline: wgpu::RenderPipeline,
    pub(crate) render_shader: wgpu::ShaderModule,
    pub(crate) uniform_buffer: wgpu::Buffer,
    pub(crate) _colormap_buffer: wgpu::Buffer,
    pub(crate) sampler: wgpu::Sampler,
    pub(crate) texture: wgpu::Texture,
    pub(crate) texture_format: wgpu::TextureFormat,
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) render_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) render_bind_group: wgpu::BindGroup,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FftRuntimeConfig {
    pub(crate) wg_x: u32,
    pub(crate) wg_bins: u32,
    pub(crate) shared_max_n: u32,
}

fn floor_pow2(v: u32) -> u32 {
    if v <= 1 {
        1
    } else {
        1 << (31 - v.leading_zeros())
    }
}

fn fft_runtime_config(limits: &wgpu::Limits) -> FftRuntimeConfig {
    let max_inv = limits.max_compute_invocations_per_workgroup.max(1);
    let max_x = limits.max_compute_workgroup_size_x.max(1);
    let max_y = limits.max_compute_workgroup_size_y.max(1);
    let wg_x = 64u32.min(max_x).min(max_inv).max(1);
    let wg_bins = 4u32.min(max_y).min((max_inv / wg_x).max(1)).max(1);
    let bytes_per_complex = size_of::<RfInputFormat>() as u32;
    let storage_budget = limits.max_compute_workgroup_storage_size;
    let max_n_by_storage = (storage_budget / (wg_bins * bytes_per_complex)).max(2);
    let shared_max_n = floor_pow2(max_n_by_storage).max(2);
    FftRuntimeConfig {
        wg_x,
        wg_bins,
        shared_max_n,
    }
}

impl RenderResources {
    pub fn update_uniforms(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniforms: Uniforms,
    ) {
        if self.uniforms.width != uniforms.width || self.uniforms.height != uniforms.height {
            (
                self.raw_data_buffer,
                self.rf_input_buffer,
                self.rf_fft_state_buffer,
                self.rf_value_buffer,
                self.rf_minmax_buffer,
                self.cache_buffer,
                self.texture,
            ) = RenderResources::get_buffers(
                device,
                self.texture_format,
                uniforms.width,
                uniforms.height,
            );

            (
                self.compute_bind_group,
                self.compute_pipeline_raw_reduce,
                self.compute_pipeline_raw,
                self.compute_pipeline_rf_transpose,
                self.compute_pipeline_rf_stage1,
                self.compute_pipeline_rf_reduce_global,
                self.compute_pipeline_rf_stage2,
                self.fft_cfg,
            ) = RenderResources::get_compute_pipelines(
                device,
                &self.raw_data_buffer,
                &self.rf_input_buffer,
                &self.rf_fft_state_buffer,
                &self.rf_value_buffer,
                &self.rf_minmax_buffer,
                &self.cache_buffer,
                &self.uniform_buffer,
                &self._colormap_buffer,
                &self.compute_bind_group_layout,
                &self.compute_pipeline_layout,
                &self.compute_shader,
            );

            let texture_view = self
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            (self.render_bind_group, self.render_pipeline) = RenderResources::get_render_pipeline(
                device,
                self.texture_format,
                &texture_view,
                &self.sampler,
                &self.render_bind_group_layout,
                &self.render_pipeline_layout,
                &self.render_shader,
            );
        }
        self.uniforms = uniforms;
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    pub fn update_raw_buffer(&self, _device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8]) {
        queue.write_buffer(&self.raw_data_buffer, 0, data);
    }

    pub fn update_rf_input_buffer(&self, _device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8]) {
        queue.write_buffer(&self.rf_input_buffer, 0, data);
    }

    /// results stored at a cache buffer, copy to texture by calling [Self::refresh_texture]
    pub fn compute_texture(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.push_debug_group("compute texture");
        const WORKGROUP_SIZE: (u8, u8, u8) = (8, 8, 1);
        let dispatch_x = self.uniforms.width.div_ceil(WORKGROUP_SIZE.0 as u32);
        let dispatch_y = self.uniforms.height.div_ceil(WORKGROUP_SIZE.1 as u32);
        if self.uniforms.compute_mode == 0 {
            if self.uniforms.raw_gpu_range != 0 {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("raw reduce"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.compute_pipeline_raw_reduce);
                cpass.set_bind_group(0, &self.compute_bind_group, &[]);
                cpass.dispatch_workgroups(1, 1, 1);
            }
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("raw colormap compute"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.compute_pipeline_raw);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            cpass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
        } else {
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("rf transpose"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.compute_pipeline_rf_transpose);
                cpass.set_bind_group(0, &self.compute_bind_group, &[]);
                cpass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
            }
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("rf fft stage1"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.compute_pipeline_rf_stage1);
                cpass.set_bind_group(0, &self.compute_bind_group, &[]);
                let stage1_dispatch_x = self.uniforms.width.div_ceil(self.fft_cfg.wg_bins.max(1));
                cpass.dispatch_workgroups(stage1_dispatch_x, 1, 1);
            }
            if self.uniforms.rf_global_norm != 0 {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("rf fft global reduce"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.compute_pipeline_rf_reduce_global);
                cpass.set_bind_group(0, &self.compute_bind_group, &[]);
                cpass.dispatch_workgroups(1, 1, 1);
            }
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("rf fft stage2"),
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.compute_pipeline_rf_stage2);
                cpass.set_bind_group(0, &self.compute_bind_group, &[]);
                cpass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
            }
        }
        encoder.pop_debug_group();
    }

    pub fn refresh_texture(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &self.cache_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size_of::<TextureFormat>() as u32 * self.uniforms.width),
                    rows_per_image: Some(self.uniforms.height),
                },
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.uniforms.width,
                height: self.uniforms.height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn paint(&self, rpass: &mut wgpu::RenderPass<'_>) {
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.render_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.draw(0..3, 0..1); // paint triangles
    }
}

impl RenderResources {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn get_compute_pipelines(
        device: &wgpu::Device,
        raw_data: &wgpu::Buffer,
        rf_input: &wgpu::Buffer,
        rf_fft_state: &wgpu::Buffer,
        rf_values: &wgpu::Buffer,
        rf_minmax: &wgpu::Buffer,
        cache: &wgpu::Buffer,
        uniform: &wgpu::Buffer,
        color_map: &wgpu::Buffer,
        bind_group_layout: &wgpu::BindGroupLayout,
        pipeline_layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
    ) -> (
        wgpu::BindGroup,
        wgpu::ComputePipeline,
        wgpu::ComputePipeline,
        wgpu::ComputePipeline,
        wgpu::ComputePipeline,
        wgpu::ComputePipeline,
        wgpu::ComputePipeline,
        FftRuntimeConfig,
    ) {
        let cfg = fft_runtime_config(&device.limits());
        let fft_override_constants = [
            ("FFT_WG_X", cfg.wg_x as f64),
            ("FFT_WG_BINS", cfg.wg_bins as f64),
            ("FFT_SHARED_MAX_N", cfg.shared_max_n as f64),
        ];
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: raw_data,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: rf_input,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: rf_fft_state,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: rf_values,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: rf_minmax,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: cache,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: uniform,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: color_map,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
            label: Some("Compute Bind Group"),
        });

        let compute_pipeline_raw_reduce =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline Raw Reduce"),
                layout: Some(pipeline_layout),
                module: shader,
                entry_point: Some("main_raw_reduce"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &fft_override_constants,
                    ..Default::default()
                },
                cache: None,
            });
        let compute_pipeline_raw =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline"),
                layout: Some(pipeline_layout),
                module: shader,
                entry_point: Some("main_raw"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &fft_override_constants,
                    ..Default::default()
                },
                cache: None,
            });
        let compute_pipeline_rf_transpose =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline RF Transpose"),
                layout: Some(pipeline_layout),
                module: shader,
                entry_point: Some("main_rf_transpose"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &fft_override_constants,
                    ..Default::default()
                },
                cache: None,
            });
        let compute_pipeline_rf_stage1 =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline RF Stage1"),
                layout: Some(pipeline_layout),
                module: shader,
                entry_point: Some("main_rf_stage1"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &fft_override_constants,
                    ..Default::default()
                },
                cache: None,
            });
        let compute_pipeline_rf_reduce_global =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline RF Global Reduce"),
                layout: Some(pipeline_layout),
                module: shader,
                entry_point: Some("main_rf_reduce_global"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &fft_override_constants,
                    ..Default::default()
                },
                cache: None,
            });
        let compute_pipeline_rf_stage2 =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Compute Pipeline RF Stage2"),
                layout: Some(pipeline_layout),
                module: shader,
                entry_point: Some("main_rf_stage2"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &fft_override_constants,
                    ..Default::default()
                },
                cache: None,
            });
        (
            compute_bind_group,
            compute_pipeline_raw_reduce,
            compute_pipeline_raw,
            compute_pipeline_rf_transpose,
            compute_pipeline_rf_stage1,
            compute_pipeline_rf_reduce_global,
            compute_pipeline_rf_stage2,
            cfg,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn get_render_pipeline(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        bind_group_layout: &wgpu::BindGroupLayout,
        pipeline_layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
    ) -> (wgpu::BindGroup, wgpu::RenderPipeline) {
        // create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        (bind_group, render_pipeline)
    }
    // get raw_data_buffer, cache_buffer, and texture
    pub(crate) fn get_buffers(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> (
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Texture,
    ) {
        let raw_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Raw Data Buffer"),
            size: (width * height) as u64 * size_of::<RawDataFormat>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rf_input_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RF Input Buffer"),
            size: (width * height) as u64 * size_of::<RfInputFormat>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rf_value_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RF Value Buffer"),
            size: (width * height) as u64 * size_of::<f32>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rf_fft_state_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RF FFT State Buffer"),
            size: (width * height) as u64 * size_of::<RfInputFormat>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rf_minmax_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("RF MinMax Buffer"),
            size: width as u64 * size_of::<[f32; 2]>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cache_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cache Buffer"),
            size: (width * height) as u64 * size_of::<TextureFormat>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Data Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        (
            raw_data_buffer,
            rf_input_buffer,
            rf_fft_state_buffer,
            rf_value_buffer,
            rf_minmax_buffer,
            cache_buffer,
            texture,
        )
    }
}
