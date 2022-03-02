[[block]]
struct ViewUniform {
    scale: vec2<f32>;
    translate: vec2<f32>;
};

[[group(0), binding(0)]]
var<uniform> view: ViewUniform;

[[block]]
struct Uniform {
    color: vec4<f32>;
    shape_scale: vec2<f32>;
};

[[group(1), binding(0)]]
var<uniform> uni: Uniform;

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn main(
    [[location(0)]] vertex_pos: vec2<f32>;
    [[location(1)]] instance_pos: vec2<f32>;
) -> VertexOutput {
    var out: VertexOutput;
    let pos = instance_pos + vertex_pos * uni.shape_scale;
    out.clip_position = vec4<f32>(pos * view.scale + view.translate, 1.0, 1.0);
    out.color = uni.color;
    return out;
}