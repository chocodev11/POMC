struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct ScreenUniform {
    screen_size: vec2<f32>,
};
@group(0) @binding(0) var<uniform> screen: ScreenUniform;

@group(1) @binding(0) var t_texture: texture_2d<f32>;
@group(1) @binding(1) var t_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(
        2.0 * in.position.x / screen.screen_size.x - 1.0,
        1.0 - 2.0 * in.position.y / screen.screen_size.y,
        0.0,
        1.0,
    );
    out.tex_coords = in.tex_coords;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(t_texture, t_sampler, in.tex_coords);
    return in.color * tex;
}
