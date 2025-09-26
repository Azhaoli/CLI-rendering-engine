use crate::{ Point2D, Vector3D, Color };
use crate::clamp;
use crate::graphicsutils::{ LightSource, LightingMode, Texture, Material };
use crate::mesh::Mesh;

use std::cmp::min;
use std::fmt::Write;

#[derive(Copy, Clone)]
struct Vertex {
	screen_XY: Point2D,
	texture_UV: Point2D,
	normal: Vector3D,
	z_coord: f32
}

impl Vertex {
	fn new(screen_XY: Point2D, texture_UV: Point2D, z_coord: f32, normal: Vector3D) -> Vertex {
		Vertex { screen_XY, texture_UV, z_coord, normal }
	}
	
	// apply barycentric interpolation
	fn interpolate(&self, p2: Vertex, p3: Vertex, a: f32, b: f32, c: f32) -> Vertex {
		let inv_z = a/self.z_coord + b/p2.z_coord + c/p3.z_coord; // apply perspective correction
		Vertex {
			screen_XY: self.screen_XY,
			z_coord: inv_z,
			
			texture_UV: (
				(a*self.texture_UV.0/self.z_coord + b*p2.texture_UV.0/p2.z_coord + c*p3.texture_UV.0/p3.z_coord) / inv_z,
				(a*self.texture_UV.1/self.z_coord + b*p2.texture_UV.1/p2.z_coord + c*p3.texture_UV.1/p3.z_coord) / inv_z
			),
			normal: self.normal.mul(a/self.z_coord).add(p2.normal.mul(b/p2.z_coord)).add(p3.normal.mul(c/p3.z_coord)).div(inv_z)
		}
	}
}

pub struct Viewport {
	width: usize,
	height: usize,
	focal_length: f32,
	pixel_buffer: Vec<Vec<Color>>,
	depth_buffer: Vec<Vec<f32>>,
	pub lights: Vec<LightSource>,
	bg_color: Color
}

impl Viewport {
	pub fn new(width: usize, height: usize, focal_length: f32, bg_color: Color) -> Viewport {
		let (mut pixel_buffer, mut depth_buffer) = (Vec::new(), Vec::new());
		for i in 0..height {
			pixel_buffer.push(vec![bg_color; width]);
			depth_buffer.push(vec![999.0; width]);
		}
		Viewport { width, height, focal_length, pixel_buffer, depth_buffer, bg_color, lights: Vec::new() }
	}
	
	pub fn clear_screen(&mut self) {
		let (mut new_pix, mut new_z) = (Vec::new(), Vec::new());
		for i in 0..self.height {
			new_pix.push(vec![self.bg_color; self.width]);
			new_z.push(vec![999.0; self.width]);
		}
		self.pixel_buffer = new_pix;
		self.depth_buffer = new_z
	}
	
	pub fn display(&self) {
		let mut buf = String::new();
		for h in (0..self.height).step_by(2) {
			for w in 0..self.width {
				let (R_t, G_t, B_t) = self.pixel_buffer[h][w].to_24bit();
				let (R_b, G_b, B_b) = self.pixel_buffer[h+1][w].to_24bit();
				write!(&mut buf, "\x1b[38;2;{R_t};{G_t};{B_t}m\x1b[48;2;{R_b};{G_b};{B_b}m▀\x1b[0m");
			}
			writeln!(&mut buf, "");
		}
		println!("{buf}");
	}
	
	fn project(&self, vector: Vector3D) -> Point2D {
		(
			(vector.X*self.focal_length/vector.Z) + (self.width as f32) * 0.5,
			(vector.Y*self.focal_length/vector.Z) + (self.height as f32) * 0.5
		)
	}
	
	fn draw_line(&mut self, p1: Point2D, p2: Point2D, color: Color) {
		if (p1.0 - p2.0).abs() > (p1.1 - p2.1).abs() {
			let (start, end) = if p1.0 > p2.0 { (p2, p1) }else { (p1, p2) };
			let dx = end.0 - start.0;
			let dy = end.1 - start.1;
			let m = dy/dx;
			
			for i in 0..(dx as usize) + 1 {
				let x = start.0 + (i as f32);
				let y = start.1 + (i as f32)*m;
				if (x > self.width as f32) || (x < 0.0) || (y > self.height as f32) || (y < 0.0) { continue; }
				self.pixel_buffer[y as usize][x as usize] = color;
		}}else {
			let (start, end) = if p1.1 > p2.1 { (p2, p1) }else { (p1, p2) };
			let dx = end.0 - start.0;
			let dy = end.1 - start.1;
			let m = dx/dy;
			
			for i in 0..(dy as usize) + 1 {
				let x = start.0 + (i as f32)*m;
				let y = start.1 + (i as f32);
				if (x > self.width as f32) || (x < 0.0) || (y > self.height as f32) || (y < 0.0) { continue; }
				self.pixel_buffer[y as usize][x as usize] = color;
		}}
	}
	
	fn draw_triangle(&mut self, p1: Vertex, p2: Vertex, p3: Vertex, tex: &Texture, mtl: &Material, norm: Vector3D) {
		// find triangle bounding box
		let (mut x_min, mut x_max) = (999.0, 0.0);
		let (mut y_min, mut y_max) = (999.0, 0.0);
		for corner in [p1.screen_XY, p2.screen_XY, p3.screen_XY] {
			if corner.0 > x_max { x_max = corner.0; }
			if corner.0 < x_min { x_min = corner.0; }
			if corner.1 > y_max { y_max = corner.1; }
			if corner.1 < y_min { y_min = corner.1; }
		}
		x_max = clamp(0.0, self.width as f32-1.0, x_max);
		y_max = clamp(0.0, self.height as f32-1.0, y_max);
		
		// find total triangle area
		let side_1 = (p1.screen_XY.0 - p2.screen_XY.0, p1.screen_XY.1 - p2.screen_XY.1);
		let side_2 = (p1.screen_XY.0 - p3.screen_XY.0, p1.screen_XY.1 - p3.screen_XY.1);
		let mut total_area = side_1.0*side_2.1 - side_1.1*side_2.0; // technically 2*area, but only ratios between areas matter :3

		// check if each point in the bounding box is in the triangle, apply shader if so, otherwise ignore it
		for h in (y_min as usize)..(y_max as usize)+1 {
			for w in (x_min as usize)..(x_max as usize)+1 {
				let dist_p1 = (w as f32 - p1.screen_XY.0, h as f32 - p1.screen_XY.1); // distance vector between (w, h) and p1
				// vertices must be oriented clockwise or all areas will be negative
				let p3_area = dist_p1.0*side_1.1 - dist_p1.1*side_1.0;
				let p2_area = dist_p1.1*side_2.0 - dist_p1.0*side_2.1;
				let p1_area = total_area - (p2_area + p3_area);

				// any area is negative, the point is outside the triangle
				if (p1_area < 0.0) || (p2_area < 0.0) || (p3_area < 0.0) { continue; }
				let (a, b, c) = (p1_area/total_area, p2_area/total_area, p3_area/total_area);
				
				let interp = p1.interpolate(p2, p3, a, b, c);
				if interp.z_coord > self.depth_buffer[h][w] { continue; }
				self.depth_buffer[h][w] = interp.z_coord;

				self.apply_phong_shader(interp, (w, h), tex, mtl, norm);
		}}
	}
	
	// (づ ᴗ _ᴗ)づ .𖥔 ݁ ˖ ✦ ‧₊˚ ⋅
	fn apply_phong_shader(&mut self, fragment: Vertex, pos: (usize, usize), tex: &Texture, mtl: &Material, face_norm: Vector3D) {
		let base_color = tex.sample(fragment.texture_UV);
		let camera_direction = Vector3D::XYZ(0.0, 0.0, 1.0).normalize();
		
		let surface_normal = match mtl.mode {
			LightingMode::Flat => face_norm.normalize(),
			LightingMode::Smooth => fragment.normal.normalize(),
			LightingMode::None => {
				self.pixel_buffer[pos.1][pos.0] = base_color;
				return;
		}};
		
		let ambient = base_color.hadamard(mtl.ambient);
		let mut new_color = Color::RGB(0.0, 0.0, 0.0);
		
		for light in self.lights.iter() {
			let light_direction = light.position.normalize();
			let diffuse_strength = clamp(0.0, 1.0, surface_normal.dot(light_direction));
			let diffuse = mtl.diffuse.mul(diffuse_strength);
		
			let specular_source = light_direction.mul(-1.0).reflect(surface_normal);
			let specular_strength = clamp(0.0, 1.0, camera_direction.dot(specular_source)).powf(mtl.highlights);
			let specular = light.color.mul(specular_strength);
			
			new_color = new_color.add(ambient.mul(0.2).add(diffuse.mul(0.4)).add(specular.mul(0.6)));
		}
		self.pixel_buffer[pos.1][pos.0] = new_color;
	}
	
	pub fn draw_mesh(&mut self, mesh: &Mesh) {
		for tri in 0..mesh.triangles.len() {
			let (tri1, tri2, tri3) = mesh.triangles[tri];
			let (tex1, tex2, tex3) = mesh.tex_tris[tri];
			let (p1, p2, p3) = (mesh.vertices[tri1], mesh.vertices[tri2], mesh.vertices[tri3]);

			self.draw_triangle(
				Vertex::new(self.project(p1), mesh.tex_coords[tex1], p1.Z, mesh.vertex_normals[tri1]),
				Vertex::new(self.project(p2), mesh.tex_coords[tex2], p2.Z, mesh.vertex_normals[tri2]),
				Vertex::new(self.project(p3), mesh.tex_coords[tex3], p3.Z, mesh.vertex_normals[tri3]),
				&mesh.texture,
				&mesh.material,
				mesh.face_normals[tri]
		);}
	}
	
	pub fn draw_wireframe(&mut self, mesh: &Mesh) {
		for tri in 0..mesh.triangles.len() {
			let (tri1, tri2, tri3) = mesh.triangles[tri];
			let (p1, p2, p3) = (
				self.project(mesh.vertices[tri1]),
				self.project(mesh.vertices[tri2]),
				self.project(mesh.vertices[tri3])
			);
			let color = Color::RGB(0.988, 0.667, 0.118);
			
			self.draw_line(p1, p2, color);
			self.draw_line(p2, p3, color);
			self.draw_line(p3, p1, color);
		}
	}
	
	pub fn draw_flat_texture(&mut self, tex: &Texture) {
		for h in 0..min(tex.height, self.height) {
			for w in 0..min(tex.width, self.width) { self.pixel_buffer[h][w] = tex.bitmap[h][w]; }
		}
	}
	
	fn line_intersect_plane(start: Vector3D, end: Vector3D, plane_pos: Vector3D, plane_normal: Vector3D) -> f32 {
		let pos_start = start.dot(plane_normal);
		let pos_end = end.dot(plane_normal);
		let pos_intersect = plane_pos.dot(plane_normal);
		(pos_intersect - pos_start) / (pos_end - pos_start)
	}
	
	
	fn lerp_UV(p1: Point2D, p2: Point2D, fac: f32) -> Point2D {
		(
			p1.0 + (p2.0 - p1.0)*fac,
			p1.1 + (p2.1 - p1.1)*fac
		)
	}

	pub fn clip_against_plane(&self, mesh: &mut Mesh, plane_pos: Vector3D, plane_normal: Vector3D) {
		let normal = plane_normal.normalize();
		let mut tris_to_remove = Vec::new();

		for t in 0..mesh.triangles.len() {
			let mut inside = Vec::new();
			let mut outside = Vec::new();
			let tri = [mesh.triangles[t].0, mesh.triangles[t].1, mesh.triangles[t].2];
			let tex = [mesh.tex_tris[t].0, mesh.tex_tris[t].1, mesh.tex_tris[t].2];
			
			// set the reference point to index 0, swap the other 2 whichever way maintains chirality of the original triangle
			let get_orientation = |pos: usize| { 
				match pos {
					0 => (0, 1, 2),
					1 => (1, 2, 0),
					2 => (2, 0, 1),
					_ => unreachable!()
			}};
			// sort vertex indeces by which side of the plane they're on
			for p in 0..3 {
				if mesh.vertices[tri[p]].dot(normal) >= plane_pos.dot(normal) { inside.push(p); }else { outside.push(p); }
			}
			
			if inside.len() == 0 { tris_to_remove.push(t); continue; } // triangles fully outside the plane are removed
			
			if inside.len() == 3 { continue; } // triangles entirely inside the plane are unaffected
			
			if inside.len() == 1 {
				let (i, o1, o2) = get_orientation(inside[0]);
				let (vi, vo1, vo2) = (mesh.vertices[tri[i]], mesh.vertices[tri[o1]], mesh.vertices[tri[o2]]);
				let (ni, no1, no2) = (mesh.vertex_normals[tri[i]], mesh.vertex_normals[tri[o1]], mesh.vertex_normals[tri[o2]]);
				let (ti, to1, to2) = (mesh.tex_coords[tex[i]], mesh.tex_coords[tex[o1]], mesh.tex_coords[tex[o2]]);
				
				let (fac1, fac2) = (
					Viewport::line_intersect_plane(vo1, vi, plane_pos, normal),
					Viewport::line_intersect_plane(vo2, vi, plane_pos, normal)
				);
				mesh.triangles.push((tri[i], mesh.vertices.len(), mesh.vertices.len()+1));
				mesh.face_normals.push(mesh.face_normals[t]);
				mesh.tex_tris.push((tex[i], mesh.tex_coords.len(), mesh.tex_coords.len()+1));
				
				mesh.vertices.push(vo1.lerp(vi, fac1));
				mesh.vertices.push(vo2.lerp(vi, fac2));
				mesh.vertex_normals.push(no1.lerp(ni, fac1));
				mesh.vertex_normals.push(no2.lerp(ni, fac2));
				mesh.tex_coords.push(Viewport::lerp_UV(to1, ti, fac1));
				mesh.tex_coords.push(Viewport::lerp_UV(to2, ti, fac2));
				
				tris_to_remove.push(t);
			}
			
			if inside.len() == 2 {
				let (o, i1, i2) = get_orientation(outside[0]);
				let (vo, vi1, vi2) = (mesh.vertices[tri[o]], mesh.vertices[tri[i1]], mesh.vertices[tri[i2]]);
				let (no, ni1, ni2) = (mesh.vertex_normals[tri[o]], mesh.vertex_normals[tri[i1]], mesh.vertex_normals[tri[i2]]);
				let (to, ti1, ti2) = (mesh.tex_coords[tex[o]], mesh.tex_coords[tex[i1]], mesh.tex_coords[tex[i2]]);
				
				let (fac1, fac2) = (
					Viewport::line_intersect_plane(vo, vi1, plane_pos, normal),
					Viewport::line_intersect_plane(vo, vi2, plane_pos, normal)
				);
				mesh.triangles.push((tri[i1], tri[i2], mesh.vertices.len()));
				mesh.triangles.push((mesh.vertices.len(), tri[i2], mesh.vertices.len()+1));
				mesh.face_normals.push(mesh.face_normals[t]);
				mesh.face_normals.push(mesh.face_normals[t]);
				mesh.tex_tris.push((tex[i1], tex[i2], mesh.tex_coords.len()));
				mesh.tex_tris.push((mesh.tex_coords.len(), tex[i2], mesh.tex_coords.len()+1));
				
				mesh.vertices.push(vo.lerp(vi1, fac1));
				mesh.vertices.push(vo.lerp(vi2, fac2));
				mesh.vertex_normals.push(no.lerp(ni1, fac1));
				mesh.vertex_normals.push(no.lerp(ni2, fac2));
				mesh.tex_coords.push(Viewport::lerp_UV(to, ti1, fac1));
				mesh.tex_coords.push(Viewport::lerp_UV(to, ti2, fac2));
				
				tris_to_remove.push(t);
		}}
		let (mut new_tris, mut new_face_norms, mut new_tex_tris) = (Vec::new(), Vec::new(), Vec::new());
		for t in 0..mesh.triangles.len() {
			if tris_to_remove.contains(&t) { continue; }
			new_tris.push(mesh.triangles[t]);
			new_face_norms.push(mesh.face_normals[t]);
			new_tex_tris.push(mesh.tex_tris[t]);
		}
		mesh.triangles = new_tris;
		mesh.face_normals = new_face_norms;
		mesh.tex_tris = new_tex_tris;
	}
}
