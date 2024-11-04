use super::*;

#[derive(Debug)]
pub struct RenderResources {
    pub(crate) uniforms: Uniforms,
    // compute stuff
    pub(crate) compute_pipeline_layout: wgpu::PipelineLayout,
    pub(crate) compute_pipeline: wgpu::ComputePipeline,
    pub(crate) compute_shader: wgpu::ShaderModule,
    pub(crate) raw_data_buffer: wgpu::Buffer,
    pub(crate) cache_buffer: wgpu::Buffer,
    pub(crate) compute_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) compute_bind_group: wgpu::BindGroup,
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

impl RenderResources {
    pub fn update_uniforms(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniforms: Uniforms,
    ) {
        if self.uniforms.width != uniforms.width || self.uniforms.height != uniforms.height {
            (self.raw_data_buffer, self.cache_buffer, self.texture) = RenderResources::get_buffers(
                device,
                self.texture_format,
                uniforms.width,
                uniforms.height,
            );

            (self.compute_bind_group, self.compute_pipeline) =
                RenderResources::get_compute_pipeline(
                    device,
                    &self.raw_data_buffer,
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

    /// results stored at a cache buffer, copy to texture by calling [Self::refresh_texture]
    pub fn compute_texture(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.push_debug_group("compute texture");
        {
            // compute pass
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.compute_bind_group, &[]);
            const WORKGROUP_SIZE: (u8, u8, u8) = (16, 8, 1);
            let dispatch_x =
                (self.uniforms.width + WORKGROUP_SIZE.0 as u32 - 1) / WORKGROUP_SIZE.0 as u32;
            let dispatch_y =
                (self.uniforms.height + WORKGROUP_SIZE.1 as u32 - 1) / WORKGROUP_SIZE.1 as u32;
            let dispatch_z = 1;
            cpass.dispatch_workgroups(dispatch_x, dispatch_y, dispatch_z);
        }
        encoder.pop_debug_group();
    }

    pub fn refresh_texture(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_buffer_to_texture(
            wgpu::ImageCopyBuffer {
                buffer: &self.cache_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(size_of::<TextureFormat>() as u32 * self.uniforms.width),
                    rows_per_image: Some(self.uniforms.height),
                },
            },
            wgpu::ImageCopyTexture {
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
        rpass.draw(0..3, 0..1); // 绘制三角形
    }
}

impl RenderResources {
    
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn get_compute_pipeline(
        device: &wgpu::Device,
        raw_data: &wgpu::Buffer,
        cache: &wgpu::Buffer,
        uniform: &wgpu::Buffer,
        color_map: &wgpu::Buffer,
        bind_group_layout: &wgpu::BindGroupLayout,
        pipeline_layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
    ) -> (wgpu::BindGroup, wgpu::ComputePipeline) {
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
                        buffer: cache,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: uniform,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: color_map,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
            label: Some("Compute Bind Group"),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(pipeline_layout),
            module: shader,
            entry_point: "main",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        (compute_bind_group, compute_pipeline)
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
        // 创建渲染管道
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
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

        // 创建绑定组
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
    ) -> (wgpu::Buffer, wgpu::Buffer, wgpu::Texture) {
        let raw_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Raw Data Buffer"),
            size: (width * height) as u64 * size_of::<RawDataFormat>() as u64,
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
        (raw_data_buffer, cache_buffer, texture)
    }
}
