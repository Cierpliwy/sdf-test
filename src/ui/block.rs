use crate::ui::widget::{UILayout, UISize, UIWidget};
use glium::backend::Facade;
use glium::draw_parameters::DrawParameters;
use glium::index::PrimitiveType;
use glium::{
    implement_vertex, program, uniform, Blend, Frame, IndexBuffer, Program, Surface, VertexBuffer,
};
use std::rc::Rc;

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
    program: Program,
    vertex_buffer: VertexBuffer<UIBlockVertex>,
    index_buffer: IndexBuffer<u16>,
}

impl UIBlockContext {
    #[allow(clippy::redundant_closure)]
    pub fn new<F: ?Sized + Facade>(facade: &F) -> Self {
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
        }).expect("Cannot create program for UIBlock");

        let vertex_buffer = VertexBuffer::immutable(
            facade,
            &[
                UIBlockVertex::new(0.0, 0.0),
                UIBlockVertex::new(0.0, 1.0),
                UIBlockVertex::new(1.0, 1.0),
                UIBlockVertex::new(1.0, 0.0),
            ],
        )
        .expect("Cannot create vertex buffer for UIBlock");

        let index_buffer =
            IndexBuffer::immutable(facade, PrimitiveType::TrianglesList, &[0, 1, 2, 0, 2, 3])
                .expect("Cannot create index buffer for UIBlock");

        Self {
            program,
            vertex_buffer,
            index_buffer,
        }
    }
}

#[derive(Copy, Clone)]
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

#[derive(Clone)]
pub struct UIBlock {
    context: Rc<UIBlockContext>,
    style: UIBlockStyle,
}

impl UIBlock {
    pub fn new(context: Rc<UIBlockContext>, style: UIBlockStyle) -> Self {
        Self { context, style }
    }

    pub fn set_style(&mut self, style: UIBlockStyle) {
        self.style = style;
    }

    pub fn get_style(&self) -> UIBlockStyle {
        self.style
    }

    pub fn render_styled(
        &self,
        frame: &mut Frame,
        layout: UILayout,
        style: UIBlockStyle,
        screen: UISize,
    ) {
        let screen = [screen.width, screen.height];
        let limit = layout.width.min(layout.height) / 2.0;

        frame
            .draw(
                &self.context.vertex_buffer,
                &self.context.index_buffer,
                &self.context.program,
                &uniform! {
                    uAlpha: style.alpha,
                    uRadius: style.radius.min(limit),
                    uSharpness: style.sharpness.min(limit),
                    uSize: [layout.width, layout.height],
                    uScreen: screen,
                    uPosition: [layout.left, layout.top],
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
            )
            .expect("Cannot draw UIBlock");
    }
}

impl UIWidget for UIBlock {
    type Event = ();

    fn render(&self, frame: &mut Frame, layout: UILayout, screen: UISize) {
        self.render_styled(frame, layout, self.style, screen);
    }
}
