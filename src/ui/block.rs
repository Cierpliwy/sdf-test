use super::{UIContext, UILayout};

use glium::draw_parameters::DrawParameters;
use glium::index::{BufferCreationError as IndexBufferCreationError, PrimitiveType};
use glium::program::ProgramChooserCreationError;
use glium::vertex::BufferCreationError as VertexBufferCreationError;
use glium::{
    implement_vertex, program, uniform, Blend, DrawError, IndexBuffer, Program, Surface,
    VertexBuffer,
};

use std::rc::Rc;

// ======== ERROR =======================================================================

#[derive(Debug)]
pub enum UIBlockError {
    IndexBufferCreationError(IndexBufferCreationError),
    VertexBufferCreationError(VertexBufferCreationError),
    ProgramChooserCreationError(ProgramChooserCreationError),
}

impl From<IndexBufferCreationError> for UIBlockError {
    fn from(error: IndexBufferCreationError) -> Self {
        UIBlockError::IndexBufferCreationError(error)
    }
}

impl From<VertexBufferCreationError> for UIBlockError {
    fn from(error: VertexBufferCreationError) -> Self {
        UIBlockError::VertexBufferCreationError(error)
    }
}

impl From<ProgramChooserCreationError> for UIBlockError {
    fn from(error: ProgramChooserCreationError) -> Self {
        UIBlockError::ProgramChooserCreationError(error)
    }
}

// ======== CONTEXT (read only data for every block ) ===================================

#[derive(Copy, Clone)]
struct UIBlockVertex {
    pos: [f32; 2],
}

impl UIBlockVertex {
    fn new(x: f32, y: f32) -> Self {
        UIBlockVertex { pos: [x, y] }
    }
}

implement_vertex!(UIBlockVertex, pos);

pub struct UIBlockContext {
    ui_context: UIContext,
    program: Program,
    vertex_buffer: VertexBuffer<UIBlockVertex>,
    index_buffer: IndexBuffer<u16>,
}

impl UIBlockContext {
    pub fn new(ui_context: UIContext) -> Result<Self, UIBlockError> {
        let gl_context = &ui_context.gl_context;

        let program = program!(gl_context, 140 => {
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

        let vertex_buffer = VertexBuffer::immutable(
            gl_context,
            &[
                UIBlockVertex::new(0.0, 0.0),
                UIBlockVertex::new(0.0, 1.0),
                UIBlockVertex::new(1.0, 1.0),
                UIBlockVertex::new(1.0, 0.0),
            ],
        )?;

        let index_buffer = IndexBuffer::immutable(
            gl_context,
            PrimitiveType::TrianglesList,
            &[0, 1, 2, 0, 2, 3],
        )?;

        Ok(Self {
            ui_context,
            program,
            vertex_buffer,
            index_buffer,
        })
    }
}

// ======== STYLE =======================================================================

pub struct UIBlockStyle {
    pub alpha: f32,
    pub radius: f32,
    pub sharpness: f32,
    pub left_offset: f32,
    pub left_color: [f32; 3],
    pub right_offset: f32,
    pub right_color: [f32; 3],
    pub inner_shadow: f32,
    pub shade_color: [f32; 3],
}

// ======== BLOCK IMPL ==================================================================

#[derive(Clone)]
pub struct UIBlock {
    context: Rc<UIBlockContext>,
}

impl UIBlock {
    pub fn new(context: Rc<UIBlockContext>) -> Self {
        Self { context }
    }

    pub fn render<S: ?Sized + Surface>(
        &self,
        surface: &mut S,
        style: &UIBlockStyle,
        layout: &UILayout,
    ) -> Result<(), DrawError> {
        let size = layout.get_size();
        let pos = layout.get_pos();
        let screen = layout.get_screen().get().size;
        let limit = size[0].min(size[1]) / 2.0;

        surface.draw(
            &self.context.vertex_buffer,
            &self.context.index_buffer,
            &self.context.program,
            &uniform! {
                uAlpha: style.alpha,
                uRadius: style.radius.min(limit),
                uSharpness: style.sharpness.min(limit),
                uSize: size,
                uScreen: screen,
                uPosition: pos,
                uLeftOffset: style.left_offset,
                uLeftColor: style.left_color,
                uRightOffset: style.right_offset,
                uRightColor: style.right_color,
                uInnerShadow: style.inner_shadow,
                uShadeColor: style.shade_color,
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
