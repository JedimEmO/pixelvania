// Combines the colour, light and glow buffers and scales them to the window.
//
// Everything is read with textureLoad, one texel per game pixel, so nothing is filtered.
// The pixel position alone decides which game pixel a window pixel shows.

struct CompositeParams {
    origin: vec2<f32>,
    scale: f32,
    bands: f32,
    offset: vec2<f32>,
    rim: f32,
    light_range: f32,
    background: vec4<f32>,
}

@group(2) @binding(0) var<uniform> params: CompositeParams;
@group(2) @binding(1) var color_tex: texture_2d<f32>;
@group(2) @binding(2) var light_tex: texture_2d<f32>;
@group(2) @binding(3) var glow_tex: texture_2d<f32>;
@group(2) @binding(4) var background_tex: texture_2d<f32>;
@group(2) @binding(5) var foreground_tex: texture_2d<f32>;

const SIZE: vec2<f32> = vec2(640.0, 360.0);

struct FragmentIn {
    @builtin(position) position: vec4<f32>,
}

fn clamp_texel(p: vec2<i32>) -> vec2<i32> {
    let size = vec2<i32>(textureDimensions(color_tex));
    return clamp(p, vec2(0), size - vec2(1));
}

// Steps light into flat bands by its brightest channel, keeping its hue.
fn band(l: vec3<f32>) -> vec3<f32> {
    let m = max(max(l.r, l.g), l.b);
    if m < 0.0001 {
        return l;
    }
    let q = max(floor(m * params.bands + 0.5), 1.0) / params.bands;
    return l * (q / m);
}

fn light_at(p: vec2<i32>) -> vec3<f32> {
    return band(textureLoad(light_tex, clamp_texel(p), 0).rgb * params.light_range);
}

fn solid_at(p: vec2<i32>) -> bool {
    return textureLoad(color_tex, clamp_texel(p), 0).a > 0.5;
}

@fragment
fn fragment(in: FragmentIn) -> @location(0) vec4<f32> {
    let local = (in.position.xy - params.origin) / params.scale;
    if local.x < 0.0 || local.y < 0.0 || local.x >= SIZE.x || local.y >= SIZE.y {
        return vec4(0.0, 0.0, 0.0, 1.0);
    }
    let p = vec2<i32>(floor(local + vec2(1.0) + params.offset));

    let albedo = textureLoad(color_tex, clamp_texel(p), 0);
    let light = light_at(p);
    var out: vec3<f32>;
    if albedo.a > 0.5 {
        out = albedo.rgb * light;
        // Rim light: a solid pixel next to empty space picks up the light in that space.
        var rim = vec3(0.0);
        let dirs = array(vec2(1, 0), vec2(-1, 0), vec2(0, 1), vec2(0, -1));
        for (var i = 0; i < 4; i++) {
            let n = p + dirs[i];
            if !solid_at(n) {
                rim = max(rim, light_at(n));
            }
        }
        out += rim * params.rim * albedo.rgb * 4.0;
    } else {
        // Scenery carries its own distant lighting, and the room's lights lift it a little more:
        // half bright in the dark, full near a lamp, so detail shows where the light is.
        // Its alpha never enters solid_at. Render-target RGB is premultiplied by coverage;
        // empty rooms retain the old fill.
        let background = textureLoad(background_tex, clamp_texel(p), 0);
        let lift = clamp(vec3(0.45) + light * 0.6, vec3(0.45), vec3(1.2));
        out = background.rgb * lift + params.background.rgb * light * (1.0 - background.a);
    }
    out += textureLoad(glow_tex, clamp_texel(p), 0).rgb;
    let foreground = textureLoad(foreground_tex, clamp_texel(p), 0);
    out = foreground.rgb + out * (1.0 - foreground.a);
    return vec4(out, 1.0);
}
