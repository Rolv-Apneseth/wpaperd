mod coordinates;
mod egl_context;
mod renderer;
mod shader;
mod transition;
mod wallpaper;

use std::ffi::{c_void, CStr};

use color_eyre::Result;
use coordinates::{get_opengl_point_coordinates, Coordinates};
use image::DynamicImage;

pub use egl_context::EglContext;
pub use renderer::Renderer;
pub use transition::Transition;

pub mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));

    pub use Gles2 as Gl;
}

// Macro that check the error code of the last OpenGL call and returns a Result.
#[macro_export]
macro_rules! gl_check {
    ($gl:expr, $desc:expr) => {{
        let error = $gl.GetError();
        if error != gl::NO_ERROR {
            let error_string = $gl.GetString(error);
            return if error_string.is_null() {
                Err(color_eyre::eyre::eyre!("OpenGL error {error}").wrap_err($desc.to_string()))
            } else {
                let error_string = CStr::from_ptr(error_string as _)
                    .to_string_lossy()
                    .into_owned();
                Err(
                    color_eyre::eyre::eyre!("OpenGL error {error}: {error_string}")
                        .wrap_err($desc.to_string()),
                )
            };
        }
    }};
}

fn initialize_objects(gl: &gl::Gl) -> Result<(gl::types::GLuint, gl::types::GLuint)> {
    unsafe {
        let mut vbo = 0;
        gl.GenBuffers(1, &mut vbo);
        gl_check!(gl, "Failed to generate the vbo buffer");
        gl.BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl_check!(gl, "Failed to bind the vbo buffer");
        let vertex_data: Vec<f32> = vec![0.0; 24 as _];
        gl.BufferData(
            gl::ARRAY_BUFFER,
            (vertex_data.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
            vertex_data.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );
        gl_check!(gl, "Failed to send the vertices array");

        let mut eab = 0;
        gl.GenBuffers(1, &mut eab);
        gl_check!(gl, "Failed to generate the eab buffer");
        gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, eab);
        gl_check!(gl, "Failed to bind the eab buffer");
        // We load the elements array buffer once, it's the same for each wallpaper
        const INDICES: [gl::types::GLuint; 6] = [0, 1, 2, 2, 3, 0];
        gl.BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            (INDICES.len() * std::mem::size_of::<gl::types::GLuint>()) as gl::types::GLsizeiptr,
            INDICES.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );
        gl_check!(gl, "Failed to send the indices array");

        const POS_ATTRIB: i32 = 0;
        const TEX_ATTRIB: i32 = 1;
        gl.VertexAttribPointer(
            POS_ATTRIB as gl::types::GLuint,
            2,
            gl::FLOAT,
            0,
            4 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            std::ptr::null(),
        );
        gl_check!(gl, "Failed to set the position attribute for the vertices");
        gl.EnableVertexAttribArray(POS_ATTRIB as gl::types::GLuint);
        gl_check!(
            gl,
            "Failed to enable the position attribute for the vertices"
        );
        gl.VertexAttribPointer(
            TEX_ATTRIB as gl::types::GLuint,
            2,
            gl::FLOAT,
            0,
            4 * std::mem::size_of::<f32>() as gl::types::GLsizei,
            (2 * std::mem::size_of::<f32>()) as *const () as *const _,
        );
        gl_check!(gl, "Failed to set the texture attribute for the vertices");
        gl.EnableVertexAttribArray(TEX_ATTRIB as gl::types::GLuint);
        gl_check!(
            gl,
            "Failed to enable the texture attribute for the vertices"
        );

        // Set the Coordinates of the two triangles. These don't change during
        // the running of the program.
        let vertex_data = get_opengl_point_coordinates(
            Coordinates::default_vec_coordinates(),
            Coordinates::default_texture_coordinates(),
        );

        // Update the vertex buffer
        gl.BufferSubData(
            gl::ARRAY_BUFFER,
            0,
            (vertex_data.len() * std::mem::size_of::<f32>()) as gl::types::GLsizeiptr,
            vertex_data.as_ptr() as *const _,
        );
        gl_check!(gl, "Failed to update the vertices buffer");

        Ok((vbo, eab))
    }
}

fn load_texture(gl: &gl::Gl, image: DynamicImage) -> Result<()> {
    unsafe {
        gl.TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA.try_into().unwrap(),
            image.width().try_into().unwrap(),
            image.height().try_into().unwrap(),
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            image.as_bytes().as_ptr() as *const c_void,
        );
        gl_check!(gl, "Failed to pass the image data to the texture");
        gl.GenerateMipmap(gl::TEXTURE_2D);
        gl_check!(gl, "Failed to generate a mip map for the texture");
        gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl_check!(gl, "Failed to define the texture min filter");
        gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl_check!(gl, "Failed to define the texture mag filter");
    }

    Ok(())
}
