@group(0)
@binding(0)
var output_texture: texture_storage_2d<rgba8unorm, write>;

@group(1)
@binding(0)
var main_texture: texture_storage_2d<rgba32float, read_write>;

@compute
@workgroup_size(16, 16)
fn main(
    @builtin(global_invocation_id) global_id: vec3<u32>
) {
    let size = textureDimensions(main_texture);
    let coords = global_id.xy;

    if coords.x >= size.x || coords.y >= size.y {
        return;
    }

    let color = textureLoad(main_texture, coords);
    textureStore(output_texture, coords, color);
}
