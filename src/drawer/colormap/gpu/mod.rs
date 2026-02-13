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

mod axis;
mod trait_impl;
use crate::drawer::processor::Component;

fn compute_shader_source() -> String {
    format!(
        "{}\n{}\n{}",
        include_str!("utils.wgsl"),
        include_str!("fft.wgsl"),
        include_str!("compute.wgsl")
    )
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub width: u32,
    pub height: u32,
    pub z_range: [f32; 2],
    pub compute_mode: u32,
    pub rf_db_scale: u32,
    pub rf_global_norm: u32,
    pub raw_component: u32,
    pub raw_db_scale: u32,
    pub raw_gpu_range: u32,
    pub _padding: [u32; 2],
}

pub(crate) type RawDataFormat = [f32; 2];
pub(crate) type RfInputFormat = [f32; 2];
pub(crate) type TextureFormat = u32;

type ResourceStore = std::collections::BTreeMap<u64, RenderResources>;

#[derive(Debug, Default, Clone)]
struct RfGpuInputCache {
    start: usize,
    history_len: usize,
    time_len: usize,
    data: Vec<RfInputFormat>,
}

#[derive(Clone)]
pub struct Drawer {
    // device: wgpu::Device,
    // queue: wgpu::Queue,
    // config: wgpu::SurfaceConfiguration,
    // compute shader stuff
    name_hash: u64,
    uniforms: Uniforms,
    data: Arc<Mutex<Vec<RawDataFormat>>>,
    rf_input: Arc<Mutex<Vec<RfInputFormat>>>,
    rf_gpu_cache: Arc<Mutex<RfGpuInputCache>>,
    current_row: u32,
    axis_drawer: axis::AxisDrawer,
}

impl std::fmt::Debug for Drawer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Drawer")
            .field("uniforms", &self.uniforms)
            .field("data", &self.data.lock())
            .field("rf_input", &self.rf_input.lock())
            .field("rf_gpu_cache", &self.rf_gpu_cache.lock())
            .field("current_row", &self.current_row)
            .field("axis_drawer", &self.axis_drawer)
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
            compute_mode: 0,
            rf_db_scale: 0,
            rf_global_norm: 0,
            raw_component: 2,
            raw_db_scale: 0,
            raw_gpu_range: 0,
            _padding: [0; 2],
        };
        let texture_format = render_state.target_format;
        let (
            raw_data_buffer,
            rf_input_buffer,
            rf_fft_state_buffer,
            rf_value_buffer,
            rf_minmax_buffer,
            cache_buffer,
            texture,
        ) = RenderResources::get_buffers(device, texture_format, uniforms.width, uniforms.height);

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
            source: wgpu::ShaderSource::Wgsl(compute_shader_source().into()),
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
                    // RF FFT input (complex) buffer
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
                    // Cache data buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // RF value buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // RF min/max buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Cache data buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
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
                        binding: 6,
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
                        binding: 7,
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

        let (
            compute_bind_group,
            compute_pipeline_raw_reduce,
            compute_pipeline_raw,
            compute_pipeline_rf_transpose,
            compute_pipeline_rf_stage1,
            compute_pipeline_rf_reduce_global,
            compute_pipeline_rf_stage2,
            fft_cfg,
        ) = RenderResources::get_compute_pipelines(
            device,
            &raw_data_buffer,
            &rf_input_buffer,
            &rf_fft_state_buffer,
            &rf_value_buffer,
            &rf_minmax_buffer,
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

        let data = vec![[0.0, 0.0]; (uniforms.width * uniforms.height) as usize];
        let rf_input = vec![[0.0, 0.0]; (uniforms.width * uniforms.height) as usize];

        let resource = RenderResources {
            uniforms,
            compute_pipeline_layout,
            compute_pipeline_raw_reduce,
            compute_pipeline_raw,
            compute_pipeline_rf_transpose,
            compute_pipeline_rf_stage1,
            compute_pipeline_rf_reduce_global,
            compute_pipeline_rf_stage2,
            compute_shader,
            raw_data_buffer,
            rf_input_buffer,
            rf_fft_state_buffer,
            rf_value_buffer,
            rf_minmax_buffer,
            cache_buffer,
            compute_bind_group_layout,
            compute_bind_group,
            fft_cfg,
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

        let axis_drawer = axis::AxisDrawer {
            x_range: 0.0f32..=(uniforms.width - 1) as f32,
            y_range: 0.0f32..=(uniforms.height - 1) as f32,
            ..Default::default()
        };

        Self {
            name_hash,
            uniforms,
            data: Arc::new(Mutex::new(data)),
            rf_input: Arc::new(Mutex::new(rf_input)),
            rf_gpu_cache: Arc::new(Mutex::new(RfGpuInputCache::default())),
            current_row: 0,
            axis_drawer,
        }
    }

    /* pub fn push_row(&mut self, row_data: &[f32], range: [f32; 2]) {
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
    } */

    pub(crate) fn data(&self) -> egui::mutex::MutexGuard<'_, Vec<RawDataFormat>> {
        self.data.lock()
    }

    pub(crate) fn rf_input(&self) -> egui::mutex::MutexGuard<'_, Vec<RfInputFormat>> {
        self.rf_input.lock()
    }

    pub(crate) fn uniforms(&self) -> Uniforms {
        self.uniforms
    }

    pub(crate) fn set_height(&mut self, height: u32) {
        self.set_size(self.uniforms.width, height);
    }

    pub(crate) fn set_size(&mut self, width: u32, height: u32) {
        if width <= 1 || height <= 1 {
            return;
        }
        self.uniforms.width = width;
        self.uniforms.height = height;
        self.axis_drawer.x_range = 0.0f32..=(width - 1) as f32;
        self.axis_drawer.y_range = 0.0f32..=(height - 1) as f32;
        let len = (self.uniforms().height * self.uniforms().width) as usize;
        self.data().resize(len, [0.0, 0.0]);
        self.rf_input().resize(len, [0.0, 0.0]);
    }

    pub(crate) fn set_z_range(&mut self, range: [f32; 2]) {
        self.uniforms.z_range = range;
    }

    pub(crate) fn set_raw_mode(&mut self, component: Component, db_scale: bool, gpu_range: bool) {
        self.uniforms.compute_mode = 0;
        self.uniforms.rf_global_norm = 0;
        self.uniforms.raw_component = component as u32;
        self.uniforms.raw_db_scale = u32::from(db_scale);
        self.uniforms.raw_gpu_range = u32::from(gpu_range);
    }

    pub(crate) fn set_rf_fft_input(
        &mut self,
        width: usize,
        height: usize,
        data: &[RfInputFormat],
        db_scale: bool,
        global_norm: bool,
    ) {
        self.set_size(width as u32, height as u32);
        self.uniforms.compute_mode = 1;
        self.uniforms.rf_db_scale = u32::from(db_scale);
        self.uniforms.rf_global_norm = u32::from(global_norm);
        self.uniforms.z_range = [0.0, 1.0];
        let mut rf = self.rf_input();
        if rf.len() != data.len() {
            rf.resize(data.len(), [0.0, 0.0]);
        }
        rf.copy_from_slice(data);
    }
}

impl Drawer {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        //log::info!("Showing drawer");
        let max_rect = ui.max_rect();
        let (rect, _, _) = self.axis_drawer.get_remained_rect(max_rect);

        /* let (rect, response) =
        ui.allocate_exact_size(egui::Vec2::splat(300.0), egui::Sense::drag()); */
        ui.painter()
            .add(egui_wgpu::Callback::new_paint_callback(rect, self.clone()));

        self.axis_drawer
            .draw_axes_with_labels_and_ticks(ui, max_rect);
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

        match self.uniforms.compute_mode {
            0 => resource.update_raw_buffer(device, queue, bytemuck::cast_slice(&self.data.lock())),
            1 => resource.update_rf_input_buffer(
                device,
                queue,
                bytemuck::cast_slice(&self.rf_input.lock()),
            ),
            _ => {}
        }
        resource.compute_texture(egui_encoder);
        resource.refresh_texture(egui_encoder);
        Vec::new()
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

#[cfg(all(test, feature = "gpu", not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use eframe::wgpu;
    use eframe::wgpu::util::DeviceExt;

    #[test]
    fn compute_pipeline_setup_is_valid() {
        let runtime = tokio::runtime::Runtime::new().expect("create tokio runtime");
        runtime.block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let adapter = match instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
            {
                Ok(adapter) => adapter,
                Err(err) => {
                    eprintln!("skip gpu setup test: request_adapter failed: {err}");
                    return;
                }
            };
            let (device, _queue) = match adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
            {
                Ok(ok) => ok,
                Err(err) => {
                    eprintln!("skip gpu setup test: request_device failed: {err}");
                    return;
                }
            };

            let uniforms = Uniforms {
                width: 8,
                height: 8,
                z_range: [0.0, 1.0],
                compute_mode: 1,
                rf_db_scale: 1,
                rf_global_norm: 1,
                raw_component: 2,
                raw_db_scale: 0,
                raw_gpu_range: 0,
                _padding: [0; 2],
            };
            let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("test uniform"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let colormap = colormap::get_colormap::<256>(COLORMAP, wgpu::TextureFormat::Rgba8Unorm);
            let colormap_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("test colormap"),
                contents: bytemuck::cast_slice(&colormap),
                usage: wgpu::BufferUsages::STORAGE,
            });
            let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Compute Shader"),
                source: wgpu::ShaderSource::Wgsl(compute_shader_source().into()),
            });
            let compute_bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Compute Bind Group Layout"),
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
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 7,
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
            let compute_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Compute Pipeline Layout"),
                    bind_group_layouts: &[&compute_bind_group_layout],
                    push_constant_ranges: &[],
                });
            let (raw_data, rf_input, rf_fft_state, rf_values, rf_minmax, cache, _texture) =
                RenderResources::get_buffers(&device, wgpu::TextureFormat::Rgba8Unorm, 8, 8);

            let (_bind_group, _raw_reduce, _raw, _transpose, _stage1, _reduce, _stage2, _fft_cfg) =
                RenderResources::get_compute_pipelines(
                    &device,
                    &raw_data,
                    &rf_input,
                    &rf_fft_state,
                    &rf_values,
                    &rf_minmax,
                    &cache,
                    &uniform_buffer,
                    &colormap_buffer,
                    &compute_bind_group_layout,
                    &compute_pipeline_layout,
                    &compute_shader,
                );
        });
    }
}
