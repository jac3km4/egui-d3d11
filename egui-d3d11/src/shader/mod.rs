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


trait Shader {
    const ENTRY_POINT: PSTR;
    const TARGET: PSTR;

    unsafe fn create(device: &ID3D11Device, blob: &ShaderData) -> Self;
}

#[allow(dead_code)]
enum ShaderData {
    CompiledBlob(ID3DBlob),
    EmbeddedData(&'static [u8]),
}

impl Shader for ID3D11VertexShader {
    const ENTRY_POINT: PSTR = c_str!("vs_main");
    const TARGET: PSTR = c_str!("vs_5_0");

    unsafe fn create(device: &ID3D11Device, blob: &ShaderData) -> Self {
        let (ptr, len) = match blob {
            ShaderData::CompiledBlob(b) => (b.GetBufferPointer(), b.GetBufferSize()),
            ShaderData::EmbeddedData(d) => (d.as_ptr() as _, d.len()),
        };

        expect!(
            device.CreateVertexShader(ptr, len, None),
            "Failed to create vertex shader."
        )
    }
}

impl Shader for ID3D11PixelShader {
    const ENTRY_POINT: PSTR = c_str!("ps_main");
    const TARGET: PSTR = c_str!("ps_5_0");

    unsafe fn create(device: &ID3D11Device, blob: &ShaderData) -> Self {
        let (ptr, len) = match blob {
            ShaderData::CompiledBlob(b) => (b.GetBufferPointer(), b.GetBufferSize()),
            ShaderData::EmbeddedData(d) => (d.as_ptr() as _, d.len()),
        };
        expect!(
            device.CreatePixelShader(ptr, len, None),
            "Failed to create pixel shader."
        )
    }
}

pub struct CompiledShaders {
    pub vertex: ID3D11VertexShader,
    pub pixel: ID3D11PixelShader,
    bytecode: ShaderData,
}

impl CompiledShaders {
    #[inline]
    pub fn get_vertex_bytecode(&self) -> *mut () {
        unsafe {
            match &self.bytecode {
                ShaderData::CompiledBlob(b) => b.GetBufferPointer() as _,
                ShaderData::EmbeddedData(d) => d.as_ptr() as _,
            }
        }
    }

    #[inline]
    pub fn get_vertex_bytecode_len(&self) -> usize {
        unsafe {
            match &self.bytecode {
                ShaderData::CompiledBlob(b) => b.GetBufferSize(),
                ShaderData::EmbeddedData(d) => d.len(),
            }
        }
    }
    
    #[cfg(not(feature = "force-compile"))]
    pub fn new(device: &ID3D11Device) -> Self {
        static VERTEX_DATA: &'static [u8] = include_bytes!("vertex_blob.bin");
    
        let vertex = Self::create_shader::<ID3D11VertexShader>(
            device,
            &ShaderData::EmbeddedData(VERTEX_DATA),
        );
        let pixel = Self::create_shader::<ID3D11PixelShader>(
            device,
            &ShaderData::EmbeddedData(include_bytes!("pixel_blob.bin")),
        );
    
        Self {
            vertex,
            pixel,
            bytecode: ShaderData::EmbeddedData(VERTEX_DATA),
        }
    }
    
    #[cfg(feature = "force-compile")]
    pub fn new(device: &ID3D11Device) -> Self {
        let vblob = Self::compile_shader::<ID3D11VertexShader>();
        let pblob = Self::compile_shader::<ID3D11PixelShader>();

        let vertex = Self::create_shader::<ID3D11VertexShader>(
            device,
            &ShaderData::CompiledBlob(vblob.clone()),
        );
        let pixel = Self::create_shader::<ID3D11PixelShader>(
            device,
            &ShaderData::CompiledBlob(pblob.clone()),
        );

        if cfg!(feature = "save-blob") {
            unsafe {
                std::fs::write(
                    "vertex_blob.bin",
                    std::slice::from_raw_parts(
                        vblob.GetBufferPointer() as *const u8,
                        vblob.GetBufferSize(),
                    ),
                )
                .unwrap();

                std::fs::write(
                    "pixel_blob.bin",
                    std::slice::from_raw_parts(
                        pblob.GetBufferPointer() as *const u8,
                        pblob.GetBufferSize(),
                    ),
                )
                .unwrap();
            }
        }

        Self {
            vertex,
            pixel,
            bytecode: ShaderData::CompiledBlob(vblob),
        }
    }

    #[allow(dead_code)]
    fn compile_shader<S>() -> ID3DBlob
    where
        S: Shader,
    {
        const SHADER_TEXT: &str = include_str!("shader.hlsl");
        
        let mut flags = D3DCOMPILE_ENABLE_STRICTNESS;
        if cfg!(debug_assertions) {
            flags |= D3DCOMPILE_DEBUG;
        }

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
                if cfg!(feature = "no-msgs") {
                    let error = std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                        error.as_ref().unwrap().GetBufferPointer() as *const u8,
                        error.as_ref().unwrap().GetBufferSize(),
                    ));

                    panic!("{}", error);
                } else {
                    unreachable!();
                }
            }

            blob.unwrap()
        }
    }

    #[inline]
    fn create_shader<S>(device: &ID3D11Device, blob: &ShaderData) -> S
    where
        S: Shader,
    {
        unsafe { S::create(device, blob) }
    }
}
