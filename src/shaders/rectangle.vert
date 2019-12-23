#version 450

layout(set = 0, binding = 0) uniform Transform {
    mat4 transform;
};

layout(location = 0) in vec4 left_top;
layout(location = 1) in vec3 right_bottom;
layout(location = 2) in vec4 color;

layout(location = 0) out vec4 fColor;


void main() {
    float left = left_top.x;
    float right = right_bottom.x;
    float top = left_top.y;
    float bottom = right_bottom.y;
    vec2 pos;

    switch (gl_VertexIndex) {
        case 0:
        pos = vec2(right, top);
        break;

        case 1:
        pos = vec2(left, top);
        break;

        case 2:
        pos = vec2(right, bottom);
        break;

        case 3:
        pos = vec2(left, bottom);
        break;

        default:
        break;
    }

    fColor = color;
    gl_Position = transform * vec4(pos, 0.0, 1.0);
}
