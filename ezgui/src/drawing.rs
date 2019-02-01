use crate::{
    Canvas, Color, Drawable, HorizontalAlignment, Prerender, ScreenPt, Text, VerticalAlignment,
};
use geom::{Bounds, Circle, Distance, Line, Polygon, Pt2D};
use glium::{uniform, Surface};

const TRIANGLES_PER_CIRCLE: usize = 60;

type Uniforms<'a> = glium::uniforms::UniformsStorage<
    'a,
    [f32; 2],
    glium::uniforms::UniformsStorage<'a, [f32; 3], glium::uniforms::EmptyUniforms>,
>;

pub struct GfxCtx<'a> {
    display: &'a glium::Display,
    target: &'a mut glium::Frame,
    program: &'a glium::Program,
    uniforms: Uniforms<'a>,
    params: glium::DrawParameters<'a>,

    // TODO Don't be pub. Delegate everything.
    pub canvas: &'a Canvas,

    pub num_new_uploads: usize,
    pub num_draw_calls: usize,
}

impl<'a> GfxCtx<'a> {
    pub fn new(
        canvas: &'a Canvas,
        display: &'a glium::Display,
        target: &'a mut glium::Frame,
        program: &'a glium::Program,
    ) -> GfxCtx<'a> {
        let params = glium::DrawParameters {
            blend: glium::Blend::alpha_blending(),
            ..Default::default()
        };

        let uniforms = uniform! {
            transform: [canvas.cam_x as f32, canvas.cam_y as f32, canvas.cam_zoom as f32],
            window: [canvas.window_width as f32, canvas.window_height as f32],
        };

        GfxCtx {
            canvas,
            display,
            target,
            program,
            uniforms,
            params,
            num_new_uploads: 0,
            num_draw_calls: 0,
        }
    }

    // Up to the caller to call unfork()!
    // TODO Canvas doesn't understand this change, so things like text drawing that use
    // map_to_screen will just be confusing.
    pub fn fork(&mut self, top_left_map: Pt2D, top_left_screen: ScreenPt, zoom: f64) {
        // map_to_screen of top_left_map should be top_left_screen
        let cam_x = (top_left_map.x() * zoom) - top_left_screen.x;
        let cam_y = (top_left_map.y() * zoom) - top_left_screen.y;

        self.uniforms = uniform! {
            transform: [cam_x as f32, cam_y as f32, zoom as f32],
            window: [self.canvas.window_width as f32, self.canvas.window_height as f32],
        };
    }

    pub fn fork_screenspace(&mut self) {
        self.uniforms = uniform! {
            transform: [0.0, 0.0, 1.0],
            window: [self.canvas.window_width as f32, self.canvas.window_height as f32],
        };
    }

    pub fn unfork(&mut self) {
        self.uniforms = uniform! {
            transform: [self.canvas.cam_x as f32, self.canvas.cam_y as f32, self.canvas.cam_zoom as f32],
            window: [self.canvas.window_width as f32, self.canvas.window_height as f32],
        };
    }

    pub fn clear(&mut self, color: Color) {
        // Without this, SRGB gets enabled and post-processes the color from the fragment shader.
        self.target
            .clear_color_srgb(color.0[0], color.0[1], color.0[2], color.0[3]);
    }

    // Use graphics::Line internally for now, but make it easy to switch to something else by
    // picking this API now.
    pub fn draw_line(&mut self, color: Color, thickness: Distance, line: &Line) {
        self.draw_polygon(color, &line.make_polygons(thickness));
    }

    pub fn draw_rounded_line(&mut self, color: Color, thickness: Distance, line: &Line) {
        self.draw_polygon_batch(vec![
            (color, &line.make_polygons(thickness)),
            (
                color,
                &Circle::new(line.pt1(), thickness / 2.0).to_polygon(TRIANGLES_PER_CIRCLE),
            ),
            (
                color,
                &Circle::new(line.pt2(), thickness / 2.0).to_polygon(TRIANGLES_PER_CIRCLE),
            ),
        ]);
    }

    pub fn draw_arrow(&mut self, color: Color, thickness: Distance, line: &Line) {
        let polygons = line.make_arrow(thickness);
        self.draw_polygon_batch(polygons.iter().map(|poly| (color, poly)).collect());
    }

    pub fn draw_circle(&mut self, color: Color, circle: &Circle) {
        self.draw_polygon(color, &circle.to_polygon(TRIANGLES_PER_CIRCLE));
    }

    pub fn draw_polygon(&mut self, color: Color, poly: &Polygon) {
        let obj = Prerender {
            display: self.display,
        }
        .upload_borrowed(vec![(color, poly)]);
        self.num_new_uploads += 1;
        self.redraw(&obj);
    }

    pub fn draw_polygon_batch(&mut self, list: Vec<(Color, &Polygon)>) {
        let obj = Prerender {
            display: self.display,
        }
        .upload_borrowed(list);
        self.num_new_uploads += 1;
        self.redraw(&obj);
    }

    pub fn redraw(&mut self, obj: &Drawable) {
        self.target
            .draw(
                &obj.vertex_buffer,
                &obj.index_buffer,
                &self.program,
                &self.uniforms,
                &self.params,
            )
            .unwrap();
        self.num_draw_calls += 1;
    }

    // Forwarded canvas stuff.
    pub fn draw_blocking_text(
        &mut self,
        txt: Text,
        (horiz, vert): (HorizontalAlignment, VerticalAlignment),
    ) {
        self.canvas.draw_blocking_text(self, txt, (horiz, vert));
    }
    pub fn get_screen_bounds(&self) -> Bounds {
        self.canvas.get_screen_bounds()
    }
    pub fn draw_text_at(&mut self, txt: Text, map_pt: Pt2D) {
        self.canvas.draw_text_at(self, txt, map_pt);
    }
    pub fn text_dims(&self, txt: &Text) -> (f64, f64) {
        self.canvas.text_dims(txt)
    }
    pub fn draw_text_at_screenspace_topleft(&mut self, txt: Text, pt: ScreenPt) {
        self.canvas.draw_text_at_screenspace_topleft(self, txt, pt);
    }
    pub fn draw_mouse_tooltip(&mut self, txt: Text) {
        self.canvas.draw_mouse_tooltip(self, txt);
    }
    pub fn screen_to_map(&self, pt: ScreenPt) -> Pt2D {
        self.canvas.screen_to_map(pt)
    }
    pub fn get_cursor_in_map_space(&self) -> Option<Pt2D> {
        self.canvas.get_cursor_in_map_space()
    }
}
