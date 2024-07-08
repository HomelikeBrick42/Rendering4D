@group(0)
@binding(0)
var output_texture: texture_storage_2d<rgba8unorm, write>;

struct Camera {
    position: vec4<f32>,
    tan_half_fov: f32,
    up_sky_color: vec3<f32>,
    down_sky_color: vec3<f32>,
    bounce_count: u32,
    sample_count: u32,
    seed_offset: u32,
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
    position: vec4<f32>,
    normal: vec4<f32>,
}

const min_distance: f32 = 0.001;

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
    if hit.distance < min_distance {
        return hit;
    }

    hit.hit = true;
    hit.color = hyper_sphere.color;
    hit.position = ray.origin + ray.direction * hit.distance;
    hit.normal = (hit.position - hyper_sphere.position) / hyper_sphere.radius;
    return hit;
}

fn sky_color(ray: Ray) -> vec3<f32> {
    return mix(camera.down_sky_color, camera.up_sky_color, ray.direction.y * 0.5 + 0.5);
}

fn random_value(state: ptr<function, u32>) -> f32 {
    *state = *state * 747796405u + 2891336453u;
    var result = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    result = (result >> 22u) ^ result;
    return f32(result) / 4294967295.0;
}

fn random_value_normal_distribution(state: ptr<function, u32>) -> f32 {
    let theta = 2.0 * 3.1415926 * random_value(state);
    let rho = sqrt(-2.0 * log(random_value(state)));
    return rho * cos(theta);
}

fn random_direction(state: ptr<function, u32>) -> vec4<f32> {
    return normalize(vec4<f32>(
        random_value_normal_distribution(state),
        random_value_normal_distribution(state),
        random_value_normal_distribution(state),
        random_value_normal_distribution(state),
    ));
}

fn random_direction_in_hemisphere(state: ptr<function, u32>, normal: vec4<f32>) -> vec4<f32> {
    var direction = random_direction(state);
    if dot(direction, normal) < 0.0 {
        direction *= -1.0;
    }
    return direction;
}

fn get_closest_hit(ray: Ray) -> Hit {
    var closest_hit: Hit;
    closest_hit.hit = false;

    for (var i = 0u; i < hyper_spheres.count; i += 1u) {
        let hit = intersect_hyper_sphere(ray, hyper_spheres.data[i]);
        if hit.hit && (!closest_hit.hit || hit.distance < closest_hit.distance) {
            closest_hit = hit;
        }
    }

    return closest_hit;
}

fn trace(ray_: Ray, state: ptr<function, u32>) -> vec3<f32> {
    var ray = ray_;
    var incoming_light = vec3<f32>(0.0);
    var ray_color = vec3<f32>(1.0);

    for (var i = 0u; i < camera.bounce_count; i += 1u) {
        let hit = get_closest_hit(ray);
        if hit.hit {
            let emissive_color = vec3<f32>(0.0);
            let emission_strength = 0.0;
            let base_color = hit.color;

            ray.origin = hit.position;
            ray.direction = normalize(hit.normal + random_direction(state));

            incoming_light += (emissive_color * emission_strength) * ray_color;
            ray_color *= base_color;
        } else {
            incoming_light += sky_color(ray) * ray_color;
            break;
        }
    }

    return incoming_light;
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

    var state: u32 = u32(coords.x + coords.y * size.x) + camera.seed_offset;

    var color = vec3<f32>(0.0);
    for (var i = 0u; i < camera.sample_count; i += 1u) {
        let uv = (vec2<f32>(coords) + vec2<f32>(random_value(&state), random_value(&state)) * 2.0 - 1.0) / vec2<f32>(size);
        let normalized_uv = vec2<f32>(uv.x, 1.0 - uv.y) * 2.0 - 1.0;

        var ray: Ray;
        ray.origin = camera.position;
        ray.direction = vec4<f32>(1.0, uv.yx * 2.0 - 1.0, 0.0);
        ray.direction.y *= camera.tan_half_fov;
        ray.direction.z *= aspect * camera.tan_half_fov;
        ray.direction = normalize(ray.direction);

        color += trace(ray, &state);
    }
    color /= f32(camera.sample_count);

    textureStore(output_texture, coords, vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0));
}
