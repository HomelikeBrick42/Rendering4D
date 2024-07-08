@group(0)
@binding(0)
var output_texture: texture_storage_2d<rgba8unorm, write>;

struct Camera {
    tan_half_fov: f32,
}

@group(1)
@binding(0)
var<uniform> camera: Camera;

@compute
@workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let size = textureDimensions(output_texture);
    let coords = global_id.xy;

    if coords.x >= size.x || coords.y >= size.y {
        return;
    }

    var aspect = f32(size.x) / f32(size.y);
    var uv = vec2<f32>(coords) / vec2<f32>(size);

    let ray_origin = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    var ray_direction = vec4<f32>(1.0, uv.yx * 2.0 - 1.0, 0.0);
    ray_direction.y *= aspect * camera.tan_half_fov;
    ray_direction.z *= camera.tan_half_fov;
    ray_direction = normalize(ray_direction);

    let color = ray_direction.xyz * 0.5 + 0.5;
    textureStore(output_texture, coords, vec4<f32>(color, 1.0));
}
