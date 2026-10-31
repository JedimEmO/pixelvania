// A soft disc of additive light, drawn into the light or glow buffer.

struct LightParams {
    color: vec4<f32>,
    falloff: f32,
}

@group(2) @binding(0) var<uniform> light: LightParams;

struct FragmentIn {
    @location(2) uv: vec2<f32>,
}

@fragment
fn fragment(in: FragmentIn) -> @location(0) vec4<f32> {
    let d = length(in.uv * 2.0 - 1.0);
    let k = pow(clamp(1.0 - d, 0.0, 1.0), light.falloff);
    return vec4(light.color.rgb * k, 1.0);
}
