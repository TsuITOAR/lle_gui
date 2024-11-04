use std::{hash::Hasher, sync::Arc};

use eframe::{
    egui_wgpu,
    wgpu::{self, util::DeviceExt},
};
mod colormap;
pub use colormap::COLORMAP;
mod vertex;
use egui::mutex::Mutex;
pub use vertex::Vertex;

mod resource;
use resource::RenderResources;

mod trait_impl;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub width: u32,
    pub height: u32,
    pub z_range: [f32; 2],
}

pub(crate) type RawDataFormat = f32;
pub(crate) type TextureFormat = u32;

type ResourceStore = std::collections::BTreeMap<u64, RenderResources>;

#[derive(Clone)]
pub struct Drawer {
    // device: wgpu::Device,
    // queue: wgpu::Queue,
    // config: wgpu::SurfaceConfiguration,
    // compute shader stuff
    name_hash: u64,
    uniforms: Uniforms,
    data: Arc<Mutex<Vec<RawDataFormat>>>,
    current_row: u32,
}

impl std::fmt::Debug for Drawer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Drawer")
            .field("uniforms", &self.uniforms)
            .field("data", &self.data.lock())
            .field("current_row", &self.current_row)
            .finish()
    }
}

impl Drawer {
    pub fn new(name: &str, width: u32, height: u32, render_state: &egui_wgpu::RenderState) -> Self {
        let device = &render_state.device;

        let uniforms = Uniforms {
            width,
            height,
            z_range: [0.0, 1.0],
        };
        let texture_format = render_state.target_format;
        let (raw_data_buffer, cache_buffer, texture) =
            RenderResources::get_buffers(device, texture_format, uniforms.width, uniforms.height);

        // Create a uniform buffer for min-max values
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let colormap = colormap::get_colormap::<256>(COLORMAP, texture_format);

        // 创建颜色映射表缓冲区
        let colormap_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Colormap Buffer"),
            contents: bytemuck::cast_slice(&colormap),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Create a compute shader module
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        });

        // Create a bind group for the compute pipeline
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[
                    // Raw data buffer
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
                    // Cache data buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Uniform buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Colormap buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create a compute pipeline
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let (compute_bind_group, compute_pipeline) = RenderResources::get_compute_pipeline(
            device,
            &raw_data_buffer,
            &cache_buffer,
            &uniform_buffer,
            &colormap_buffer,
            &compute_bind_group_layout,
            &compute_pipeline_layout,
            &compute_shader,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 创建采样器
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        // 创建绑定组布局
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        // 创建渲染管道布局
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        // 创建顶点数据
        const VERTICES: &[Vertex] = &[
            Vertex {
                position: [-1.0, 3.0, 0.0],
            },
            Vertex {
                position: [-1.0, -1.0, 0.0],
            },
            Vertex {
                position: [3.0, -1.0, 0.0],
            },
        ];

        // 创建顶点缓冲区
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // 创建着色器模块
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // 创建渲染管道
        let (render_bind_group, render_pipeline) = RenderResources::get_render_pipeline(
            device,
            texture_format,
            &texture_view,
            &sampler,
            &render_bind_group_layout,
            &render_pipeline_layout,
            &render_shader,
        );

        let data = vec![0.0; (uniforms.width * uniforms.height) as usize];

        let resource = RenderResources {
            uniforms,
            compute_pipeline_layout,
            compute_pipeline,
            compute_shader,
            raw_data_buffer,
            cache_buffer,
            compute_bind_group_layout,
            compute_bind_group,
            render_pipeline_layout,
            render_pipeline,
            render_shader,
            uniform_buffer,
            _colormap_buffer: colormap_buffer,
            sampler,
            texture,
            texture_format,
            vertex_buffer,
            render_bind_group_layout,
            render_bind_group,
        };

        let mut hash = std::hash::DefaultHasher::new();
        hash.write_str(name);
        let name_hash = hash.finish();

        let callback_resource = &mut render_state.renderer.write().callback_resources;
        if !callback_resource.contains::<ResourceStore>() {
            callback_resource.insert(ResourceStore::new());
        }

        let map = callback_resource.get_mut::<ResourceStore>().unwrap();

        map.insert(name_hash, resource);

        Self {
            name_hash,
            uniforms,
            data: Arc::new(Mutex::new(data)),
            current_row: 0,
        }
    }

    pub fn push_row(&mut self, row_data: &[f32], range: [f32; 2]) {
        let start = (self.current_row * self.uniforms.width) as usize;
        let end = start + self.uniforms.width as usize;
        self.data.lock()[start..end].copy_from_slice(row_data);
        self.current_row = (self.current_row + 1) % self.uniforms.height;
        if range[0] < self.uniforms.z_range[0] || range[1] > self.uniforms.z_range[1] {
            self.uniforms.z_range[0] = self.uniforms.z_range[0].min(range[0]);
            self.uniforms.z_range[1] = self.uniforms.z_range[1].max(range[1]);
        }
        /* self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        ); */
    }

    pub(crate) fn data(&self) -> egui::mutex::MutexGuard<'_, Vec<RawDataFormat>> {
        self.data.lock()
    }

    pub(crate) fn uniforms(&self) -> Uniforms {
        self.uniforms
    }

    pub(crate) fn uniforms_mut(&mut self) -> &mut Uniforms {
        &mut self.uniforms
    }
}

impl Drawer {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        //log::info!("Showing drawer");
        let rect = ui.max_rect();
        /* let (rect, response) =
        ui.allocate_exact_size(egui::Vec2::splat(300.0), egui::Sense::drag()); */
        ui.painter()
            .add(egui_wgpu::Callback::new_paint_callback(rect, self.clone()));
        /* egui::Frame::canvas(ui.style())
        .show(ui, |ui| {
            self.draw(ui);
        })
        .response */
    }

    /* fn draw(&self, ui: &mut egui::Ui) {
        //let rect = ui.max_rect();
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::splat(300.0), egui::Sense::drag());

        let cb = egui_wgpu::Callback::new_paint_callback(rect, self.clone());

        let callback = egui::PaintCallback {
            rect,
            callback: std::sync::Arc::new(cb),
        };

        ui.painter().add(callback);
    } */
}

impl egui_wgpu::CallbackTrait for Drawer {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        //log::info!("Preparing drawer");
        let resource: &mut RenderResources = callback_resources
            .get_mut::<ResourceStore>()
            .unwrap()
            .get_mut(&self.name_hash)
            .unwrap();
        resource.update_uniforms(device, queue, self.uniforms);

        resource.update_raw_buffer(device, queue, bytemuck::cast_slice(&self.data.lock()));
        resource.compute_texture(egui_encoder);
        resource.refresh_texture(egui_encoder);
        vec![]
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        //log::info!("Painting drawer");
        let resource: &RenderResources = callback_resources
            .get::<ResourceStore>()
            .unwrap()
            .get(&self.name_hash)
            .unwrap();
        resource.paint(render_pass);
    }
}
