cbuffer GlobalParams: register(b0) {
    float4 gamma_ratios;
    float2 global_viewport_size;
    float grayscale_enhanced_contrast;
    float subpixel_enhanced_contrast;
    uint is_bgr;
    uint3 global_pad;
};

cbuffer BatchParams: register(b1) {
    uint batch_start_index;
    uint3 batch_pad;
};

Texture2D<float4> canvas_texture: register(t0);
SamplerState canvas_sampler: register(s0);

struct Bounds {
    float2 origin;
    float2 size;
};

struct GpuCanvasPrimitive {
    uint order;
    uint shader_index;
    uint input;
    uint pad;
    Bounds bounds;
    Bounds content_mask;
    float opacity;
    float scale_factor;
    uint2 meta_pad;
    float4 uniforms[16];
};

struct GpuCanvasVertexOutput {
    nointerpolation uint canvas_id: TEXCOORD0;
    float4 position: SV_Position;
    float4 clip_distance: SV_ClipDistance;
};

struct GpuCanvasFragmentInput {
    nointerpolation uint canvas_id: TEXCOORD0;
    float4 position: SV_Position;
};

struct TerminalParams {
    float time;
    float page_load;
    float curvature;
    float scanline;
    float2 mouse;
    float2 world_size;
    float4 tint;
    float4 background;
    float flicker;
};

StructuredBuffer<GpuCanvasPrimitive> gpu_canvases: register(t1);

TerminalParams terminal_params(GpuCanvasPrimitive canvas) {
    TerminalParams terminal;
    terminal.time = canvas.uniforms[0].x;
    terminal.page_load = canvas.uniforms[0].y;
    terminal.curvature = canvas.uniforms[0].z;
    terminal.scanline = canvas.uniforms[0].w;
    terminal.mouse = canvas.uniforms[1].xy;
    terminal.world_size = canvas.uniforms[1].zw;
    terminal.tint = canvas.uniforms[2];
    terminal.background = canvas.uniforms[3];
    terminal.flicker = canvas.uniforms[4].x;
    return terminal;
}

float4 to_device_position(float2 unit_vertex, Bounds bounds) {
    float2 position = unit_vertex * bounds.size + bounds.origin;
    float2 device_position = position / global_viewport_size * float2(2.0, -2.0)
        + float2(-1.0, 1.0);
    return float4(device_position, 0.0, 1.0);
}

float4 distance_from_clip_rect(float2 unit_vertex, Bounds bounds, Bounds clip_bounds) {
    float2 position = unit_vertex * bounds.size + bounds.origin;
    float2 top_left = position - clip_bounds.origin;
    float2 bottom_right = clip_bounds.origin + clip_bounds.size - position;
    return float4(top_left.x, bottom_right.x, top_left.y, bottom_right.y);
}

float2 terminal_rotate(float2 p, float angle) {
    float cosine = cos(angle);
    float sine = sin(angle);
    return float2(
        cosine * p.x - sine * p.y,
        sine * p.x + cosine * p.y
    );
}

float terminal_noise(float2 p, float time) {
    return sin(p.x * 10.0) * sin(p.y * (3.0 + sin(time * 0.090909))) + 0.2;
}

float terminal_fbm(float2 p, float time) {
    p *= 1.1;
    float amplitude = 0.5;
    float value = amplitude * terminal_noise(p, time);
    p = terminal_rotate(p, time * 0.02) * 2.0;
    amplitude *= 0.454545;
    value += amplitude * terminal_noise(p, time);
    p = terminal_rotate(p, time * 0.02) * 2.0;
    amplitude *= 0.454545;
    value += amplitude * terminal_noise(p, time);
    return value;
}

float terminal_pattern(float2 p, float time) {
    float2 q = float2(
        terminal_fbm(p + 1.0, time),
        terminal_fbm(terminal_rotate(p, 0.1 * time) + 1.0, time)
    );
    float2 r = float2(
        terminal_fbm(terminal_rotate(q, 0.1), time),
        terminal_fbm(q, time)
    );
    return terminal_fbm(p + r, time);
}

float terminal_digit(float2 p, TerminalParams terminal) {
    const float2 grid = float2(30.0, 15.0);
    float2 cell = floor(p * grid) / grid;
    p *= grid;
    float shader_time = terminal.time * 0.333333;
    float intensity = terminal_pattern(cell * 0.1, shader_time) * 1.3 - 0.03;

    float2 mouse_world = terminal.mouse * terminal.world_size;
    float mouse_distance = distance(cell, mouse_world);
    float mouse_influence = exp(-mouse_distance * 8.0) * 5.0;
    intensity += mouse_influence;
    intensity += sin(mouse_distance * 20.0 - terminal.time * 5.0)
        * 0.1 * mouse_influence;

    float cell_random = frac(sin(dot(cell, float2(12.9898, 78.233))) * 43758.5453);
    float progress = saturate((terminal.page_load - cell_random * 0.8) / 0.2);
    intensity *= smoothstep(0.0, 1.0, progress);

    p = frac(p) * 1.2;
    float2 point5 = float2(p.x * 5.0, (1.0 - p.y) * 5.0);
    float x = frac(point5.x);
    float y = frac(point5.y);
    float i = floor(point5.y) - 2.0;
    float j = floor(point5.x) - 2.0;
    float threshold = (i * i + j * j) * 0.0625;
    float brightness = step(0.1, intensity - threshold)
        * (0.2 + y * 0.8) * (0.75 + x * 0.25);
    return step(0.0, p.x) * step(p.x, 1.0)
        * step(0.0, p.y) * step(p.y, 1.0) * brightness;
}

float terminal_displace(float2 p, TerminalParams terminal) {
    float y = p.y - frac(terminal.time * 0.25);
    float window = 1.0 / (1.0 + 50.0 * y * y);
    float enabled = step(0.8, sin(terminal.time + 4.0 * cos(terminal.time * 2.0)));
    return sin(p.y * 20.0 + terminal.time) * 0.0125 * enabled * terminal.flicker
        * (1.0 + cos(terminal.time * 60.0)) * window;
}

float terminal_signal(float2 p, TerminalParams terminal) {
    float shader_time = terminal.time * 0.333333;
    float bar = (step(frac(p.y + shader_time * 20.0), 0.2) * 0.4 + 1.0)
        * terminal.scanline;
    p.x += terminal_displace(p, terminal);
    float middle = terminal_digit(p, terminal);
    const float offset = 0.002;
    float sum = 0.0;
    [unroll]
    for (int y = -1; y <= 1; y++) {
        [unroll]
        for (int x = -1; x <= 1; x++) {
            sum += terminal_digit(p + float2(x, y) * offset, terminal);
        }
    }
    return middle * 0.9 + sum * 0.1 * bar;
}

float2 terminal_barrel(float2 uv, float curvature) {
    float2 centered = uv * 2.0 - 1.0;
    centered *= 1.0 + curvature * dot(centered, centered);
    return centered * 0.5 + 0.5;
}

float3 terminal_color(float2 uv, TerminalParams terminal) {
    uv = terminal_barrel(uv, terminal.curvature);
    float signal = terminal_signal(uv * terminal.world_size, terminal);
    return lerp(terminal.background.rgb, terminal.tint.rgb, saturate(signal * 0.55));
}

GpuCanvasVertexOutput gpu_canvas_vertex(
    uint vertex_id: SV_VertexID,
    uint instance_id: SV_InstanceID
) {
    float2 unit_vertex = float2(float(vertex_id & 1u), 0.5 * float(vertex_id & 2u));
    uint canvas_id = batch_start_index + instance_id;
    GpuCanvasPrimitive canvas = gpu_canvases[canvas_id];

    GpuCanvasVertexOutput output;
    output.position = to_device_position(unit_vertex, canvas.bounds);
    output.clip_distance = distance_from_clip_rect(
        unit_vertex,
        canvas.bounds,
        canvas.content_mask
    );
    output.canvas_id = canvas_id;
    return output;
}

float4 gpu_canvas_fragment(GpuCanvasFragmentInput input): SV_Target {
    GpuCanvasPrimitive canvas = gpu_canvases[input.canvas_id];
    TerminalParams terminal = terminal_params(canvas);
    float2 local = input.position.xy - canvas.bounds.origin;
    float2 mask_local = input.position.xy - canvas.content_mask.origin;
    float corner_radius = 8.0 * canvas.scale_factor;
    if (mask_local.x < corner_radius && mask_local.y < corner_radius
        && distance(mask_local, float2(corner_radius, corner_radius)) > corner_radius) {
        discard;
    }
    float2 uv = float2(
        local.x / canvas.bounds.size.x,
        1.0 - local.y / canvas.bounds.size.y
    );
    return float4(terminal_color(uv, terminal), canvas.opacity);
}
