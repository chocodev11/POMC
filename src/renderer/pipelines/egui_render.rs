use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme, Allocator};
use gpu_allocator::MemoryLocation;

use crate::renderer::shader;
use crate::renderer::util;

pub struct EguiRenderer {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    screen_layout: vk::DescriptorSetLayout,
    texture_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    screen_set: vk::DescriptorSet,
    screen_buffer: vk::Buffer,
    screen_allocation: Option<Allocation>,
    sampler: vk::Sampler,
    textures: HashMap<egui::TextureId, ManagedTexture>,
    vertex_buffer: Option<(vk::Buffer, Allocation, usize)>,
    index_buffer: Option<(vk::Buffer, Allocation, usize)>,
}

struct ManagedTexture {
    image: vk::Image,
    view: vk::ImageView,
    allocation: Allocation,
    descriptor_set: vk::DescriptorSet,
    staging_buffer: vk::Buffer,
    staging_allocation: Allocation,
}

impl EguiRenderer {
    pub fn new(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        allocator: &Arc<Mutex<Allocator>>,
    ) -> Self {
        let screen_layout = util::create_descriptor_set_layout(
            device,
            vk::DescriptorType::UNIFORM_BUFFER,
            vk::ShaderStageFlags::VERTEX,
        );

        let texture_bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        let texture_layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&texture_bindings);
        let texture_layout =
            unsafe { device.create_descriptor_set_layout(&texture_layout_info, None) }
                .expect("failed to create egui texture layout");

        let layouts = [screen_layout, texture_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&layouts);
        let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_info, None) }
            .expect("failed to create egui pipeline layout");

        let pipeline = create_pipeline(device, render_pass, pipeline_layout);

        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: 64,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLER,
                descriptor_count: 64,
            },
        ];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(65)
            .pool_sizes(&pool_sizes);
        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None) }
            .expect("failed to create egui descriptor pool");

        let screen_layouts = [screen_layout];
        let screen_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&screen_layouts);
        let screen_set = unsafe { device.allocate_descriptor_sets(&screen_alloc_info) }
            .expect("failed to allocate screen descriptor set")[0];

        let (screen_buffer, screen_allocation) = create_mapped_buffer(
            device,
            allocator,
            8,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            "egui_screen",
        );

        let buffer_info = [vk::DescriptorBufferInfo {
            buffer: screen_buffer,
            offset: 0,
            range: 8,
        }];
        let write = vk::WriteDescriptorSet::default()
            .dst_set(screen_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&buffer_info);
        unsafe { device.update_descriptor_sets(&[write], &[]) };

        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE);
        let sampler = unsafe { device.create_sampler(&sampler_info, None) }
            .expect("failed to create egui sampler");

        Self {
            pipeline,
            pipeline_layout,
            screen_layout,
            texture_layout,
            descriptor_pool,
            screen_set,
            screen_buffer,
            screen_allocation: Some(screen_allocation),
            sampler,
            textures: HashMap::new(),
            vertex_buffer: None,
            index_buffer: None,
        }
    }

    pub fn update_texture(
        &mut self,
        device: &ash::Device,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        allocator: &Arc<Mutex<Allocator>>,
        id: egui::TextureId,
        delta: &egui::epaint::ImageDelta,
    ) {
        let pixels = match &delta.image {
            egui::ImageData::Color(img) => {
                img.pixels
                    .iter()
                    .flat_map(|c| c.to_array())
                    .collect::<Vec<u8>>()
            }
            egui::ImageData::Font(img) => {
                img.srgba_pixels(None)
                    .flat_map(|c| c.to_array())
                    .collect::<Vec<u8>>()
            }
        };

        let [w, h] = delta.image.size().map(|v| v as u32);

        if delta.pos.is_some() {
            // Partial texture updates require sub-image copies; egui only uses
            // them for incremental font atlas growth which is rare enough to skip
            log::debug!("Partial egui texture update ignored for {:?}", id);
            return;
        }

        self.free_texture(device, allocator, id);

        let (image, view, allocation) =
            util::create_gpu_image(device, allocator, w, h, "egui_texture");
        let (staging_buffer, staging_allocation) =
            util::create_staging_buffer(device, allocator, &pixels, "egui_staging");
        util::upload_image(device, queue, command_pool, staging_buffer, image, w, h);

        let tex_layouts = [self.texture_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&tex_layouts);
        let descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info) }
            .expect("failed to allocate egui texture descriptor set")[0];

        let image_info = [vk::DescriptorImageInfo {
            sampler: vk::Sampler::null(),
            image_view: view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        }];
        let sampler_info = [vk::DescriptorImageInfo {
            sampler: self.sampler,
            image_view: vk::ImageView::null(),
            image_layout: vk::ImageLayout::UNDEFINED,
        }];
        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(&image_info),
            vk::WriteDescriptorSet::default()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(&sampler_info),
        ];
        unsafe { device.update_descriptor_sets(&writes, &[]) };

        self.textures.insert(
            id,
            ManagedTexture {
                image,
                view,
                allocation,
                descriptor_set,
                staging_buffer,
                staging_allocation,
            },
        );
    }

    pub fn free_texture(
        &mut self,
        device: &ash::Device,
        allocator: &Arc<Mutex<Allocator>>,
        id: egui::TextureId,
    ) {
        if let Some(tex) = self.textures.remove(&id) {
            let mut alloc = allocator.lock().unwrap();
            unsafe {
                device.destroy_image_view(tex.view, None);
                device.destroy_image(tex.image, None);
                device.destroy_buffer(tex.staging_buffer, None);
            }
            alloc.free(tex.allocation).ok();
            alloc.free(tex.staging_allocation).ok();
        }
    }

    pub fn render(
        &mut self,
        device: &ash::Device,
        allocator: &Arc<Mutex<Allocator>>,
        cmd: vk::CommandBuffer,
        primitives: &[egui::ClippedPrimitive],
        screen_size: [f32; 2],
        pixels_per_point: f32,
    ) {
        if primitives.is_empty() {
            return;
        }

        let screen_data = screen_size;
        self.screen_allocation.as_mut().unwrap().mapped_slice_mut().unwrap()[..8]
            .copy_from_slice(bytemuck::cast_slice(&screen_data));

        let mut total_vertices = 0usize;
        let mut total_indices = 0usize;
        for prim in primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = &prim.primitive {
                total_vertices += mesh.vertices.len();
                total_indices += mesh.indices.len();
            }
        }

        if total_vertices == 0 {
            return;
        }

        let vertex_size = total_vertices * 20;
        let index_size = total_indices * 4;

        self.ensure_buffer(device, allocator, vertex_size, true);
        self.ensure_buffer(device, allocator, index_size, false);

        let (vb, va, _) = self.vertex_buffer.as_mut().unwrap();
        let vertex_slice = va.mapped_slice_mut().unwrap();
        let mut v_offset = 0usize;

        let (ib, ia, _) = self.index_buffer.as_mut().unwrap();
        let index_slice = ia.mapped_slice_mut().unwrap();
        let mut i_offset = 0usize;

        let vb_handle = *vb;
        let ib_handle = *ib;

        for prim in primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = &prim.primitive {
                let vb_bytes = bytemuck::cast_slice::<egui::epaint::Vertex, u8>(&mesh.vertices);
                vertex_slice[v_offset..v_offset + vb_bytes.len()].copy_from_slice(vb_bytes);
                v_offset += vb_bytes.len();

                let ib_bytes = bytemuck::cast_slice::<u32, u8>(&mesh.indices);
                index_slice[i_offset..i_offset + ib_bytes.len()].copy_from_slice(ib_bytes);
                i_offset += ib_bytes.len();
            }
        }

        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline);
            device.cmd_bind_vertex_buffers(cmd, 0, &[vb_handle], &[0]);
            device.cmd_bind_index_buffer(cmd, ib_handle, 0, vk::IndexType::UINT32);
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.screen_set],
                &[],
            );
        }

        let mut vertex_base = 0i32;
        let mut index_base = 0u32;

        let physical_w = (screen_size[0] * pixels_per_point) as u32;
        let physical_h = (screen_size[1] * pixels_per_point) as u32;

        for prim in primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = &prim.primitive {
                if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                    continue;
                }

                let clip = &prim.clip_rect;
                let min_x = (clip.min.x * pixels_per_point).round() as i32;
                let min_y = (clip.min.y * pixels_per_point).round() as i32;
                let max_x = (clip.max.x * pixels_per_point).round() as i32;
                let max_y = (clip.max.y * pixels_per_point).round() as i32;

                let x = min_x.max(0);
                let y = min_y.max(0);
                let w = (max_x - x).max(0) as u32;
                let h = (max_y - y).max(0) as u32;

                if w == 0 || h == 0 || x as u32 >= physical_w || y as u32 >= physical_h {
                    vertex_base += mesh.vertices.len() as i32;
                    index_base += mesh.indices.len() as u32;
                    continue;
                }

                let scissor = vk::Rect2D {
                    offset: vk::Offset2D { x, y },
                    extent: vk::Extent2D {
                        width: w.min(physical_w.saturating_sub(x as u32)),
                        height: h.min(physical_h.saturating_sub(y as u32)),
                    },
                };
                unsafe { device.cmd_set_scissor(cmd, 0, &[scissor]) };

                if let Some(tex) = self.textures.get(&mesh.texture_id) {
                    unsafe {
                        device.cmd_bind_descriptor_sets(
                            cmd,
                            vk::PipelineBindPoint::GRAPHICS,
                            self.pipeline_layout,
                            1,
                            &[tex.descriptor_set],
                            &[],
                        );
                    }
                }

                unsafe {
                    device.cmd_draw_indexed(
                        cmd,
                        mesh.indices.len() as u32,
                        1,
                        index_base,
                        vertex_base,
                        0,
                    );
                }

                vertex_base += mesh.vertices.len() as i32;
                index_base += mesh.indices.len() as u32;
            }
        }
    }

    fn ensure_buffer(
        &mut self,
        device: &ash::Device,
        allocator: &Arc<Mutex<Allocator>>,
        required: usize,
        is_vertex: bool,
    ) {
        let buf = if is_vertex {
            &mut self.vertex_buffer
        } else {
            &mut self.index_buffer
        };

        if let Some((_, _, cap)) = buf {
            if *cap >= required {
                return;
            }
        }

        if let Some((b, a, _)) = buf.take() {
            unsafe { device.destroy_buffer(b, None) };
            allocator.lock().unwrap().free(a).ok();
        }

        let size = required.next_power_of_two().max(4096);
        let usage = if is_vertex {
            vk::BufferUsageFlags::VERTEX_BUFFER
        } else {
            vk::BufferUsageFlags::INDEX_BUFFER
        };
        let name = if is_vertex { "egui_vertices" } else { "egui_indices" };

        let (buffer, allocation) =
            create_mapped_buffer(device, allocator, size as u64, usage, name);
        *buf = Some((buffer, allocation, size));
    }

    pub fn destroy(&mut self, device: &ash::Device, allocator: &Arc<Mutex<Allocator>>) {
        let mut alloc = allocator.lock().unwrap();

        for (_, tex) in self.textures.drain() {
            unsafe {
                device.destroy_image_view(tex.view, None);
                device.destroy_image(tex.image, None);
                device.destroy_buffer(tex.staging_buffer, None);
            }
            alloc.free(tex.allocation).ok();
            alloc.free(tex.staging_allocation).ok();
        }

        if let Some((b, a, _)) = self.vertex_buffer.take() {
            unsafe { device.destroy_buffer(b, None) };
            alloc.free(a).ok();
        }
        if let Some((b, a, _)) = self.index_buffer.take() {
            unsafe { device.destroy_buffer(b, None) };
            alloc.free(a).ok();
        }

        unsafe { device.destroy_buffer(self.screen_buffer, None) };
        if let Some(a) = self.screen_allocation.take() {
            alloc.free(a).ok();
        }

        unsafe { device.destroy_sampler(self.sampler, None) };

        drop(alloc);

        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.screen_layout, None);
            device.destroy_descriptor_set_layout(self.texture_layout, None);
        }
    }
}

fn create_mapped_buffer(
    device: &ash::Device,
    allocator: &Arc<Mutex<Allocator>>,
    size: u64,
    usage: vk::BufferUsageFlags,
    name: &str,
) -> (vk::Buffer, Allocation) {
    let buffer_info = vk::BufferCreateInfo::default()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = unsafe { device.create_buffer(&buffer_info, None) }
        .expect("failed to create buffer");
    let mem_reqs = unsafe { device.get_buffer_memory_requirements(buffer) };

    let allocation = allocator
        .lock()
        .unwrap()
        .allocate(&AllocationCreateDesc {
            name,
            requirements: mem_reqs,
            location: MemoryLocation::CpuToGpu,
            linear: true,
            allocation_scheme: AllocationScheme::GpuAllocatorManaged,
        })
        .expect("failed to allocate buffer memory");

    unsafe {
        device
            .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
            .expect("failed to bind buffer memory");
    }

    (buffer, allocation)
}

fn create_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    layout: vk::PipelineLayout,
) -> vk::Pipeline {
    let vert_spv = shader::include_spirv!("egui.vert.spv");
    let frag_spv = shader::include_spirv!("egui.frag.spv");

    let vert_module = shader::create_shader_module(device, vert_spv);
    let frag_module = shader::create_shader_module(device, frag_spv);

    let entry = c"main";
    let stages = [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(entry),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(entry),
    ];

    // egui::epaint::Vertex: pos(2xf32) + color(4xu8) + uv(2xf32) = 20 bytes
    let binding_descs = [vk::VertexInputBindingDescription {
        binding: 0,
        stride: 20,
        input_rate: vk::VertexInputRate::VERTEX,
    }];

    let attr_descs = [
        vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32_SFLOAT,
            offset: 0,
        },
        // UV comes after color in shader, but in memory: pos(0) + color(8) + uv(12)
        vk::VertexInputAttributeDescription {
            location: 1,
            binding: 0,
            format: vk::Format::R32G32_SFLOAT,
            offset: 12,
        },
        vk::VertexInputAttributeDescription {
            location: 2,
            binding: 0,
            format: vk::Format::R8G8B8A8_UNORM,
            offset: 8,
        },
    ];

    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&binding_descs)
        .vertex_attribute_descriptions(&attr_descs);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .line_width(1.0);

    let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(false)
        .depth_write_enable(false);

    // Premultiplied alpha blending
    let blend_attachment = [vk::PipelineColorBlendAttachmentState {
        blend_enable: vk::TRUE,
        src_color_blend_factor: vk::BlendFactor::ONE,
        dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ONE,
        dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
        alpha_blend_op: vk::BlendOp::ADD,
        color_write_mask: vk::ColorComponentFlags::RGBA,
    }];
    let color_blending =
        vk::PipelineColorBlendStateCreateInfo::default().attachments(&blend_attachment);

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let pipeline_info = [vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .depth_stencil_state(&depth_stencil)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0)];

    let pipeline = unsafe {
        device.create_graphics_pipelines(vk::PipelineCache::null(), &pipeline_info, None)
    }
    .expect("failed to create egui pipeline")[0];

    unsafe {
        device.destroy_shader_module(vert_module, None);
        device.destroy_shader_module(frag_module, None);
    }

    pipeline
}
