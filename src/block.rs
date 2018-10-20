use glium::backend::Facade;
use glium::draw_parameters::DrawParameters;
use glium::index::{BufferCreationError as IndexBufferCreationError, PrimitiveType};
use glium::program::ProgramChooserCreationError;
use glium::vertex::BufferCreationError as VertexBufferCreationError;
use glium::{Blend, DrawError, IndexBuffer, Program, Surface, VertexBuffer};

pub struct GLBlockConfig {
    pub alpha: f32,
    pub radius: f32,
    pub sharpness: f32,
    pub screen: [f32; 2],
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub left_offset: f32,
    pub left_color: [f32; 3],
    pub right_offset: f32,
    pub right_color: [f32; 3],
    pub inner_shadow: f32,
    pub shade_color: [f32; 3],
}

pub struct GLBlockProgram {
    program: Program,
}

impl GLBlockProgram {
    pub fn new<F: ?Sized + Facade>(facade: &F) -> Result<Self, ProgramChooserCreationError> {
        let program = program!(facade, 140 => {
        vertex: r#"
            #version 140
            
            in vec2 pos;
            out vec2 vPos;
            out vec2 vMask;

            uniform float uRadius;
            uniform float uSharpness;
            uniform vec2 uScreen;
            uniform vec2 uPosition;
            uniform vec2 uSize;

            void main() {
                vec2 sharpness = vec2(uSharpness);
                vec2 radius = vec2(uRadius);
                vec2 blockSize = uSize + 2.0 * sharpness;
                vec2 size = pos / uScreen * blockSize;
                vec2 offset = (uPosition - sharpness) / uScreen;
                gl_Position = vec4((size + offset) * 2.0 - 1.0, 0.0, 1.0);
                vPos = pos * blockSize - sharpness - radius;
                vMask = uSize - 2.0 * radius;
            }
        "#,
        fragment: r#"
            #version 140

            in vec2 vPos;
            in vec2 vMask;
            out vec4 color;

            uniform float uAlpha;
            uniform float uSharpness;
            uniform float uRadius;
            uniform vec2 uSize;

            uniform float uLeftOffset;
            uniform vec3 uLeftColor;
            uniform float uRightOffset;
            uniform vec3 uRightColor;
            uniform float uInnerShadow;
            uniform vec3 uShadeColor;

            void main() {
                vec2 mask = clamp(vPos, vec2(0.0), vMask);
                float dist = length(vPos - mask);
                float area = 1.0 - clamp((dist - uRadius) / uSharpness, 0.0, 1.0);
                float shade = smoothstep(uInnerShadow, 0.0, dist);
                vec3 c = mix(uLeftColor, uRightColor, smoothstep(uLeftOffset, uRightOffset, vPos.x));
                c = mix(uShadeColor, c, shade);
                color = vec4(c, area * uAlpha);
            }
        "#,
        })?;

        Ok(Self { program })
    }
}

#[derive(Copy, Clone)]
pub struct GLBlockVertex {
    pos: [f32; 2],
}

impl GLBlockVertex {
    fn new(x: f32, y: f32) -> Self {
        GLBlockVertex { pos: [x, y] }
    }
}

implement_vertex!(GLBlockVertex, pos);

pub struct GLBlock {
    vertex_buffer: VertexBuffer<GLBlockVertex>,
    index_buffer: IndexBuffer<u16>,
}

#[derive(Debug)]
pub enum GLBlockError {
    IndexBufferCreationError(IndexBufferCreationError),
    VertexBufferCreationError(VertexBufferCreationError),
}

impl From<IndexBufferCreationError> for GLBlockError {
    fn from(error: IndexBufferCreationError) -> Self {
        GLBlockError::IndexBufferCreationError(error)
    }
}

impl From<VertexBufferCreationError> for GLBlockError {
    fn from(error: VertexBufferCreationError) -> Self {
        GLBlockError::VertexBufferCreationError(error)
    }
}

impl GLBlock {
    pub fn new<F: ?Sized + Facade>(facade: &F) -> Result<Self, GLBlockError> {
        let vertex_buffer = VertexBuffer::immutable(
            facade,
            &[
                GLBlockVertex::new(0.0, 0.0),
                GLBlockVertex::new(0.0, 1.0),
                GLBlockVertex::new(1.0, 1.0),
                GLBlockVertex::new(1.0, 0.0),
            ],
        )?;

        let index_buffer =
            IndexBuffer::immutable(facade, PrimitiveType::TrianglesList, &[0, 1, 2, 0, 2, 3])?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn render<S: ?Sized + Surface>(
        &self,
        surface: &mut S,
        program: &GLBlockProgram,
        config: &GLBlockConfig,
    ) -> Result<(), DrawError> {
        let limit = config.size[0].min(config.size[1]) / 2.0;

        surface.draw(
            &self.vertex_buffer,
            &self.index_buffer,
            &program.program,
            &uniform!{
                uAlpha: config.alpha,
                uRadius: config.radius.min(limit),
                uSharpness: config.sharpness.min(limit),
                uSize: config.size,
                uScreen: config.screen,
                uPosition: config.position,
                uLeftOffset: config.left_offset,
                uLeftColor: config.left_color,
                uRightOffset: config.right_offset,
                uRightColor: config.right_color,
                uInnerShadow: config.inner_shadow,
                uShadeColor: config.shade_color,
            },
            &DrawParameters {
                blend: Blend::alpha_blending(),
                color_mask: (true, true, true, false),
                ..Default::default()
            },
        )?;
        Ok(())
    }
}
