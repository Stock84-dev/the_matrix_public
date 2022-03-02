[[group(2), binding(0)]]
var t_diffuse: texture_2d<f32>;
// [[group(2), binding(1)]]
// var scale: vec2<f32>;

[[stage(fragment)]]
fn main(
    // NOTE: this position is in pixel space
    [[builtin(position)]] clip_position: vec4<f32>,
) -> [[location(0)]] vec4<f32> {
    // let dims = textureDimensions(t_diffuse);
    // let size = vec2<f32>(823., 989. / 2.);
    // let scale = vec2<f32>(823., 989.) / size;
    // let pos = vec2<f32>(clip_position.x , clip_position.y - 989. / 4.);
    // var texel_pos: vec2<f32> = pos * scale;
    let texel_pos = vec2<f32>(clip_position.x, clip_position.y);
    // texel_pos.y = texel_pos.y + 0.5;

    let texel = textureLoad(t_diffuse, vec2<i32>(texel_pos), 0);
    return texel;
}