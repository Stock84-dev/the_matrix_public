// one param takes 12 bytes, 64 KiB / 12 = 5461 paramaters
[[block]]
struct Uniform {
    // scale by y axis
    scales: array<f32, N_PARAMS>;
    // move property between different axis an then offset by y
    translations: array<vec2<f32>, N_PARAMS>;
    opacities: array<f32, N_PARAMS>;
    color: vec4<f32>;
};

[[group(0), binding(0)]]
var<uniform> uni: Uniform;

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn main(
    [[builtin(vertex_index)]] vid: u32,
    [[location(0)]] value: f32,
) -> VertexOutput {
    var out: VertexOutput;
    let n_params = u32(N_PARAMS);
    let pid = vid % n_params;
    // if (pid == 0u) {
    //     out.clip_position = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    // } elseif (pid == 1u) {
    //     out.clip_position = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    // } else {
    //     out.clip_position = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    //     // out.clip_position = vec4<f32>(f32(pid) / f32(N_PARAMS),f32(pid) / f32(N_PARAMS), 1.0, 1.0);
    //     // out.clip_position = vec4<f32>(f32(pid) / f32(N_PARAMS),f32(pid) / f32(N_PARAMS), 1.0, 1.0);
    // }
    out.clip_position = vec4<f32>(uni.translations[pid].x, value * uni.scales[pid] + uni.translations[pid].y, 1.0, 1.0);
    // out.color = uni.color;
    // out.color = vec4<f32>(1., 0., 0., 1.);
    //out.color = vec4<f32>(1., 0., 0., uni.opacities[pid]);
    out.color = vec4<f32>(1., 0., 0., 0.1);
    return out;
}
