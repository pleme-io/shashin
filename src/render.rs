//! GPU render pipeline for the image viewer and gallery.
//!
//! Implements `madori::RenderCallback` to draw:
//! - Single image viewer with zoom/pan/rotation
//! - Gallery thumbnail grid
//! - Info overlay with metadata
//! - Status bar with position/zoom

use crate::config::ShashinConfig;
use crate::gallery::Gallery;
use crate::input::Mode;
use crate::metadata::{FileInfo, ImageMetadata};
use crate::viewer::ImageViewer;
use garasu::GpuContext;
use glyphon::{Color as GlyphonColor, TextArea, TextBounds};
use madori::render::{RenderCallback, RenderContext};

/// Slideshow state.
#[derive(Debug)]
pub struct SlideshowState {
    /// Whether the slideshow is active.
    pub active: bool,
    /// Time elapsed since last image change.
    pub elapsed: f32,
    /// Interval between images.
    pub interval: f32,
}

impl SlideshowState {
    #[must_use]
    pub fn new(interval: f32) -> Self {
        Self {
            active: false,
            elapsed: 0.0,
            interval,
        }
    }

    /// Update slideshow timer. Returns true if time to advance.
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.active {
            return false;
        }
        self.elapsed += dt;
        if self.elapsed >= self.interval {
            self.elapsed = 0.0;
            true
        } else {
            false
        }
    }

    /// Toggle slideshow on/off.
    pub fn toggle(&mut self) {
        self.active = !self.active;
        self.elapsed = 0.0;
    }
}

/// Main renderer for shashin, implements madori's `RenderCallback`.
pub struct ShashinRenderer {
    /// Image viewer state.
    pub viewer: ImageViewer,
    /// Gallery state.
    pub gallery: Gallery,
    /// Current UI mode.
    pub mode: Mode,
    /// Slideshow state.
    pub slideshow: SlideshowState,
    /// Cached EXIF metadata for the current image.
    pub current_metadata: Option<ImageMetadata>,
    /// Cached file info for the current image.
    pub current_file_info: Option<FileInfo>,

    /// wgpu texture holding the current image pixels.
    image_texture: Option<wgpu::Texture>,
    /// Texture bind group for rendering the image.
    image_bind_group: Option<wgpu::BindGroup>,
    /// Bind group layout for image textures.
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    /// Render pipeline for drawing textured quads.
    render_pipeline: Option<wgpu::RenderPipeline>,
    /// Sampler for image textures.
    sampler: Option<wgpu::Sampler>,
    /// Vertex buffer for fullscreen quad.
    vertex_buffer: Option<wgpu::Buffer>,
    /// Uniform buffer for view transform.
    uniform_buffer: Option<wgpu::Buffer>,
    /// Bind group for uniforms.
    uniform_bind_group: Option<wgpu::BindGroup>,
    /// Bind group layout for uniforms.
    uniform_bind_group_layout: Option<wgpu::BindGroupLayout>,

    /// Background clear color.
    clear_color: wgpu::Color,

    /// Surface texture format (captured during init).
    surface_format: Option<wgpu::TextureFormat>,
}

/// Uniforms passed to the image shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ImageUniforms {
    /// Transform: [offset_x, offset_y, scale_x, scale_y]
    transform: [f32; 4],
    /// Rotation angle in radians.
    rotation: f32,
    /// Flip: [horizontal, vertical] (0.0 or 1.0)
    flip_h: f32,
    flip_v: f32,
    _padding: f32,
}

impl ShashinRenderer {
    /// Create a new renderer with the given config.
    #[must_use]
    pub fn new(config: &ShashinConfig) -> Self {
        let clear_color = parse_hex_color(&config.viewer.background).unwrap_or(wgpu::Color {
            r: 0.180,
            g: 0.204,
            b: 0.251,
            a: 1.0,
        });

        Self {
            viewer: ImageViewer::new(&config.viewer),
            gallery: Gallery::new(&config.gallery),
            mode: Mode::Viewer,
            slideshow: SlideshowState::new(config.slideshow.interval_secs),
            current_metadata: None,
            current_file_info: None,
            image_texture: None,
            image_bind_group: None,
            bind_group_layout: None,
            render_pipeline: None,
            sampler: None,
            vertex_buffer: None,
            uniform_buffer: None,
            uniform_bind_group: None,
            uniform_bind_group_layout: None,
            clear_color,
            surface_format: None,
        }
    }

    /// Upload current image to GPU texture.
    pub fn upload_image(&mut self, gpu: &GpuContext) {
        let Some(ref img) = self.viewer.current_image else {
            return;
        };

        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shashin_image"),
            size: wgpu::Extent3d {
                width: img.width,
                height: img.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &img.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * img.width),
                rows_per_image: Some(img.height),
            },
            wgpu::Extent3d {
                width: img.width,
                height: img.height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create bind group for this texture
        if let Some(ref layout) = self.bind_group_layout {
            if let Some(ref sampler) = self.sampler {
                let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("shashin_image_bind_group"),
                    layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ],
                });
                self.image_bind_group = Some(bind_group);
            }
        }

        self.image_texture = Some(texture);

        // Update metadata
        if let Some(path) = self.viewer.current_path() {
            self.current_metadata = ImageMetadata::from_file(path);
            self.current_file_info = Some(FileInfo::from_path(
                path,
                img.width,
                img.height,
                &img.format,
            ));
        }
    }

    /// Initialize the GPU render pipeline.
    fn init_pipeline(&mut self, gpu: &GpuContext, format: wgpu::TextureFormat) {
        // Texture bind group layout
        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("shashin_texture_bgl"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        // Uniform bind group layout
        let uniform_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("shashin_uniform_bgl"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        // Create sampler
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shashin_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Shader module
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("shashin_shader"),
                source: wgpu::ShaderSource::Wgsl(IMAGE_SHADER.into()),
            });

        // Pipeline layout
        let pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("shashin_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout, &uniform_bind_group_layout],
                    push_constant_ranges: &[],
                });

        // Render pipeline
        let render_pipeline =
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("shashin_render_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: 16, // 4 * f32
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &[
                                wgpu::VertexAttribute {
                                    offset: 0,
                                    shader_location: 0,
                                    format: wgpu::VertexFormat::Float32x2,
                                },
                                wgpu::VertexAttribute {
                                    offset: 8,
                                    shader_location: 1,
                                    format: wgpu::VertexFormat::Float32x2,
                                },
                            ],
                        }],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

        // Vertex buffer for a fullscreen quad (two triangles)
        let vertices: &[f32] = &[
            // pos(x,y), uv(u,v)
            -1.0, -1.0, 0.0, 1.0, // bottom-left
            1.0, -1.0, 1.0, 1.0, // bottom-right
            1.0, 1.0, 1.0, 0.0, // top-right
            -1.0, -1.0, 0.0, 1.0, // bottom-left
            1.0, 1.0, 1.0, 0.0, // top-right
            -1.0, 1.0, 0.0, 0.0, // top-left
        ];

        let vertex_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shashin_vertex_buffer"),
            size: (vertices.len() * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(vertices));

        // Uniform buffer
        let uniforms = ImageUniforms {
            transform: [0.0, 0.0, 1.0, 1.0],
            rotation: 0.0,
            flip_h: 0.0,
            flip_v: 0.0,
            _padding: 0.0,
        };

        let uniform_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shashin_uniform_buffer"),
            size: std::mem::size_of::<ImageUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        gpu.queue
            .write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let uniform_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shashin_uniform_bg"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        self.bind_group_layout = Some(bind_group_layout);
        self.uniform_bind_group_layout = Some(uniform_bind_group_layout);
        self.sampler = Some(sampler);
        self.render_pipeline = Some(render_pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.uniform_buffer = Some(uniform_buffer);
        self.uniform_bind_group = Some(uniform_bind_group);
    }

    /// Update uniform buffer with current view transform.
    fn update_uniforms(&self, gpu: &GpuContext) {
        let Some(ref img) = self.viewer.current_image else {
            return;
        };
        let Some(ref uniform_buffer) = self.uniform_buffer else {
            return;
        };

        let view = &self.viewer.view;
        let (rw, rh) = view.rotated_dimensions(img.width, img.height);
        let win_w = self.viewer.window_width as f32;
        let win_h = self.viewer.window_height as f32;

        if win_w == 0.0 || win_h == 0.0 {
            return;
        }

        // Scale: how much of the window the image occupies
        let scale_x = (rw as f32 * view.zoom) / win_w;
        let scale_y = (rh as f32 * view.zoom) / win_h;

        // Offset: pan as fraction of window
        let offset_x = view.pan_x / win_w * 2.0;
        let offset_y = -view.pan_y / win_h * 2.0; // flip Y

        let uniforms = ImageUniforms {
            transform: [offset_x, offset_y, scale_x, scale_y],
            rotation: view.rotation.radians(),
            flip_h: if view.flip.horizontal { 1.0 } else { 0.0 },
            flip_v: if view.flip.vertical { 1.0 } else { 0.0 },
            _padding: 0.0,
        };

        gpu.queue
            .write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Render a text overlay showing image info.
    fn render_info_overlay(
        &self,
        text: &mut garasu::TextRenderer,
        gpu: &GpuContext,
        width: u32,
        height: u32,
    ) {
        if !self.viewer.show_info {
            return;
        }

        let mut lines = Vec::new();

        // File info
        if let Some(ref info) = self.current_file_info {
            for (label, value) in info.display_lines() {
                lines.push(format!("{label}: {value}"));
            }
        }

        // EXIF metadata
        if let Some(ref meta) = self.current_metadata {
            if !lines.is_empty() && meta.has_any() {
                lines.push(String::new()); // separator
            }
            for (label, value) in meta.display_lines() {
                lines.push(format!("{label}: {value}"));
            }
        }

        if lines.is_empty() {
            lines.push("No metadata available".into());
        }

        let info_text = lines.join("\n");
        let mut buffer = text.create_buffer(&info_text, 14.0, 20.0);
        buffer.set_size(&mut text.font_system, Some(350.0), Some(height as f32));
        buffer.shape_until_scroll(&mut text.font_system, false);

        let text_area = TextArea {
            buffer: &buffer,
            left: (width as f32) - 370.0,
            top: 10.0,
            scale: 1.0,
            bounds: TextBounds {
                left: (width as i32) - 370,
                top: 10,
                right: width as i32 - 10,
                bottom: height as i32 - 10,
            },
            default_color: GlyphonColor::rgba(220, 225, 232, 230),
            custom_glyphs: &[],
        };

        let _ = text.prepare(&gpu.device, &gpu.queue, width, height, [text_area]);
    }

    /// Render the status bar at the bottom.
    fn render_status_bar(
        &self,
        text: &mut garasu::TextRenderer,
        gpu: &GpuContext,
        width: u32,
        height: u32,
    ) {
        let status = match self.mode {
            Mode::Viewer | Mode::Slideshow => {
                let pos = self.viewer.position_display();
                let zoom = self.viewer.zoom_display();
                let rot = self.viewer.view.rotation.degrees();
                let file = self
                    .viewer
                    .current_path()
                    .and_then(|p| p.file_name())
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                let slideshow = if self.slideshow.active {
                    " [SLIDESHOW]"
                } else {
                    ""
                };
                format!("{file}  |  {pos}  |  {zoom}  |  {rot}deg{slideshow}")
            }
            Mode::Gallery => {
                let count = self.gallery.len();
                let sort = format!("{:?}", self.gallery.sort_order());
                let marked = self.gallery.marked_count();
                let dir = self
                    .gallery
                    .directory()
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                let filter = if self.gallery.filter_query().is_empty() {
                    String::new()
                } else {
                    format!("  |  filter: {}", self.gallery.filter_query())
                };
                let mark_str = if marked > 0 {
                    format!("  |  {marked} marked")
                } else {
                    String::new()
                };
                format!("{dir}  |  {count} images  |  sort: {sort}{filter}{mark_str}")
            }
        };

        let mut buffer = text.create_buffer(&status, 12.0, 16.0);
        buffer.set_size(
            &mut text.font_system,
            Some(width as f32),
            Some(20.0),
        );
        buffer.shape_until_scroll(&mut text.font_system, false);

        let bar_y = height.saturating_sub(24);

        let text_area = TextArea {
            buffer: &buffer,
            left: 8.0,
            top: bar_y as f32,
            scale: 1.0,
            bounds: TextBounds {
                left: 0,
                top: bar_y as i32,
                right: width as i32,
                bottom: height as i32,
            },
            default_color: GlyphonColor::rgba(180, 190, 200, 200),
            custom_glyphs: &[],
        };

        let _ = text.prepare(&gpu.device, &gpu.queue, width, height, [text_area]);
    }
}

impl RenderCallback for ShashinRenderer {
    fn init(&mut self, gpu: &GpuContext) {
        // We need the surface format. We'll detect it via adapter capabilities.
        // For now use Bgra8UnormSrgb which is standard for most platforms.
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;
        self.surface_format = Some(format);
        self.init_pipeline(gpu, format);

        // If we already have an image loaded (set before GPU init), upload it
        if self.viewer.current_image.is_some() {
            self.upload_image(gpu);
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.viewer.resize(width, height);
        self.gallery.update_layout(width, height);
    }

    fn render(&mut self, ctx: &mut RenderContext<'_>) {
        // Update viewer animation
        self.viewer.update(ctx.dt);

        // Handle slideshow advancement
        if self.slideshow.tick(ctx.dt) {
            if let Err(e) = self.viewer.next_image() {
                tracing::warn!("slideshow advance failed: {e}");
            } else {
                self.upload_image(ctx.gpu);
            }
        }

        // Begin render pass
        let mut encoder =
            ctx.gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("shashin_encoder"),
                });

        // Clear pass
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shashin_clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: ctx.surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        // Image render pass (only in viewer/slideshow mode with an image)
        if matches!(self.mode, Mode::Viewer | Mode::Slideshow) {
            if let (
                Some(pipeline),
                Some(bind_group),
                Some(uniform_bg),
                Some(vertex_buf),
            ) = (
                &self.render_pipeline,
                &self.image_bind_group,
                &self.uniform_bind_group,
                &self.vertex_buffer,
            ) {
                // Update transform uniforms
                self.update_uniforms(ctx.gpu);

                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("shashin_image"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: ctx.surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.set_bind_group(1, uniform_bg, &[]);
                pass.set_vertex_buffer(0, vertex_buf.slice(..));
                pass.draw(0..6, 0..1);
            }
        }

        ctx.gpu.queue.submit(std::iter::once(encoder.finish()));

        // Text overlays (info panel, status bar)
        // These use a separate prepare/render cycle via glyphon.
        if matches!(self.mode, Mode::Viewer | Mode::Slideshow) && self.viewer.show_info {
            self.render_info_overlay(ctx.text, ctx.gpu, ctx.width, ctx.height);

            let mut pass_encoder =
                ctx.gpu
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("shashin_info_text"),
                    });
            {
                let mut pass = pass_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("shashin_info_text_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: ctx.surface_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                let _ = ctx.text.render(&mut pass);
            }
            ctx.gpu
                .queue
                .submit(std::iter::once(pass_encoder.finish()));
        }

        // Status bar text
        self.render_status_bar(ctx.text, ctx.gpu, ctx.width, ctx.height);
        let mut status_encoder =
            ctx.gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("shashin_status_text"),
                });
        {
            let mut pass = status_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shashin_status_text_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: ctx.surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let _ = ctx.text.render(&mut pass);
        }
        ctx.gpu
            .queue
            .submit(std::iter::once(status_encoder.finish()));
    }
}

/// WGSL shader for rendering images with transform uniforms.
const IMAGE_SHADER: &str = r"
struct Uniforms {
    transform: vec4<f32>,  // offset_x, offset_y, scale_x, scale_y
    rotation: f32,
    flip_h: f32,
    flip_v: f32,
    _padding: f32,
};

@group(1) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Apply scale
    var pos = input.position * vec2<f32>(uniforms.transform.z, uniforms.transform.w);

    // Apply rotation
    let cos_r = cos(uniforms.rotation);
    let sin_r = sin(uniforms.rotation);
    let rotated = vec2<f32>(
        pos.x * cos_r - pos.y * sin_r,
        pos.x * sin_r + pos.y * cos_r,
    );

    // Apply offset
    out.position = vec4<f32>(
        rotated.x + uniforms.transform.x,
        rotated.y + uniforms.transform.y,
        0.0,
        1.0,
    );

    // Apply flip to UV
    var uv = input.uv;
    if (uniforms.flip_h > 0.5) {
        uv.x = 1.0 - uv.x;
    }
    if (uniforms.flip_v > 0.5) {
        uv.y = 1.0 - uv.y;
    }
    out.uv = uv;

    return out;
}

@group(0) @binding(0) var image_texture: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(image_texture, image_sampler, input.uv);
}
";

/// Parse a hex color string to wgpu::Color.
fn parse_hex_color(hex: &str) -> Option<wgpu::Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(wgpu::Color {
        r: f64::from(r) / 255.0,
        g: f64::from(g) / 255.0,
        b: f64::from(b) / 255.0,
        a: 1.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_color_valid() {
        let c = parse_hex_color("#2E3440").unwrap();
        assert!((c.r - 0.180).abs() < 0.01);
        assert!((c.g - 0.204).abs() < 0.01);
        assert!((c.b - 0.251).abs() < 0.01);
        assert!((c.a - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_hex_color_no_hash() {
        let c = parse_hex_color("ECEFF4").unwrap();
        assert!(c.r > 0.9);
    }

    #[test]
    fn parse_hex_color_invalid() {
        assert!(parse_hex_color("invalid").is_none());
        assert!(parse_hex_color("#FFF").is_none());
    }

    #[test]
    fn slideshow_state_inactive() {
        let mut ss = SlideshowState::new(5.0);
        assert!(!ss.active);
        assert!(!ss.tick(1.0));
    }

    #[test]
    fn slideshow_state_active_tick() {
        let mut ss = SlideshowState::new(2.0);
        ss.toggle();
        assert!(ss.active);
        assert!(!ss.tick(1.0)); // 1s < 2s
        assert!(ss.tick(1.5)); // 2.5s >= 2s, should advance
        assert!((ss.elapsed - 0.0).abs() < f32::EPSILON); // reset
    }

    #[test]
    fn slideshow_toggle() {
        let mut ss = SlideshowState::new(5.0);
        assert!(!ss.active);
        ss.toggle();
        assert!(ss.active);
        ss.toggle();
        assert!(!ss.active);
    }

    #[test]
    fn image_uniforms_size() {
        // Ensure the uniform struct is properly aligned for GPU
        assert_eq!(std::mem::size_of::<ImageUniforms>(), 32);
    }
}
