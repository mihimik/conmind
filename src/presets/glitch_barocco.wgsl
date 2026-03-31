struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index << 1u) & 2i) * 2.0 - 1.0;
    let y = f32(i32(in_vertex_index) & 2i) * 2.0 - 1.0;

    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 0.5 - y * 0.5);
    return out;
}

struct Audio {
    time: f32,
    bass: f32,
    mid: f32,
    high: f32,
    volume: f32,
};

@group(0) @binding(0)
var<uniform> audio: Audio;

@group(0) @binding(1)
var<storage, read> spectrum: array<f32>;

struct WindowSize {
    width: f32,
    height: f32,
};

@group(0) @binding(2)
var<uniform> window_size: WindowSize;


fn get_fractal_layer(uv: vec2<f32>, t: f32) -> vec2<f32> {
    var p = uv;
    let offset = 0.7;
    for (var i = 0; i < 4; i++) {
        p = abs(p) / (dot(p, p) + 0.1) - offset;
        let a = t * 0.1 + f32(i) * 0.2;
        p = vec2<f32>(p.x * cos(a) - p.y * sin(a), p.x * sin(a) + p.y * cos(a));
    }
    return p;
}

fn get_ascii_mask(luma: f32, char_uv: vec2<f32>) -> f32 {
    var mask = 0.0;
    if (luma > 0.6) {
        mask = 1.0;
    } else if (luma > 0.3) {
        mask = step(0.35, abs(char_uv.x - 0.5)) + step(0.35, abs(char_uv.y - 0.5));
    } else if (luma > 0.1) {
        mask = step(length(char_uv - 0.5), 0.15);
    }
    return mask;
}

fn get_lasers(uv_in: vec2<f32>, t: f32, high: f32) -> vec3<f32> {
    var final_lasers = vec3<f32>(0.0);
    let max_layers = 6;

    for (var i = 0; i < max_layers; i++) {
        let fi = f32(i);
        let layer_trig = smoothstep(fi / f32(max_layers), (fi + 1.0) / f32(max_layers), high);
        if (layer_trig <= 0.0) { continue; }

        let angle = fi * 1.57 + t * (0.1 + fi * 0.05);
        let uv_rot = vec2<f32>(
            uv_in.x * cos(angle) - uv_in.y * sin(angle),
            uv_in.x * sin(angle) + uv_in.y * cos(angle)
        );

        let pos = fract(t * (0.3 + fi * 0.1) + fi * 0.23);

        let dist = abs(uv_rot.y - pos);
        let line = step(dist, 0.003) - step(dist, 0.0015);

        let noise = fract(sin(dot(uv_rot * (t + fi), vec2<f32>(12.9898, 78.233))) * 43758.5453);
        let glitch = step(noise, 0.05 * layer_trig);

        var l_color = vec3<f32>(0.1, 0.4, 1.0);
        if (i % 2 == 1) {
            l_color = vec3<f32>(0.4, 0.1, 0.8);
        }

        final_lasers += l_color * (line + glitch) * 0.4 * layer_trig;
    }
    return final_lasers;
}

fn get_scene(uv_screen: vec2<f32>, audio: Audio, window_size: WindowSize) -> vec3<f32> {
    let aspect = window_size.width / window_size.height;
    var uv_raw = vec2<f32>((uv_screen.x - 0.5) * aspect, uv_screen.y - 0.5);

    let g_trig = smoothstep(0.6, 0.9, audio.high);
    if (g_trig > 0.0) {
        uv_raw.x += sin(floor(uv_raw.y * 40.0) * 10.0 + audio.time * 30.0) * 0.05 * g_trig;
    }

    let t = audio.time * 0.5;
    let p1 = get_fractal_layer(uv_raw * exp2(-fract(t) * 5.0), audio.time);
    let p2 = get_fractal_layer(uv_raw * exp2(-fract(t + 0.5) * 5.0), audio.time);
    let p = mix(p1, p2, cos(fract(t) * 6.28318) * 0.5 + 0.5);

    let r = length(p);
    let spiral = atan2(p.y, p.x) + sin(r * 3.0 - audio.time) * audio.bass;
    let flower = cos(spiral * 5.0 + audio.time) * (0.1 + audio.bass * 0.1);

    let glow = (0.01 + audio.mid * 0.03) / (abs(r - 0.3 - flower) + 0.002);

    var color = vec3<f32>(1.0, 0.1, 0.05) * glow;
    color = mix(color, vec3<f32>(1.0, 0.4, 0.1) * glow * 1.5, pow(audio.bass, 3.0));

    let char_uv = fract(uv_screen * vec2<f32>(window_size.width / 10.0, window_size.height / 16.0));
    let mask = get_ascii_mask(dot(color, vec3<f32>(0.299, 0.587, 0.114)), char_uv);
    let vignette = smoothstep(1.3, 0.3, length(uv_raw));

    return color * mask * vignette;
}




@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pulse = pow(smoothstep(0.75, 0.95, audio.bass), 4.0);
    let shift = pulse * 0.04;

    let r_c = get_scene(in.uv + vec2<f32>(shift, 0.0), audio, window_size).r;
    let g_c = get_scene(in.uv, audio, window_size).g;
    let b_c = get_scene(in.uv - vec2<f32>(shift, 0.0), audio, window_size).b;

    var final_output = vec3<f32>(r_c, g_c, b_c);
    let lasers = get_lasers(in.uv, audio.time, audio.high);

    final_output += lasers;

    final_output += final_output * pulse * 1.5;

    let scan_noise = fract(sin(in.uv.y * 1234.56 + audio.time) * 43758.5453);

    if (pulse > 0.4 && scan_noise > 0.7) {
        let tear = sin(in.uv.y * 50.0 + audio.time * 10.0) * 0.1 * pulse;
        final_output = get_scene(in.uv + vec2<f32>(tear, 0.0), audio, window_size) + lasers;
    }

    final_output = clamp(final_output, vec3<f32>(0.0), vec3<f32>(1.2));

    return vec4<f32>(final_output, 1.0);
}
