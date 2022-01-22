use std::ptr::null_mut as null;
use windows::Win32::{
    Foundation::PSTR,
    Graphics::{
        Direct3D::{
            Fxc::{D3DCompile, D3DCOMPILE_DEBUG, D3DCOMPILE_ENABLE_STRICTNESS},
            ID3DBlob,
        },
        Direct3D11::{ID3D11Device, ID3D11PixelShader, ID3D11VertexShader},
    },
};

const SHADER_TEXT: &str = include_str!("shader.hlsl");

trait Shader {
    const ENTRY_POINT: PSTR;
    const TARGET: PSTR;

    unsafe fn create(device: &ID3D11Device, blob: &Option<ID3DBlob>) -> Self;
}

impl Shader for ID3D11VertexShader {
    const ENTRY_POINT: PSTR = c_str!("vs_main");
    const TARGET: PSTR = c_str!("vs_5_0");

    unsafe fn create(device: &ID3D11Device, blob: &Option<ID3DBlob>) -> Self {
        expect!(
            device.CreateVertexShader(
                blob.as_ref().unwrap().GetBufferPointer(),
                blob.as_ref().unwrap().GetBufferSize(),
                None,
            ),
            "Failed to create vertex shader."
        )
    }
}

impl Shader for ID3D11PixelShader {
    const ENTRY_POINT: PSTR = c_str!("ps_main");
    const TARGET: PSTR = c_str!("ps_5_0");

    unsafe fn create(device: &ID3D11Device, blob: &Option<ID3DBlob>) -> Self {
        expect!(
            device.CreatePixelShader(
                blob.as_ref().unwrap().GetBufferPointer(),
                blob.as_ref().unwrap().GetBufferSize(),
                None,
            ),
            "Failed to create pixel shader."
        )
    }
}

pub struct ShaderBlobs {
    pub vertex: ID3DBlob,
    pub pixel: ID3DBlob,
}

pub struct CompiledShaders {
    pub vertex: ID3D11VertexShader,
    pub pixel: ID3D11PixelShader,
    pub blobs: ShaderBlobs,
}

impl CompiledShaders {
    pub fn new(device: &ID3D11Device) -> Self {
        let mut flags = D3DCOMPILE_ENABLE_STRICTNESS;
        if cfg!(debug_assertions) {
            flags |= D3DCOMPILE_DEBUG;
        }

        let (vblob, vertex) = Self::compile_shader::<ID3D11VertexShader>(flags, device);
        let (pblob, pixel) = Self::compile_shader::<ID3D11PixelShader>(flags, device);

        Self {
            vertex,
            pixel,
            blobs: ShaderBlobs {
                vertex: vblob,
                pixel: pblob,
            },
        }
    }

    fn compile_shader<S>(flags: u32, device: &ID3D11Device) -> (ID3DBlob, S)
    where
        S: Shader,
    {
        unsafe {
            let mut blob = None;
            let mut error = None;

            if D3DCompile(
                SHADER_TEXT.as_ptr() as _,
                SHADER_TEXT.len() as _,
                PSTR(null()),
                null(),
                None,
                S::ENTRY_POINT,
                S::TARGET,
                flags,
                0,
                &mut blob,
                &mut error,
            )
            .is_err()
            {
                let error = std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                    error.as_ref().unwrap().GetBufferPointer() as *const u8,
                    error.as_ref().unwrap().GetBufferSize(),
                ));

                panic!("{}", error);
            }

            let shader = S::create(device, &blob);
            (blob.unwrap(), shader)
        }
    }
}
