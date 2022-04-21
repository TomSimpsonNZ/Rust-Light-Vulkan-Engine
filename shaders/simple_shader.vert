#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;
layout(location = 2) in vec3 normal;
layout(location = 3) in vec2 uv;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec3 fragPosWorld;
layout(location = 2) out vec3 fragNormalWorld;

layout(set = 0, binding = 0) uniform GlobalUbo {
    mat4 projectionViewMatrix;
    vec4 ambientLightColor;
    vec4 lightPosition;
    vec4 lightColor;
} ubo;

layout(push_constant) uniform Push {
    mat4 modelMatrix; // projection * view * model
    mat4 normalMatrix;
} push;

void main() {
    vec4 positionWorld = push.modelMatrix * vec4(position, 1.0);
    gl_Position = ubo.projectionViewMatrix * positionWorld;

    // temporary: this is only correct sometimes!
    // Only works if uniform skaling is applied
    // vec3 normalWorldSpace = normalize(mat3(push.modelMatrix) * normal);

    // Very expensive
    // mat3 normalMatrix = transpose(inverse(mat3(push.modelMatrix)));
    // vec3 normalWorldSpace = normalize(normalMatrix * normal);

    fragNormalWorld = normalize(mat3(push.normalMatrix) * normal);
    fragPosWorld = positionWorld.xyz;
    fragColor = color;
}