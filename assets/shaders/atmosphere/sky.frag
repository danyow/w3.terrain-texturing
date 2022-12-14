// ----------------------------------------------------------------------------
// ported from bevy_atmosphere:
//  https://github.com/JonahPlusPlus/bevy_atmosphere
//  by Jonah Henriksson
// ----------------------------------------------------------------------------
#version 450

precision highp float;

layout(location = 0) in vec3 v_Pos;
layout(location = 0) out vec4 o_Target;

layout(set = 1, binding = 0) uniform AtmosphereMat {
    /// Default: (0.0, 6372e3, 0.0)
    vec3 ray_origin;
    /// Default: (0.0, 1.0, 1.0)
    vec3 sun_position;
    /// Default: 22.0
    float sun_intensity;
    /// Represents Planet radius (Default: 6371e3) and Atmosphere radius (Default: 6471e3)
    vec2 radius;
    /// Represents Rayleigh coefficient (Default: (5.5e-6, 13.0e-6, 22.4e-6)) and scale height (Default: 8e3)
    vec4 rayleigh;
    /// Represents Mie coefficient (Default: 21e-6), scale height (Default: 1.2e3) and preferred scattering direction (Default: 0.758)
    vec3 mie;
};

#define PI 3.14159265359
#define iSteps 16
#define jSteps 8

vec2 sphereIntersetion(vec3 rayStart, vec3 rayDirection, float sphereRadius) {
    // ray-sphere intersection that assumes
    // the sphere is centered at the origin.
    // No intersection when result.x > result.y
    float a = dot(rayDirection, rayDirection);
    float b = 2.0 * dot(rayDirection, rayStart);
    float c = dot(rayStart, rayStart) - (sphereRadius * sphereRadius);
    float d = (b * b) - 4.0 * a * c;
    if (d < 0.0) return vec2(1e5,-1e5);
    return vec2(
        (-b - sqrt(d)) / (2.0 * a),
        (-b + sqrt(d)) / (2.0 * a)
    );
}

vec3 atmosphere(vec3 r, vec3 r0, vec3 pSun, float iSun, float rPlanet, float rAtmos, vec3 kRlh, float kMie, float shRlh, float shMie, float g) {
    // Normalize the sun and view directions.
    pSun = normalize(pSun);
    r = normalize(r);

    // Calculate the step size of the primary ray.
    vec2 p = sphereIntersetion(r0, r, rAtmos);
    if (p.x > p.y) return vec3(0,0,0);
    p.y = min(p.y, sphereIntersetion(r0, r, rPlanet).x);
    float iStepSize = (p.y - p.x) / float(iSteps);

    // Initialize the primary ray time.
    float iTime = 0.0;

    // Initialize accumulators for Rayleigh and Mie scattering.
    vec3 totalRlh = vec3(0);
    vec3 totalMie = vec3(0);

    // Initialize optical depth accumulators for the primary ray.
    float iOdRlh = 0.0;
    float iOdMie = 0.0;

    // Calculate the Rayleigh and Mie phases.
    float mu = dot(r, pSun);
    float mumu = mu * mu;
    float gg = g * g;
    float pRlh = 3.0 / (16.0 * PI) * (1.0 + mumu);
    float pMie = 3.0 / (8.0 * PI) * ((1.0 - gg) * (mumu + 1.0)) / (pow(1.0 + gg - 2.0 * mu * g, 1.5) * (2.0 + gg));

    // Sample the primary ray.
    for (int i = 0; i < iSteps; i++) {

        // Calculate the primary ray sample position.
        vec3 iPos = r0 + r * (iTime + iStepSize * 0.5);

        // Calculate the height of the sample.
        float iHeight = length(iPos) - rPlanet;

        // Calculate the optical depth of the Rayleigh and Mie scattering for this step.
        float odStepRlh = exp(-iHeight / shRlh) * iStepSize;
        float odStepMie = exp(-iHeight / shMie) * iStepSize;

        // Accumulate optical depth.
        iOdRlh += odStepRlh;
        iOdMie += odStepMie;

        // Calculate the step size of the secondary ray.
        float jStepSize = sphereIntersetion(iPos, pSun, rAtmos).y / float(jSteps);

        // Initialize the secondary ray time.
        float jTime = 0.0;

        // Initialize optical depth accumulators for the secondary ray.
        float jOdRlh = 0.0;
        float jOdMie = 0.0;

        // Sample the secondary ray.
        for (int j = 0; j < jSteps; j++) {

            // Calculate the secondary ray sample position.
            vec3 jPos = iPos + pSun * (jTime + jStepSize * 0.5);

            // Calculate the height of the sample.
            float jHeight = length(jPos) - rPlanet;

            // Accumulate the optical depth.
            jOdRlh += exp(-jHeight / shRlh) * jStepSize;
            jOdMie += exp(-jHeight / shMie) * jStepSize;

            // Increment the secondary ray time.
            jTime += jStepSize;
        }

        // Calculate attenuation.
        // vec3 attn = exp(-(kMie * (iOdMie + jOdMie) + kRlh * (iOdRlh + jOdRlh)));
        vec3 attn = exp(-(kMie * 1.3 * (iOdMie + jOdMie) + kRlh * (0.4995 * iOdRlh + jOdRlh)));

        // Accumulate scattering.
        totalRlh += odStepRlh * attn;
        totalMie += odStepMie * attn;

        // Increment the primary ray time.
        iTime += iStepSize;

    }

    // Calculate and return the final color.
    return iSun * (pRlh * kRlh * totalRlh + pMie * kMie * totalMie);
}

void main() {
    vec3 sky = atmosphere(
        normalize(v_Pos),   // normalized ray direction
        ray_origin,         // ray origin
        sun_position,       // position of the sun
        sun_intensity,      // intensity of the sun
        radius.x,           // radius of the planet in meters
        radius.y,           // radius of the atmosphere in meters
        rayleigh.xyz,       // Rayleigh scattering coefficient
        mie.x,              // Mie scattering coefficient
        rayleigh.w,         // Rayleigh scale height
        mie.y,              // Mie scale height
        mie.z               // Mie preferred scattering direction
    );

    sky = 1.0 - exp(-1.0 * sky);
    o_Target = vec4(sky, 1.0);
    // o_Target = vec4(0.5, 1.0, 1.0, 1.0);
}
