@group(0)
@binding(0)
var output_texture: texture_storage_2d<rgba8unorm, write>;

struct Camera {
    position: vec4<f32>,
    tan_half_fov: f32,
    up_sky_color: vec3<f32>,
    down_sky_color: vec3<f32>,
}

@group(1)
@binding(0)
var<uniform> camera: Camera;

struct HyperSphere {
    position: vec4<f32>,
    color: vec3<f32>,
    radius: f32,
}

struct HyperSpheres {
    count: u32,
    data: array<HyperSphere>,
}

@group(2)
@binding(0)
var<storage, read> hyper_spheres: HyperSpheres;

struct Ray {
    origin: vec4<f32>,
    direction: vec4<f32>,
}

struct Hit {
    hit: bool,
    color: vec3<f32>,
    distance: f32,
}

fn intersect_hyper_sphere(ray: Ray, hyper_sphere: HyperSphere) -> Hit {
    var hit: Hit;
    hit.hit = false;

    let oc = ray.origin - hyper_sphere.position;
    let a = dot(ray.direction, ray.direction);
    let half_b = dot(oc, ray.direction);
    let c = dot(oc, oc) - hyper_sphere.radius * hyper_sphere.radius;
    let discriminant = half_b * half_b - a * c;

    if discriminant < 0.0 {
        return hit;
    }

    let sqrt_discriminant = sqrt(discriminant);
    let t0 = (-half_b - sqrt_discriminant) / a;
    let t1 = (-half_b + sqrt_discriminant) / a;

    hit.distance = t0;
    if hit.distance < 0.0 {
        return hit;
    }

    hit.color = hyper_sphere.color;
    hit.hit = true;
    return hit;
}

fn sky_color(ray: Ray) -> vec3<f32> {
    return mix(camera.down_sky_color, camera.up_sky_color, ray.direction.y * 0.5 + 0.5);
}

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

    var ray: Ray;
    ray.origin = camera.position;
    ray.direction = vec4<f32>(1.0, uv.yx * 2.0 - 1.0, 0.0);
    ray.direction.y *= camera.tan_half_fov;
    ray.direction.z *= aspect * camera.tan_half_fov;
    ray.direction = normalize(ray.direction);

    var closest_hit: Hit;
    closest_hit.hit = false;
    for (var i = 0u; i < hyper_spheres.count; i += 1u) {
        let hit = intersect_hyper_sphere(ray, hyper_spheres.data[i]);
        if hit.hit && (!closest_hit.hit || hit.distance < closest_hit.distance) {
            closest_hit = hit;
        }
    }

    var color = sky_color(ray);
    if closest_hit.hit {
        color = closest_hit.color;
    }

    textureStore(output_texture, coords, vec4<f32>(color, 1.0));
}
