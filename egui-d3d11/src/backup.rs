use std::{cell::RefCell, mem::zeroed};
use windows::Win32::{
    Foundation::RECT,
    Graphics::{
        Direct3D::D3D_PRIMITIVE_TOPOLOGY,
        Direct3D11::{
            ID3D11BlendState, ID3D11Buffer, ID3D11ClassInstance, ID3D11DepthStencilState,
            ID3D11DeviceContext, ID3D11GeometryShader, ID3D11InputLayout, ID3D11PixelShader,
            ID3D11RasterizerState, ID3D11SamplerState, ID3D11ShaderResourceView,
            ID3D11VertexShader, D3D11_VIEWPORT,
            D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE,
        },
        Dxgi::Common::DXGI_FORMAT,
    },
};

#[derive(Default)]
pub struct BackupState(RefCell<InnerState>);

impl BackupState {
    #[inline]
    pub fn save(&self, context: &ID3D11DeviceContext) {
        unsafe {
            self.0.borrow_mut().save(context);
        }
    }

    #[inline]
    pub fn restore(&self, context: &ID3D11DeviceContext) {
        unsafe {
            self.0.borrow_mut().restore(context);
        }
    }
}

#[derive(Default)]
struct InnerState {
    scissor_rects: [RECT; D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE as _],
    scissor_count: u32,

    viewports: [D3D11_VIEWPORT; D3D11_VIEWPORT_AND_SCISSORRECT_OBJECT_COUNT_PER_PIPELINE as _],
    viewport_count: u32,

    raster_state: Option<ID3D11RasterizerState>,

    blend_state: Option<ID3D11BlendState>,
    blend_factor: [f32; 4],
    blend_mask: u32,

    depth_stencil_state: Option<ID3D11DepthStencilState>,
    stencil_ref: u32,

    pixel_shader_resource: Option<ID3D11ShaderResourceView>,

    sampler: Option<ID3D11SamplerState>,

    vertex_shader: Option<ID3D11VertexShader>,
    vertex_shader_instances: ClassInstances,
    vertex_shader_instances_count: u32,

    geometry_shader: Option<ID3D11GeometryShader>,
    geometry_shader_instances: ClassInstances,
    geomentry_shader_instances_count: u32,

    pixel_shader: Option<ID3D11PixelShader>,
    pixel_shader_instances: ClassInstances,
    pixel_shader_instances_count: u32,

    constant_buffer: Option<ID3D11Buffer>,
    primitive_topology: D3D_PRIMITIVE_TOPOLOGY,

    index_buffer: Option<ID3D11Buffer>,
    index_buffer_format: DXGI_FORMAT,
    index_buffer_offest: u32,

    vertex_buffer: Option<ID3D11Buffer>,
    vertex_buffer_strides: u32,
    vertex_buffer_offsets: u32,

    input_layout: Option<ID3D11InputLayout>,
}

impl InnerState {
    #[inline]
    pub unsafe fn save(&mut self, ctx: &ID3D11DeviceContext) {
        ctx.RSGetScissorRects(&mut self.scissor_count, self.scissor_rects.as_mut_ptr());
        ctx.RSGetViewports(&mut self.viewport_count, self.viewports.as_mut_ptr());
        ctx.RSGetState(&mut self.raster_state);
        ctx.OMGetBlendState(
            &mut self.blend_state,
            self.blend_factor.as_mut_ptr(),
            &mut self.blend_mask,
        );
        ctx.OMGetDepthStencilState(&mut self.depth_stencil_state, &mut self.stencil_ref);
        ctx.PSGetShaderResources(0, 1, &mut self.pixel_shader_resource);
        ctx.PSGetSamplers(0, 1, &mut self.sampler);
        self.pixel_shader_instances_count = 256;
        self.vertex_shader_instances_count = 256;
        self.geomentry_shader_instances_count = 256;

        ctx.PSGetShader(
            &mut self.pixel_shader,
            self.pixel_shader_instances.as_mut_ptr(),
            &mut self.pixel_shader_instances_count,
        );
        ctx.VSGetShader(
            &mut self.vertex_shader,
            self.vertex_shader_instances.as_mut_ptr(),
            &mut self.vertex_shader_instances_count,
        );
        ctx.GSGetShader(
            &mut self.geometry_shader,
            self.geometry_shader_instances.as_mut_ptr(),
            &mut self.geomentry_shader_instances_count,
        );

        ctx.VSGetConstantBuffers(0, 1, &mut self.constant_buffer);
        ctx.IAGetPrimitiveTopology(&mut self.primitive_topology);
        ctx.IAGetIndexBuffer(
            &mut self.index_buffer,
            &mut self.index_buffer_format,
            &mut self.index_buffer_offest,
        );
        ctx.IAGetVertexBuffers(
            0,
            1,
            &mut self.vertex_buffer,
            &mut self.vertex_buffer_strides,
            &mut self.vertex_buffer_offsets,
        );
        ctx.IAGetInputLayout(&mut self.input_layout);
    }

    #[inline]
    pub unsafe fn restore(&mut self, ctx: &ID3D11DeviceContext) {
        ctx.RSSetScissorRects(self.scissor_count, self.scissor_rects.as_ptr());
        ctx.RSSetViewports(self.viewport_count, self.viewports.as_ptr());
        ctx.RSSetState(self.raster_state.take());
        ctx.OMSetBlendState(
            self.blend_state.take(),
            self.blend_factor.as_ptr(),
            self.blend_mask,
        );
        ctx.OMSetDepthStencilState(self.depth_stencil_state.take(), self.stencil_ref);
        ctx.PSSetShaderResources(0, 1, &self.pixel_shader_resource.take());
        ctx.PSSetSamplers(0, 1, &self.sampler.take());
        ctx.PSSetShader(
            self.pixel_shader.take(),
            self.pixel_shader_instances.as_ptr(),
            self.pixel_shader_instances_count,
        );
        self.pixel_shader_instances.release();

        ctx.VSSetShader(
            self.vertex_shader.take(),
            self.vertex_shader_instances.as_ptr(),
            self.vertex_shader_instances_count,
        );
        self.vertex_shader_instances.release();

        ctx.GSSetShader(
            self.geometry_shader.take(),
            self.geometry_shader_instances.as_ptr(),
            self.geomentry_shader_instances_count,
        );
        self.geometry_shader_instances.release();

        ctx.VSSetConstantBuffers(0, 1, &self.constant_buffer);
        ctx.IASetPrimitiveTopology(self.primitive_topology);
        ctx.IASetIndexBuffer(
            self.index_buffer.take(),
            self.index_buffer_format,
            self.index_buffer_offest,
        );
        ctx.IASetVertexBuffers(
            0,
            1,
            &self.vertex_buffer.take(),
            &self.vertex_buffer_strides,
            &self.vertex_buffer_offsets,
        );
        ctx.IASetInputLayout(self.input_layout.take());
    }
}

struct ClassInstances([Option<ID3D11ClassInstance>; 256]);

impl ClassInstances {
    #[inline]
    pub fn as_ptr(&self) -> *const Option<ID3D11ClassInstance> {
        &self.0[0]
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut Option<ID3D11ClassInstance> {
        &mut self.0[0]
    }

    #[inline]
    pub fn release(&mut self) {
        self.0.iter().for_each(drop);
    }
}

impl Default for ClassInstances {
    fn default() -> Self {
        unsafe { zeroed() }
    }
}
