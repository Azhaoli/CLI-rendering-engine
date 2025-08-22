use crate::{ Triangle, Color, Vector3D, Point2D };
use crate::clamp;

#[derive(Clone)]
pub enum LightingMode {
	Flat,
	Smooth,
	None
}

#[derive(Clone)]
pub struct Material {
	pub ambient: Color,
	pub diffuse: Color,
	pub specular: Color,
	pub highlights: f32,
	pub opacity: f32,
	pub mode: LightingMode,
}

impl Material {
	pub fn new(ambient: Color, diffuse: Color, specular: Color, highlights: f32, opacity: f32, mode: LightingMode) -> Material {
		Material{ ambient, diffuse, specular, highlights, opacity, mode }
	}
	
	pub fn missing() -> Material {
		Material {
			ambient: Color::RGB(0.75, 0.75, 0.75),
			diffuse: Color::RGB(1.0, 0.0, 1.0),
			specular: Color::RGB(1.0, 1.0, 1.0),
			highlights: 20.0,
			opacity: 1.0,
			mode: LightingMode::None
		}
	}
}

#[derive(Clone)]
pub struct Texture {
	pub width: usize,
	pub height: usize,
	pub bitmap: Vec<Vec<Color>>
}

impl Texture {
	pub fn new(width: usize, height: usize, bitmap: Vec<Vec<Color>>) -> Texture { Texture{ width, height, bitmap } }
	
	pub fn missing(width: usize, height: usize, size: usize) -> Texture {
		let (mut bit_row1, mut bit_row2, mut bitmap) = (Vec::new(), Vec::new(), Vec::new());
		for w in 0..width {
			if w % (2*size) >= size {
				bit_row1.push(Color::RGB(0.5, 0.5, 0.5));
				bit_row2.push(Color::RGB(0.9, 0.9, 0.9));
			}else {
				bit_row1.push(Color::RGB(0.9, 0.9, 0.9));
				bit_row2.push(Color::RGB(0.5, 0.5, 0.5));
			}
		}
		for h in 0..height {
			if h % (2*size) >= size { bitmap.push(bit_row1.clone()); }
			else { bitmap.push(bit_row2.clone()); }
		}
		Texture{ width, height, bitmap }
	}
	
	fn lerp_color(&self, c1: Color, c2: Color, fac: f32) -> Color {
		Color::RGB(
			c1.RGB[0] + (c2.RGB[0] - c1.RGB[0])*fac,
			c1.RGB[1] + (c2.RGB[1] - c1.RGB[1])*fac,
			c1.RGB[2] + (c2.RGB[2] - c1.RGB[2])*fac
		)
	}
	
	pub fn sample(&self, UV: Point2D) -> Color {
		// clamp U and V
		let u = clamp(0.0, 1.0, UV[0]);
		let v = clamp(0.0, 1.0, UV[1]);
		let (tx, ty) = (u * (self.width-1) as f32, v * (self.height-1) as f32);
		
		let (u_fac, v_fac) = (tx.fract(), ty.fract());
		let (c0, c1, c2, c3) = (
			self.bitmap[ty.floor() as usize][tx.floor() as usize],
			self.bitmap[ty.floor() as usize][tx.ceil() as usize],
			self.bitmap[ty.ceil() as usize][tx.floor() as usize],
			self.bitmap[ty.ceil() as usize][tx.ceil() as usize]
		);
		let (c01, c23) = (
			self.lerp_color(c0, c1, u_fac), self.lerp_color(c2, c3, u_fac)
		);
		self.lerp_color(c01, c23, v_fac)
	}
}

pub enum Transform {
	RotateX(f32),
	RotateY(f32),
	RotateZ(f32),
	Translate(Vector3D),
	Scale(Vector3D)
}

#[derive(Clone)]
pub struct Mesh {
	pub vertices: Vec<Vector3D>,
	pub triangles: Vec<Triangle>,
	pub tex_coords: Vec<Point2D>,
	pub tex_tris: Vec<Triangle>,
	
	pub face_normals: Vec<Vector3D>,
	pub vertex_normals: Vec<Vector3D>,
	pub origin: Vector3D,
	pub texture: Texture,
	pub material: Material,
	pub cull_backfaces: bool
}

impl Mesh {
	pub fn new(vertices: Vec<Vector3D>, triangles: Vec<Triangle>) -> Mesh {
		let mut unset_face_normals = Vec::new();
		let mut unset_vertex_normals = Vec::new();
		for t in 0..triangles.len() { unset_face_normals.push(Vector3D::zero()); }
		for t in 0..vertices.len() { unset_vertex_normals.push(Vector3D::zero()); }
		
		Mesh{
			vertices,
			triangles,
			tex_coords: Vec::new(),
			tex_tris: Vec::new(),
			face_normals: unset_face_normals,
			vertex_normals: unset_vertex_normals,
			origin: Vector3D::zero(),
			texture: Texture::missing(10, 10, 2),
			material: Material::missing(),
			cull_backfaces: true
		}
	}
	
	pub fn empty() -> Mesh {
		Mesh{
			vertices: Vec::new(),
			triangles: Vec::new(),
			tex_coords: Vec::new(),
			tex_tris: Vec::new(),
			face_normals: Vec::new(),
			vertex_normals: Vec::new(),
			origin: Vector3D::zero(),
			texture: Texture::missing(1, 1, 1),
			material: Material::missing(),
			cull_backfaces: true
		}
	}
	
	pub fn center(&self) -> Vector3D {
		let mut center = Vector3D::zero();
		let num_vertices = self.vertices.len() as f32;
		for v in self.vertices.iter() { center = center.add(*v); }
		center.div(num_vertices)
	}
	
	pub fn transform(&mut self, transform: Transform) {
		self.vertices = match transform {
			Transform::RotateX(angle) => self.vertices.iter().map(|v| v.sub(self.origin).rotate_x(angle).add(self.origin)).collect(),
			Transform::RotateY(angle) => self.vertices.iter().map(|v| v.sub(self.origin).rotate_y(angle).add(self.origin)).collect(),
			Transform::RotateZ(angle) => self.vertices.iter().map(|v| v.sub(self.origin).rotate_z(angle).add(self.origin)).collect(),
			Transform::Translate(vec) => {
				self.origin = self.origin.add(vec);
				self.vertices.iter().map(|v| v.add(vec)).collect()
			},
			Transform::Scale(vec) => self.vertices.iter().map(|v| v.sub(self.origin).mul_elements(vec).add(self.origin)).collect()
		};
	}
	
	pub fn recalculate_normals(&mut self) {
		for t in 0..self.triangles.len() {
			let (p1, p2, p3) = (
				self.vertices[self.triangles[t][0]],
				self.vertices[self.triangles[t][1]],
				self.vertices[self.triangles[t][2]]
			);
			let (l1, l2) = (
				p2.sub(p1),
				p3.sub(p1)
			);
			let normal = l1.cross(l2).normalize();
			self.face_normals[t] = normal;
			self.vertex_normals[self.triangles[t][0]] = self.vertex_normals[self.triangles[t][0]].add(normal);
			self.vertex_normals[self.triangles[t][1]] = self.vertex_normals[self.triangles[t][1]].add(normal);
			self.vertex_normals[self.triangles[t][2]] = self.vertex_normals[self.triangles[t][2]].add(normal);
		}
		for v in 0..self.vertices.len() { self.vertex_normals[v] = self.vertex_normals[v].normalize(); }
	}
}


