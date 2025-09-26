use crate::{ Triangle, Vector3D, Point2D };
use crate::graphicsutils::{ Texture, Material };

pub enum Transform {
	Scale(Vector3D),
	Translate(Vector3D),
	Rotate(Vector3D, Vector3D)
}

#[derive(Clone)]
pub struct Mesh {
	pub vertices: Vec<Vector3D>,
	pub triangles: Vec<Triangle>,
	
	pub tex_coords: Vec<Point2D>,
	pub tex_tris: Vec<Triangle>,

	pub face_normals: Vec<Vector3D>,
	pub vertex_normals: Vec<Vector3D>,

	pub texture: Texture,
	pub material: Material,
	pub origin: Vector3D
}

impl Mesh {
	pub fn new(vertices: Vec<Vector3D>, triangles: Vec<Triangle>) -> Mesh {
		Mesh{			
			tex_coords: Vec::new(),
			tex_tris: Vec::new(),
			
			vertex_normals: vec![Vector3D::zero(); vertices.len()],
			face_normals: vec![Vector3D::zero(); triangles.len()],
			
			vertices,
			triangles,

			texture: Texture::missing(10, 10, 2),
			material: Material::missing(),
			origin: Vector3D::zero(),
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
			texture: Texture::missing(10, 10, 1),
			material: Material::missing(),
		}
	}
	
	pub fn center(&self) -> Vector3D {
		let mut center = Vector3D::zero();
		// find center of mesh bounding box
		let (mut x_min, mut x_max) = (999.0, -999.0);
		let (mut y_min, mut y_max) = (999.0, -999.0);
		let (mut z_min, mut z_max) = (999.0, -999.0);
		for v in self.vertices.iter() {
			if x_min > v.X { x_min = v.X; }
			if x_max < v.X { x_max = v.X; }
			if y_min > v.Y { y_min = v.Y; }
			if y_max < v.Y { y_max = v.Y; }
			if z_min > v.Z { z_min = v.Z; }
			if z_max < v.Z { z_max = v.Z; }
		}
		Vector3D::XYZ((x_max-x_min)/2.0, (y_max-y_min)/2.0, (z_max-z_min)/2.0)
	}
	
	pub fn transform(&mut self, action: Transform) {
		self.vertices = match action {
			// rotatation using double reflection
			Transform::Rotate(a, b) => {
				// rotate normals so they don't need to be recalculted each frame
				self.face_normals = self.face_normals.iter().map(|f| f.reflect(a).reflect(b)).collect();
				self.vertex_normals = self.vertex_normals.iter().map(|v| v.reflect(a).reflect(b)).collect();
				self.vertices.iter().map(|v| v.sub(self.origin).reflect(a).reflect(b).add(self.origin)).collect()
			},
			Transform::Translate(vec) => {
				self.origin = self.origin.add(vec);
				self.vertices.iter().map(|v| v.add(vec)).collect()
			},
			Transform::Scale(vec) => self.vertices.iter().map(|v| v.sub(self.origin).hadamard(vec).add(self.origin)).collect()
		};
	}
	
	pub fn recalculate_normals(&mut self) {
		for t in 0..self.triangles.len() {
			let (t1, t2, t3) = self.triangles[t];
			let (p1, p2, p3) = (self.vertices[t1], self.vertices[t2], self.vertices[t3]);
			
			let (l1, l2) = (p2.sub(p1), p3.sub(p1));
			let normal = l1.cross(l2).normalize();
			
			self.face_normals[t] = normal;
			self.vertex_normals[t1] = self.vertex_normals[t1].add(normal);
			self.vertex_normals[t2] = self.vertex_normals[t2].add(normal);
			self.vertex_normals[t3] = self.vertex_normals[t3].add(normal);
		}
		for v in 0..self.vertices.len() { self.vertex_normals[v] = self.vertex_normals[v].normalize(); }
	}
}
