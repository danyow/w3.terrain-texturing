// view
struct View {
    view_proj: mat4x4<f32>;
    inverse_view: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
    near: f32;
    far: f32;
    width: f32;
    height: f32;
};

// lights
struct DirectionalLight {
    color: vec3<f32>;
    brightness: f32;
    direction: vec3<f32>;
};

// mesh
struct Mesh {
    model: mat4x4<f32>;
    inverse_transpose_model: mat4x4<f32>;
    clipmap_and_lod: u32;
};

// textures
struct TextureParam {
    blend_sharpness: f32;
    slope_base_dampening: f32;
    slope_normal_dampening: f32;
    specularity_scale: f32;
    specularity: f32;
    specularity_base: f32;
    specularity_scale_copy: f32;
    falloff: f32;
};

struct TextureParameters {
    param: array<TextureParam, 31u>;
};

// clipmap
struct ClipmapLayerInfo {
    map_offset: vec2<u32>;
    resolution: f32;
    size: f32;
};

struct ClipmapInfo {
    world_offset: vec2<f32>;
    world_res: f32;
    layers: array<ClipmapLayerInfo, 10u>;
};

// view
[[group(0), binding(0)]] var<uniform> view: View;
[[group(0), binding(1)]] var<uniform> sunlight: DirectionalLight;

[[group(1), binding(0)]] var<uniform> mesh: Mesh;

// textures
[[group(2), binding(0)]] var textureArray: texture_2d_array<f32>;
[[group(2), binding(1)]] var terrainTextureSampler: sampler;
[[group(2), binding(2)]] var normalArray: texture_2d_array<f32>;
[[group(2), binding(3)]] var terrainNormalSampler: sampler;
[[group(2), binding(4)]] var<uniform> textureParams: TextureParameters;

// texturing
[[group(3), binding(0)]] var controlMap: texture_storage_2d_array<r16uint, read>;
[[group(3), binding(1)]] var tintmapArray: texture_2d_array<f32>;
[[group(3), binding(2)]] var tintmapSampler: sampler;
[[group(3), binding(3)]] var<uniform> clipmap: ClipmapInfo;

struct TextureMapping {
    diffuse: vec3<f32>;
    normal: vec3<f32>;
};

fn sample_texture(
    textureSlot: u32,
    texturingPos: vec2<f32>,
    scaleValue: f32,
    ddx: vec2<f32>,
    ddy: vec2<f32>,
) -> TextureMapping {
    var out: TextureMapping;

    // scale position and derivatives
    let texturingPos = texturingPos * scaleValue;
    let scaledDDX = ddx * scaleValue;
    let scaledDDY = ddy * scaleValue;

    out.diffuse = textureSampleGrad(
            textureArray, terrainTextureSampler, texturingPos, i32(textureSlot), scaledDDX, scaledDDY).rgb;

    out.normal = textureSampleGrad(
        normalArray, terrainNormalSampler, texturingPos, i32(textureSlot), scaledDDX, scaledDDY).xyz;

    // W3 uses dirextX normals -> invert green channel
    // TODO move to normalmap loading
    out.normal.g = 1.0 - out.normal.g;

    return out;
}

fn triplanar_mapping(
    triplanarWeights: vec3<f32>,
    worldPosition: vec3<f32>,
    worldNormal: vec3<f32>,

    textureSlot: u32,
    scaleValue: f32,

    partialDDX_xy: vec2<f32>,
    partialDDY_xy: vec2<f32>,

    partialDDX_xz: vec2<f32>,
    partialDDY_xz: vec2<f32>,

    partialDDX_yz: vec2<f32>,
    partialDDY_yz: vec2<f32>,

) -> TextureMapping {
    var output: TextureMapping;

    // initialize to zero
    output.diffuse = vec3<f32>(0.0);
    output.normal = vec3<f32>(0.0);

    // from presentation: prefer branching than sampling
    if (triplanarWeights.z > 0.0) {
        var texturingPos = vec2<f32>(worldPosition.x, -worldPosition.y);

        if (worldNormal.z < 0.0) {
            texturingPos.y = -texturingPos.y;
        }

        let a = sample_texture(textureSlot, texturingPos, scaleValue, partialDDX_xy, partialDDY_xy);
        output.diffuse = triplanarWeights.z * a.diffuse;
        output.normal = triplanarWeights.z * a.normal;
    }

    if (triplanarWeights.y > 0.0) {
        let texturingPos = worldPosition.xz;

        // terrain normal never points down
        // if (worldNormal.y < 0.0) {
        //     texturingPos.x = -texturingPos.x;
        // }

        let a = sample_texture(textureSlot, texturingPos, scaleValue, partialDDX_xz, partialDDY_xz);
        output.diffuse = output.diffuse + triplanarWeights.y * a.diffuse;
        output.normal = output.normal + triplanarWeights.y * a.normal;
    }

    if (triplanarWeights.x > 0.0) {
        var texturingPos = vec2<f32>(-worldPosition.y, worldPosition.z);

        if (worldNormal.x < 0.0) {
            texturingPos.x = -texturingPos.x;
        }

        let a = sample_texture(textureSlot, texturingPos, scaleValue, partialDDX_yz, partialDDY_yz);
        output.diffuse = output.diffuse + triplanarWeights.x * a.diffuse;
        output.normal = output.normal + triplanarWeights.x * a.normal;
    }

    return output;
}

struct FragmentInput {
    [[builtin(position)]] frag_coord: vec4<f32>;
    [[location(0)]] world_position: vec4<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {

    let gamma = 2.2;

    // color mesh depending on current lod level
    let lod = mesh.clipmap_and_lod >> 16u;
    let clipmap_level = mesh.clipmap_and_lod & 15u;

    let fragmentPos = in.world_position.xyz;
    let fragmentNormal = normalize(in.normal.xyz);

    // test clipmap
    let mapOffset = vec2<f32>(clipmap.layers[clipmap_level].map_offset);
    let mapScaling: f32 = clipmap.layers[clipmap_level].resolution;
    let mapSize: f32 = clipmap.layers[clipmap_level].size;

    var controlMapPos: vec2<f32> = (fragmentPos.xz - clipmap.world_offset) / clipmap.world_res;
    controlMapPos = (controlMapPos - mapOffset) / mapScaling;

    let controlMapPosCoord: vec2<i32> =  clamp(vec2<i32>(controlMapPos), vec2<i32>(0), vec2<i32>(i32(mapSize)));
    let controlMapValueA: vec4<u32> = textureLoad(controlMap, controlMapPosCoord, i32(clipmap_level));

    // scale texture
    let baseScale = 0.333;

    let partialDDX_xy: vec2<f32> = dpdx(fragmentPos.xy);
    let partialDDY_xy: vec2<f32> = dpdy(fragmentPos.xy);

    let partialDDX_xz: vec2<f32> = dpdx(fragmentPos.xz);
    let partialDDY_xz: vec2<f32> = dpdy(fragmentPos.xz);

    let partialDDX_yz: vec2<f32> = dpdx(fragmentPos.yz);
    let partialDDY_yz: vec2<f32> = dpdy(fragmentPos.yz);

    // test texture
    let overlayTextureA: u32 = (controlMapValueA.x & 31u) - 1u;
    let bkgrndTextureA: u32 = ((controlMapValueA.x >> 5u) & 31u) - 1u;

    // ------------------------------------------------------------------------
    // triplanar mapping of background texture

    // const from presentation
    let tighten = vec3<f32>(0.576);
    // must be > 0.0 to normalize properly
    var triplanarWeights: vec3<f32> = max(vec3<f32>(0.00), abs(fragmentNormal) - tighten);

    // normalize
    triplanarWeights = triplanarWeights / (triplanarWeights.x + triplanarWeights.y + triplanarWeights.z);

    let tri = triplanar_mapping(
        triplanarWeights,
        fragmentPos,
        fragmentNormal,
        bkgrndTextureA,
        baseScale,
        partialDDX_xy, partialDDY_xy,
        partialDDX_xz, partialDDY_xz,
        partialDDX_yz, partialDDY_yz
    );
    // ------------------------------------------------------------------------

    let overlayDiffuse = vec4<f32>(pow(tri.diffuse.rgb, vec3<f32>(gamma)), 1.0);
    let overlayNormalA = tri.normal;


    // TBN matrix for normals
    // Note: because terrain is generated from heightmap a base tangent vector is
    // assumed to be (1, 0, 0) (in the heightmap plane)
    // renorthogonalize tangent with respect to normal
    let fragmentTangent: vec3<f32> = normalize(vec3<f32>(1.0, 0.0, 0.0) - fragmentNormal * dot(vec3<f32>(1.0, 0.0, 0.0), fragmentNormal));
    let biTangent: vec3<f32> = cross(fragmentNormal, fragmentTangent);
    let TBN = mat3x3<f32>(fragmentTangent, biTangent, fragmentNormal);

    // normal vectors range is [-1..1] and mapped in texture to [0..1], so remap:
    var overlayNormal = normalize(overlayNormalA.rgb * 2.0 - 1.0);
    // transform normalmap normal into world space
    overlayNormal = TBN * overlayNormal;

    // --- lighting
    // phong-blinn

    // directional light
    // sun light coming from the sun
    let lightDirection = normalize(-sunlight.direction);

    // pointlight direction
    // let lightDirection = normalize(lightPos - fragmentPos);
    // let viewDirection = normalize(view.world_position.xyz - fragmentPos);
    let viewDirection = normalize(view.world_position.xyz);
    let halfwayDirection = normalize(lightDirection + viewDirection);

    let ambientStrength = 0.003;
    let diffuseStrength = max(dot(overlayNormal, lightDirection), 0.0);
    let specularStrength = 0.5;
    // shininess
    let specularExp = 32.0;
    // let reflectDirection = reflect(-lightDirection, fragmentNormal);
    // let specular = pow(max(dot(viewDirection, reflectDirection), 0.0), specularExp); // phong
    let specular = pow(max(dot(fragmentNormal, halfwayDirection), 0.0), 1.0 * specularExp);

    let ambientCol = sunlight.color * ambientStrength;
    let diffuseCol = diffuseStrength * sunlight.color;
    let specularCol = specularStrength * specular * sunlight.color;

    let col = ambientCol + diffuseCol + specularCol;

    var fragmentCol = vec4<f32>(col * overlayDiffuse.rgb, 1.0);

    // --------------------------------------------------------------------------------------------
    // debug visualization for wireframes and clipmap level

    // https://catlikecoding.com/unity/tutorials/advanced-rendering/flat-and-wireframe-shading/
    let barys = vec3<f32>(in.uv.x, in.uv.y, 1.0 - in.uv.x - in.uv.y);
    let minBarys = min(barys.x, min(barys.y, barys.z));
    // fwidth = abs(dpdx(minBarys)) + abs(dpdy(minBarys));
    let delta = fwidth(minBarys);

    let r = lod % 2u;
    let g = r + lod % 4u;
    let b = r + lod % 3u;
    let wireframeCol = 0.2 * vec4<f32>((1.0 + f32(r)) / 2.0, f32(g) / 3.0, f32(b) / 2.0, 0.0);
    let clipmapCol = vec4<f32>(1.0);

    let wireframeWidth = 0.75 * delta;

    // fragmentCol = mix(wireframeCol, fragmentCol, smoothStep(0.0, wireframeWidth, minBarys));
    // fragmentCol = mix(fragmentCol, clipmapCol, f32(clipmap_level) / 6.0);
    // --------------------------------------------------------------------------------------------

    // --- gamma correction
    fragmentCol = pow(fragmentCol, vec4<f32>(1.0 / gamma));

    return fragmentCol;
}
