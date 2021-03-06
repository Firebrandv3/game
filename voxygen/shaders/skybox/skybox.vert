#version 330 core

in vec3 vert_pos;

layout (std140)
uniform global_consts {
	mat4 view_mat;
	mat4 proj_mat;
	vec4 cam_origin;
	vec4 play_origin;
	vec4 view_distance;
	vec4 time;
};

out vec3 frag_pos;

void main() {
	frag_pos = vert_pos;

	gl_Position = proj_mat * view_mat * vec4(3000 * vert_pos + cam_origin.xyz, 1);
}
