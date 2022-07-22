// adapted instancing example

#import bevy_pbr::mesh_view_bind_group
#import bevy_pbr::mesh_struct

[[group(1), binding(0)]]
var<uniform> mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
    // matrix columns
    [[location(3)]] i_c1: vec4<f32>;
    [[location(4)]] i_c2: vec4<f32>;
    [[location(5)]] i_c3: vec4<f32>;
    [[location(6)]] i_c4: vec4<f32>;
    [[location(7)]] i_color: vec4<f32>;

};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let inst_matrix: mat4x4<f32> = mat4x4<f32>(vertex.i_c1, vertex.i_c2, vertex.i_c3, vertex.i_c4);
    let final = mesh.model * inst_matrix;
    let world_position = final * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.color = vertex.i_color;
    return out;
}

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color;
}
