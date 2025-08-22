use crate::{ Color, Vector3D, Point2D };
use crate::mesh::{ Mesh, Texture, Material, LightingMode };
use crate::clamp;

use std::io::Write;
use std::io;
use std::cmp::{ max, min };
use std::f32::consts::PI;

#[derive(Copy, Clone)]
pub struct LightSource {
	color: Color,
	position: Vector3D
}

impl LightSource {
	pub fn new(color: Color, position: Vector3D) -> LightSource { LightSource{ color, position } }
	pub fn magenta(position: Vector3D) -> LightSource { LightSource{ color: Color::RGB(1.0, 0.0, 1.0), position } }
}


#[derive(Copy, Clone)]
pub struct VertexData {
	pub UV: Point2D,
	pub screenXY: Point2D,
	pub worldXYZ: Vector3D,
	pub normal: Vector3D,
	pub inv_depth: f32 // used for texture coordinate perspective correction
}

impl VertexData {
	fn new(UV: Point2D, screenXY: Point2D, worldXYZ: Vector3D, normal: Vector3D, inv_depth: f32) -> VertexData { VertexData{ UV, screenXY, worldXYZ, normal, inv_depth } }
	
	fn unset() -> VertexData {
		VertexData {
			UV: [0.0; 2],
			screenXY: [0.0; 2],
			worldXYZ: Vector3D::zero(),
			normal: Vector3D::zero(),
			inv_depth: 1000.0
		}
	}
	
	fn lerp(&self, other: VertexData, x: f32, y: f32, fac: f32) -> VertexData {
		VertexData {
			UV: [
				self.UV[0] + (other.UV[0] - self.UV[0])*fac,
				self.UV[1] + (other.UV[1] - self.UV[1])*fac
			],
			screenXY: [x, y],
			// interpolated normals don't have unit length, each fragment is normalized when the shader is run
			normal: self.normal.add(other.normal.sub(self.normal).mul(fac)),
			worldXYZ: self.worldXYZ.add(other.worldXYZ.sub(self.worldXYZ).mul(fac)),
			inv_depth: self.inv_depth + (other.inv_depth - self.inv_depth)*fac
		}
	}
}


pub struct Viewport {
	pub pixel_buffer: Vec<Vec<Color>>,
	pub data_buffer: Vec<Vec<VertexData>>,
	pub lights: Vec<LightSource>,
	pub width: usize,
	pub height: usize,
	pub focal_length: f32,
	pub bg: Color
}

impl Viewport {
	pub fn new(width: usize, height: usize, focal_length: f32, bg: Color) -> Viewport {
		let (mut pix_row, mut pix_buf) = (Vec::new(), Vec::new());
		let (mut data_row, mut data_buf) = (Vec::new(), Vec::new());
		for w in 0..width {
			pix_row.push(bg);
			data_row.push(VertexData::unset());
		}
		for h in 0..height {
			pix_buf.push(pix_row.clone());
			data_buf.push(data_row.clone());
		}
		Viewport {
			pixel_buffer: pix_buf,
			data_buffer: data_buf,
			lights: Vec::new(),
			width,
			height,
			focal_length,
			bg
		}
	}

	pub fn render_to_image(&mut self) -> Texture {
		let mut render = Vec::new();
		let mut pixel_row = Vec::new();
		for h in 0..self.height {
			for w in 0..self.width {
				pixel_row.push(self.pixel_buffer[h][w]);
			}
			render.push(pixel_row.clone());
			pixel_row.clear();
		}
		Texture::new(self.width, self.height, render)
	}
	
	pub fn clear_buffer(&mut self) {
		for h in 0..self.height {
			for w in 0..self.width {
				self.pixel_buffer[h][w] = self.bg;
				self.data_buffer[h][w] = VertexData::unset();
		}}
	}
	
	pub fn project(&self, vec: Vector3D) -> Point2D {
		[
			(vec.X*self.focal_length / vec.Z) + (self.width as f32*0.5),
			(vec.Y*self.focal_length / vec.Z) + (self.height as f32*0.5)
		]
	} 
	
	fn draw_line(&self, p1: VertexData, p2: VertexData) -> Vec<VertexData> {
		let mut points = Vec::new();
		if (p1.screenXY[0] - p2.screenXY[0]).abs() > (p1.screenXY[1] - p2.screenXY[1]).abs() {
			let (start, end) = if p1.screenXY[0] < p2.screenXY[0] { (p1, p2) }else { (p2, p1) };
			let dx = end.screenXY[0] - start.screenXY[0];
			let dy = end.screenXY[1] - start.screenXY[1];
			
			let m = dy/dx;
			for i in 0..(dx as usize)+1 {
				let x = start.screenXY[0] + (i as f32);
				let y = start.screenXY[1] + (i as f32)*m;
				points.push(start.lerp(end, x, y, (i as f32) / dx));
		
		}}else {
			let (start, end) = if p1.screenXY[1] < p2.screenXY[1] { (p1, p2) }else { (p2, p1) };
			let dx = end.screenXY[0] - start.screenXY[0];
			let dy = end.screenXY[1] - start.screenXY[1];
			
			let m = dx/dy;
			for i in 0..(dy as usize)+1 {
				let x = start.screenXY[0] + (i as f32)*m;
				let y = start.screenXY[1] + (i as f32);
				points.push(start.lerp(end, x, y, (i as f32) / dy));
		}}
		
		points
	}
	
	fn draw_triangle(&mut self, p: [VertexData; 3], tex: &Texture, mtl: &Material, face_norm: Vector3D) {
		let mut points = Vec::new();
		points.extend(self.draw_line(p[0], p[1]));
		points.extend(self.draw_line(p[1], p[2]));
		points.extend(self.draw_line(p[2], p[0]));
		
		let (mut y_min, mut y_max) = (1000, 0);
		for corner in p.into_iter() {
			y_min = min(y_min, corner.screenXY[1] as usize);
			y_max = max(y_max, corner.screenXY[1] as usize);
		}
		
		let mut x_bounds = Vec::new();
		for x in y_min..y_max+1 {
			x_bounds.push((
				VertexData::new([0.0; 2], [1000.0, 0.0], Vector3D::zero(), Vector3D::zero(), 0.0),
				VertexData::new([0.0; 2], [-1000.0, 0.0], Vector3D::zero(), Vector3D::zero(), 0.0)
		));}
		
		for p in points.iter() {
			let y = p.screenXY[1] as usize;
			if p.screenXY[0] < x_bounds[y-y_min].0.screenXY[0] { x_bounds[y-y_min].0 = *p; }
			if p.screenXY[0] > x_bounds[y-y_min].1.screenXY[0] { x_bounds[y-y_min].1 = *p; }
		}
		
		for y in y_min..y_max+1 {
			if (y < 0) || (y >= self.height) { continue; }
		
			let (x_min, x_max) = (x_bounds[y-y_min].0.screenXY[0], x_bounds[y-y_min].1.screenXY[0]);
			for x in x_min as usize..x_max as usize {
				if (x < 0) || (x >= self.width) { continue; }
				
				let interp = x_bounds[y-y_min].0.lerp(x_bounds[y-y_min].1, x as f32, y as f32, (x as f32 - x_min)/(x_max-x_min));
				
				if interp.inv_depth > self.data_buffer[y][x].inv_depth { continue; }
				self.data_buffer[y][x] = interp;
				self.apply_phong_shader(interp, tex, mtl, face_norm);
		}}
		
	}
	
	// basic phong lighting
	fn apply_phong_shader(&mut self, fragment: VertexData, tex: &Texture, mtl: &Material, face_norm: Vector3D) {
		let base_color = tex.sample([fragment.UV[0]/fragment.inv_depth, fragment.UV[1]/fragment.inv_depth]);
		let camera_direction = Vector3D::XYZ(0.0, 0.0, 1.0).normalize();
		
		let surface_normal = match mtl.mode {
			LightingMode::Flat => face_norm.normalize(),
			LightingMode::Smooth => fragment.normal.normalize(),
			LightingMode::None => {
				self.pixel_buffer[fragment.screenXY[1] as usize][fragment.screenXY[0] as usize] = base_color;
				return;
		}};
		
		let ambient = base_color.mul_elements(mtl.ambient);
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

		//self.pixel_buffer[fragment.screenXY[1] as usize][fragment.screenXY[0] as usize] = new_color;
		
		let smooth_clamp = |val: f32| (2.0/PI)*(val/5.0).atan();
		let pos = fragment.worldXYZ.div(fragment.inv_depth);
		self.pixel_buffer[fragment.screenXY[1] as usize][fragment.screenXY[0] as usize] = Color::RGB(smooth_clamp(pos.X), smooth_clamp(pos.Y), smooth_clamp(pos.Z));
		
		/*
		let d = (2.0/PI)*(fragment.inv_depth.abs()*4.0).atan();
		self.pixel_buffer[fragment.screenXY[1] as usize][fragment.screenXY[0] as usize] = Color::RGB(d, d, d);
		*/
		//self.pixel_buffer[fragment.screenXY[1] as usize][fragment.screenXY[0] as usize] = Color::RGB(surface_normal.X.abs(), surface_normal.Y.abs(), surface_normal.Z.abs());
	}


	fn line_intersect_plane(&self, start: Vector3D, end: Vector3D, plane_pos: Vector3D, plane_normal: Vector3D) -> f32 {
		let pos_start = start.dot(plane_normal);
		let pos_end = end.dot(plane_normal);
		let pos_intersect = plane_pos.dot(plane_normal);
		(pos_intersect - pos_start) / (pos_end - pos_start)
	}

	// There has to be a better way to do this somehow TwT
	pub fn clip_against_plane(&self, mesh: &mut Mesh, plane_pos: Vector3D, plane_normal: Vector3D) {
		let normal = plane_normal.normalize();
		let mut tris_to_remove = Vec::new();
		let mut new_verts = Vec::new();

		for t in 0..mesh.triangles.len() {
			let mut inside = Vec::new();
			let mut outside = Vec::new();
			// sort vertex indeces by which side of the plane they're on
			for p in 0..3 {
				let vertex = mesh.vertices[mesh.triangles[t][p]];
				if vertex.dot(normal) >= plane_pos.dot(normal) { inside.push(p); }else { outside.push(p); }
			}
			match inside.len() {
				0 => { tris_to_remove.push(t); }, // triangles fully inside the plane are removed
				1 => {
					let mut new_tri = [0; 3];
					let mut tex_tri = [0; 3];
					let (p1, p2, p3) = (
						 mesh.triangles[t][inside[0]], mesh.triangles[t][outside[0]], mesh.triangles[t][outside[1]]
					);
					let (tp1, tp2, tp3) = (
						 mesh.tex_tris[t][inside[0]], mesh.tex_tris[t][outside[0]], mesh.tex_tris[t][outside[1]]
					);
					
					new_tri[inside[0]] = p1;
					tex_tri[inside[0]] = tp1;
					// find points where triangle edges intersect the plane
					let (fac1, fac2) = (
						self.line_intersect_plane(mesh.vertices[p2], mesh.vertices[p1], plane_pos, normal),
						self.line_intersect_plane(mesh.vertices[p3], mesh.vertices[p1], plane_pos, normal)
					);
					let (i1, i2) = (
						mesh.vertices[p2].add(mesh.vertices[p1].sub(mesh.vertices[p2]).mul(fac1)),
						mesh.vertices[p3].add(mesh.vertices[p1].sub(mesh.vertices[p3]).mul(fac2))
					);
					// check if there are any existing vertices at the same location
					let (mut idx1, mut idx2) = (0, 0);
					let (mut merge1, mut merge2) = (false, false);
					for idx in new_verts.iter() {
						let vertex: Vector3D = mesh.vertices[*idx];
						if vertex.sub(i1).mag() <= 0.01 { merge1 = true; idx1 = *idx; }
						if vertex.sub(i2).mag() <= 0.01 { merge2 = true; idx2 = *idx; }
					}
					// add new vertices if none exist, or anchor the triangle to the existing vertex
					if !merge1 {
						new_tri[outside[0]] = mesh.vertices.len();
						new_verts.push(mesh.vertices.len());
						mesh.vertices.push(i1);
						mesh.vertex_normals.push(Vector3D::zero()); // add new vertex normal
					}else {
						new_tri[outside[0]] = idx1;
					}
					if !merge2 {
						new_tri[outside[1]] = mesh.vertices.len();
						new_verts.push(mesh.vertices.len());
						mesh.vertices.push(i2);
						mesh.vertex_normals.push(Vector3D::zero());
					}else {
						new_tri[outside[1]] = idx2;
					}
					// interpolate texture coordinates at intersection point
					tex_tri[outside[0]] = mesh.tex_coords.len();
					tex_tri[outside[1]] = mesh.tex_coords.len()+1;
					mesh.tex_coords.push([
						mesh.tex_coords[tp2][0] + (mesh.tex_coords[tp1][0] - mesh.tex_coords[tp2][0])*fac1,
						mesh.tex_coords[tp2][1] + (mesh.tex_coords[tp1][1] - mesh.tex_coords[tp2][1])*fac1
					]);
					mesh.tex_coords.push([
						mesh.tex_coords[tp3][0] + (mesh.tex_coords[tp1][0] - mesh.tex_coords[tp3][0])*fac2,
						mesh.tex_coords[tp3][1] + (mesh.tex_coords[tp1][1] - mesh.tex_coords[tp3][1])*fac2
					]);
					
					mesh.face_normals.push(Vector3D::zero());
					
					mesh.tex_tris.push(tex_tri);
					mesh.triangles.push(new_tri);
					tris_to_remove.push(t);
				},
				2 => {
					let mut new_tri1 = [0; 3];
					let mut new_tri2 = [0; 3];
					let mut tex_tri1 = [0; 3];
					let mut tex_tri2 = [0; 3];
					let (p1, p2, p3) = (
						 mesh.triangles[t][inside[0]], mesh.triangles[t][inside[1]], mesh.triangles[t][outside[0]]
					);
					let (tp1, tp2, tp3) = (
						 mesh.tex_tris[t][inside[0]], mesh.tex_tris[t][inside[1]], mesh.tex_tris[t][outside[0]]
					);
					
					new_tri1[inside[0]] = p1;
					new_tri1[inside[1]] = p2;
					new_tri2[inside[1]] = p2;
					
					tex_tri1[inside[0]] = tp1;
					tex_tri1[inside[1]] = tp2;
					tex_tri2[inside[1]] = tp2;
					let (fac1, fac2) = (
						self.line_intersect_plane(mesh.vertices[p3], mesh.vertices[p1], plane_pos, normal),
						self.line_intersect_plane(mesh.vertices[p3], mesh.vertices[p2], plane_pos, normal)
					);
					let (i1, i2) = (
						mesh.vertices[p3].add(mesh.vertices[p1].sub(mesh.vertices[p3]).mul(fac1)),
						mesh.vertices[p3].add(mesh.vertices[p2].sub(mesh.vertices[p3]).mul(fac2))
					);
					
					let (mut idx1, mut idx2) = (0, 0);
					let (mut merge1, mut merge2) = (false, false);
					for idx in new_verts.iter() {
						let vertex: Vector3D = mesh.vertices[*idx];
						if vertex.sub(i1).mag() <= 0.01 { merge1 = true; idx1 = *idx; }
						if vertex.sub(i2).mag() <= 0.01 { merge2 = true; idx2 = *idx; }
					}
					let start_idx = mesh.vertices.len();
					if !merge1 {
						new_tri1[outside[0]] = mesh.vertices.len();
						new_tri2[inside[0]] = mesh.vertices.len();
						new_verts.push(mesh.vertices.len());
						mesh.vertices.push(i1);	
						mesh.vertex_normals.push(Vector3D::zero());
					}else {
						new_tri1[outside[0]] = idx1;
						new_tri2[inside[0]] = idx1;
					}
					if !merge2 {
						new_tri2[outside[0]] = mesh.vertices.len();
						new_verts.push(mesh.vertices.len());
						mesh.vertices.push(i2);
						mesh.vertex_normals.push(Vector3D::zero());
					}else {
						new_tri2[outside[0]] = idx2;
					}

					tex_tri1[outside[0]] = mesh.tex_coords.len();
					tex_tri2[inside[0]] = mesh.tex_coords.len();
					tex_tri2[outside[0]] = mesh.tex_coords.len()+1;
					mesh.tex_coords.push([
						mesh.tex_coords[tp3][0] + (mesh.tex_coords[tp1][0] - mesh.tex_coords[tp3][0])*fac1,
						mesh.tex_coords[tp3][1] + (mesh.tex_coords[tp1][1] - mesh.tex_coords[tp3][1])*fac1
					]);
					mesh.tex_coords.push([
						mesh.tex_coords[tp3][0] + (mesh.tex_coords[tp2][0] - mesh.tex_coords[tp3][0])*fac2,
						mesh.tex_coords[tp3][1] + (mesh.tex_coords[tp2][1] - mesh.tex_coords[tp3][1])*fac2
					]);
					
					mesh.face_normals.push(Vector3D::zero());
					mesh.face_normals.push(Vector3D::zero());
					
					mesh.tex_tris.push(tex_tri1);
					mesh.tex_tris.push(tex_tri2);
					mesh.triangles.push(new_tri1);
					mesh.triangles.push(new_tri2);
					tris_to_remove.push(t);
				},
				3 => (), // triangles entirely outside the plane are unaffected
				_ => ()
		}}
		let (mut new_tris, mut new_face_norms, mut new_tex_tris) = (Vec::new(), Vec::new(), Vec::new());
		for t in 0..mesh.triangles.len() {
			if !tris_to_remove.contains(&t) {
				new_tris.push(mesh.triangles[t]);
				new_face_norms.push(mesh.face_normals[t]);
				new_tex_tris.push(mesh.tex_tris[t]);
		}}
		mesh.triangles = new_tris;
		mesh.face_normals = new_face_norms;
		mesh.tex_tris = new_tex_tris;
	}

	pub fn draw_mesh(&mut self, mesh: &mut Mesh) {
		// near plane clipping so the projection doesn't divide by 0 or flip some vertices upside down
		self.clip_against_plane(mesh, Vector3D::XYZ(0.0, 0.0, -3.0), Vector3D::XYZ(0.0, 0.0, -1.0));
		mesh.recalculate_normals();
		for t in 0..mesh.triangles.len() {
			// backface culling
			// normal dot (triangle location - camera direction)
			if mesh.cull_backfaces {
				if mesh.face_normals[t].dot(mesh.vertices[mesh.triangles[t][0]].sub(Vector3D::XYZ(0.0, 0.0, 1.0))) > 0.0 { continue; }
			}
			let (p1, p2, p3) = (
				mesh.vertices[mesh.triangles[t][0]],
				mesh.vertices[mesh.triangles[t][1]],
				mesh.vertices[mesh.triangles[t][2]]
			);
			let (t1, t2, t3) = (
				mesh.tex_coords[mesh.tex_tris[t][0]],
				mesh.tex_coords[mesh.tex_tris[t][1]],
				mesh.tex_coords[mesh.tex_tris[t][2]],
			);
			let (n1, n2, n3) = (
				mesh.vertex_normals[mesh.triangles[t][0]].div(p1.Z),
				mesh.vertex_normals[mesh.triangles[t][1]].div(p1.Z),
				mesh.vertex_normals[mesh.triangles[t][2]].div(p1.Z)
			);
			self.draw_triangle(
				[
					VertexData::new([t1[0]/p1.Z, t1[1]/p1.Z], self.project(p1), p1, n1, 1.0/p1.Z),
					VertexData::new([t2[0]/p2.Z, t2[1]/p2.Z], self.project(p2), p2, n2, 1.0/p2.Z),
					VertexData::new([t3[0]/p3.Z, t3[1]/p3.Z], self.project(p3), p3, n3, 1.0/p3.Z)
				],
				&mesh.texture,
				&mesh.material,
				mesh.face_normals[t]
		);}
	}
	
	pub fn draw_wireframe(&mut self, mesh: &Mesh) {
		for t in 0..mesh.triangles.len() {
			if mesh.cull_backfaces {
				if mesh.face_normals[t].dot(mesh.vertices[mesh.triangles[t][0]].sub(Vector3D::XYZ(0.0, 0.0, 1.0))) > 0.0 { continue; }
			}
			let (p1, p2, p3) = (
				mesh.vertices[mesh.triangles[t][0]],
				mesh.vertices[mesh.triangles[t][1]],
				mesh.vertices[mesh.triangles[t][2]],
			);
			let (v1, v2, v3) = (
				VertexData::new([0.0; 2], self.project(p1), p1.div(p1.Z), Vector3D::zero(), 1.0/p1.Z),
				VertexData::new([0.0; 2], self.project(p2), p2.div(p2.Z), Vector3D::zero(), 1.0/p2.Z),
				VertexData::new([0.0; 2], self.project(p3), p3.div(p3.Z), Vector3D::zero(), 1.0/p3.Z)
			);
			let mut points = Vec::new();
			points.extend(self.draw_line(v1, v2));
			points.extend(self.draw_line(v2, v3));
			points.extend(self.draw_line(v3, v1));
			for p in points {
				if (p.screenXY[0] < 0.0) || (p.screenXY[0] >= self.width as f32) { continue; }
				if (p.screenXY[1] < 0.0) || (p.screenXY[1] >= self.height as f32) { continue; }
				self.pixel_buffer[p.screenXY[1] as usize][p.screenXY[0] as usize] = Color::RGB(0.988, 0.784, 0.353);
		}}
	}
	
	pub fn draw_flat_texture(&mut self, x_offset: usize, y_offset: usize, tex: Texture) {
		for y in 0..tex.height {
			for x in 0..tex.width {
				self.pixel_buffer[y+y_offset][x+x_offset] = tex.bitmap[y][x];
		}}
	}
}
