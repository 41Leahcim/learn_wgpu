// Vertex shader

// Will store the output of the vertex shader
struct VertexOutput{
    // `@builtin(position)` tells WGPU this contains the vertex's clip coordinates (gl_Position)
    // Those coordinates will be translated to the actual pixel position between return by vertex
    // and receive by fragment shader.
    // You need an extra variable, if you want to keep the original values.
    @builtin(position) clip_position: vec4<f32>,
};

// `@vertex` marks the function as a valid entry point for a vertex shader
@vertex
fn vs_main(
    // @builtin(vertex_index) marks a parameter as vertex_index parameter
    @builtin(vertex_index) in_vertex_index: u32
) -> VertexOutput{
    // Create the Vertex Output, var means that a variable can be modified
    var out: VertexOutput;

    // f32(..) and i32(..) are casts
    // Variables defined with let can't be modified
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

// Fragment shader

// `@location(0)`: store the returned value as first color target
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    // Make the 
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}
