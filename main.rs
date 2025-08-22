use mesh::{ Mesh, Texture, Transform, LightingMode };
use viewport::{ Viewport, LightSource };
use graphicsutils::ambient_occlusion;
use exportutils::{ write_bitmap, load_bitmap, load_object };

mod viewport;
mod mesh;
mod exportutils;
mod graphicsutils;

type Point2D = [f32; 2];
type Triangle = [usize; 3];

#[derive(Copy, Clone, Debug)]
struct Vector3D {
	X: f32,
	Y: f32,
	Z: f32
}

impl Vector3D {
	fn XYZ(X: f32, Y: f32, Z: f32) -> Vector3D { Vector3D{ X, Y, Z } }
	
	fn zero() -> Vector3D { Vector3D{ X: 0.0, Y: 0.0, Z: 0.0 } }
	
	fn add(&self, other: Vector3D) -> Vector3D {
		Vector3D {
			X: self.X + other.X,
			Y: self.Y + other.Y,
			Z: self.Z + other.Z
		}
	}
	
	fn sub(&self, other: Vector3D) -> Vector3D {
		Vector3D {
			X: self.X - other.X,
			Y: self.Y - other.Y,
			Z: self.Z - other.Z
		}
	}
	
	fn mul(&self, fac: f32) -> Vector3D {
		Vector3D {
			X: fac*self.X,
			Y: fac*self.Y,
			Z: fac*self.Z
		}
	}
	
	fn div(&self, fac: f32) -> Vector3D {
		Vector3D {
			X: self.X / fac,
			Y: self.Y / fac,
			Z: self.Z / fac
		}
	}
	
	fn mul_elements(&self, other: Vector3D) -> Vector3D {
		Vector3D {
			X: self.X * other.X,
			Y: self.Y * other.Y,
			Z: self.Z * other.Z
		}
	}
	
	fn dot(&self, other: Vector3D) -> f32 {
		self.X*other.X + self.Y*other.Y + self.Z*other.Z
	}
	
	fn mag(&self) -> f32 {
		(self.X*self.X + self.Y*self.Y + self.Z*self.Z).sqrt()
	}
	
	fn normalize(&self) -> Vector3D {
		let mag = (self.X*self.X + self.Y*self.Y + self.Z*self.Z).sqrt();
		Vector3D {
			X: self.X / mag,
			Y: self.Y / mag,
			Z: self.Z / mag
		}
	}
	
	fn cross(&self, other: Vector3D) -> Vector3D {
		Vector3D {
			X: self.Y*other.Z - self.Z*other.Y,
			Y: self.Z*other.X - self.X*other.Z,
			Z: self.X*other.Y - self.Y*other.X
		}
	}
	
	fn rotate_x(&self, angle: f32) -> Vector3D {
		let (c, s) = (angle.cos(), angle.sin());
		Vector3D {
			X: self.X,
			Y: self.Y*c - self.Z*s,
			Z: self.Y*s + self.Z*c
		}
	}
	
	fn rotate_y(&self, angle: f32) -> Vector3D {
		let (c, s) = (angle.cos(), angle.sin());
		Vector3D {
			X: self.X*c - self.Z*s,
			Y: self.Y,
			Z: self.X*s + self.Z*c
		}
	}
	
	fn rotate_z(&self, angle: f32) -> Vector3D {
		let (c, s) = (angle.cos(), angle.sin());
		Vector3D {
			X: self.X*c - self.Y*s,
			Y: self.X*s + self.Z*c,
			Z: self.Z
		}
	}
	
	// reflect self across other
	fn reflect(&self, other: Vector3D) -> Vector3D {
		let para = other.mul(self.dot(other)); // component of self parallel to other
		let perp = self.sub(para); // component of self perpendicular to other
		para.sub(perp)
	}
}

#[derive(Copy, Clone, Debug)]
struct Color {
	RGB: [f32; 3] // R, G, ang B are ranges 0-1
}

impl Color {
	fn RGB(R: f32, G: f32, B: f32) -> Color { Color{ RGB: [R, G, B] } }
	
	fn add(&self, other: Color) -> Color {
		Color { RGB: [
			clamp(0.0, 1.0, self.RGB[0] + other.RGB[0]),
			clamp(0.0, 1.0, self.RGB[1] + other.RGB[1]),
			clamp(0.0, 1.0, self.RGB[2] + other.RGB[2])
		]}
	}
	
	fn sub(&self, other: Color) -> Color {
		Color { RGB: [
			clamp(0.0, 1.0, self.RGB[0] - other.RGB[0]),
			clamp(0.0, 1.0, self.RGB[1] - other.RGB[1]),
			clamp(0.0, 1.0, self.RGB[2] - other.RGB[2])
		]}
	}
	
	fn mul_elements(&self, other: Color) -> Color {
		Color { RGB: [
			self.RGB[0] * other.RGB[0],
			self.RGB[1] * other.RGB[1],
			self.RGB[2] * other.RGB[2]
		]}
	}
	
	fn mul(&self, fac: f32) -> Color {
		Color { RGB: [
			self.RGB[0] * fac,
			self.RGB[1] * fac,
			self.RGB[2] * fac
		]}
	}
}

fn clamp(min: f32, max: f32, val: f32) -> f32 {
	if val >= max { max }else if val < min { min }else { val }
}

fn main() {
	// import assets
	let mut bunny = match load_object(String::from("stanford_bunny")) {
		Ok(Mesh) => Mesh,
		Err(e) => {
			println!("error: {:?}\n", e);
			Mesh::empty()
	}};
	
	let mut cube = match load_object(String::from("cube")) {
		Ok(Mesh) => Mesh,
		Err(e) => {
			println!("error: {:?}\n", e);
			Mesh::empty()
	}};
	
	let mut plane = match load_object(String::from("plane")) {
		Ok(Mesh) => Mesh,
		Err(e) => {
			println!("error: {:?}\n", e);
			Mesh::empty()
	}};
	
	let mut column = match load_object(String::from("column")) {
		Ok(Mesh) => Mesh,
		Err(e) => {
			println!("error: {:?}\n", e);
			Mesh::empty()
	}};
	
	let mut space_texture = match load_bitmap(String::from("space_1")) {
		Ok(bmp) => bmp,
		Err(e) => {
			println!("error: {:?}\n", e);
			Texture::missing(10, 10, 1)
	}};
	
	// setup render environment
	//let bg = Color::RGB(1.0, 0.8, 0.95);
	let bg = Color::RGB(0.0, 0.0, 0.0);

	let mut screen = Viewport::new(1140, 840, 260.0, bg);
	
	bunny.origin = bunny.center();
	bunny.transform(Transform::Translate(Vector3D::XYZ(0.0, 0.0, -10.0)));
	
	bunny.transform(Transform::Scale(Vector3D::XYZ(50.0, 50.0, 50.0)));
	bunny.material.mode = LightingMode::Smooth;
	bunny.recalculate_normals();
	
	// skybox
	cube.origin = cube.center();
	cube.transform(Transform::Scale(Vector3D::XYZ(50.0, 50.0, 50.0)));
	//cube.transform(Transform::RotateZ(0.485));
	//cube.transform(Transform::RotateY(0.485));
	
	cube.cull_backfaces = false;
	cube.texture = Texture::missing(1000, 1000, 10);
	//cube.texture = space_texture;
	cube.material.mode = LightingMode::Flat;
	cube.recalculate_normals();
	
	plane.origin = plane.center();
	plane.transform(Transform::Translate(Vector3D::XYZ(0.0, -3.5, -12.0)));
	plane.transform(Transform::Scale(Vector3D::XYZ(15.0, 15.0, 15.0)));
	plane.material.mode = LightingMode::Flat;
	plane.recalculate_normals();
	
	column.origin = column.center();
	column.transform(Transform::Scale(Vector3D::XYZ(0.9, 0.9, 0.9)));
	column.material.mode = LightingMode::Smooth;
	
	let mut column2 = column.clone();
	column2.transform(Transform::Translate(Vector3D::XYZ(5.0, -3.3, -4.5)));
	column2.transform(Transform::RotateY(0.485));
	column2.recalculate_normals();
	
	column.transform(Transform::Translate(Vector3D::XYZ(-5.0, -3.3, -4.5)));
	column.transform(Transform::RotateY(-0.485));
	
	column.recalculate_normals();
	
	let mut column3 = column.clone();
	column3.transform(Transform::Translate(Vector3D::XYZ(0.0, 0.0, -4.0)));
	
	let mut column4 = column2.clone();
	column4.transform(Transform::Translate(Vector3D::XYZ(0.0, 0.0, -4.0)));
	
	screen.lights.push(LightSource::new(Color::RGB(1.0, 1.0, 1.0), Vector3D::XYZ(-1.0, -2.0, -1.0)));
	screen.lights.push(LightSource::new(Color::RGB(1.0, 1.0, 1.0), Vector3D::XYZ(1.0, 2.0, -1.0)));
	
	screen.draw_mesh(&mut column);
	screen.draw_mesh(&mut column2);
	screen.draw_mesh(&mut column3);
	screen.draw_mesh(&mut column4);
	
	screen.draw_mesh(&mut bunny);
	
	
	screen.draw_mesh(&mut plane);
	screen.draw_mesh(&mut cube);
	screen.draw_wireframe(&mut plane);
	screen.draw_wireframe(&mut cube);
	
	//ambient_occlusion(&mut screen, 50, 200, 10.0, 0.01);
	//screen.gaussian_blur(2.0); // smooth the edges to make the image less zigzags
	
	write_bitmap(String::from("RENDER"), screen.render_to_image());
}

