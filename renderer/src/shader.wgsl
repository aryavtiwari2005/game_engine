struct CameraUniform {
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct InstanceInput {
    @location(2) model_matrix_0: vec4<f32>,
    @location(3) model_matrix_1: vec4<f32>,
    @location(4) model_matrix_2: vec4<f32>,
    @location(5) model_matrix_3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_position: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var out: VertexOutput;
    out.color = model.color;
    let world_pos_vec4 = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_position = world_pos_vec4.xyz;
    out.clip_position = camera.view_proj * world_pos_vec4;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Generate flat shading normal dynamically using screen derivatives
    let normal = normalize(cross(dpdx(in.world_position), dpdy(in.world_position)));
    
    // Directional light from top-right-front
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.5));
    let diffuse = max(dot(normal, light_dir), 0.0);
    
    let ambient = 0.25;
    let lighting = ambient + diffuse * 0.75;
    
    return vec4<f32>(in.color * lighting, 1.0);
}
