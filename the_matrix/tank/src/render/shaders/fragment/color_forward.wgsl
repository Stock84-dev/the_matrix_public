struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    //[[location(0), interpolate(flat)]]
    [[location(0)]]
    color: vec4<f32>;
};

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color;
    // return vec4<f32>(1., 1., 1., 1.);
}
