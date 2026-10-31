// A texture added into the light or glow buffer. The texture's alpha is ignored.

@group(2) @binding(0) var image: texture_2d<f32>;
@group(2) @binding(1) var image_sampler: sampler;

struct FragmentIn {
    @location(2) uv: vec2<f32>,
}

@fragment
fn fragment(in: FragmentIn) -> @location(0) vec4<f32> {
    let c = textureSample(image, image_sampler, in.uv);
    return vec4(c.rgb, 1.0);
}
