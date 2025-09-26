use std::{ thread, time };
use mesh::{ Mesh, Transform };
use graphicsutils::{ LightSource, LightingMode, Texture, Material };
use viewport::Viewport;

use std::fs::File;
use std::io::Read;

use regex::Regex;

mod mesh;
mod viewport;
mod graphicsutils;

type Point2D = (f32, f32);
type Triangle = (usize, usize, usize);

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
		Vector3D { X: self.X + other.X, Y: self.Y + other.Y, Z: self.Z + other.Z }
	}
	
	fn sub(&self, other: Vector3D) -> Vector3D {
		Vector3D { X: self.X - other.X, Y: self.Y - other.Y, Z: self.Z - other.Z }
	}
	
	fn mul(&self, fac: f32) -> Vector3D {
		Vector3D { X: fac*self.X, Y: fac*self.Y, Z: fac*self.Z }
	}
	
	fn div(&self, fac: f32) -> Vector3D {
		Vector3D { X: self.X / fac, Y: self.Y / fac, Z: self.Z / fac }
	}
	
	fn hadamard(&self, other: Vector3D) -> Vector3D {
		Vector3D { X: self.X * other.X, Y: self.Y * other.Y, Z: self.Z * other.Z }
	}
	
	fn dot(&self, other: Vector3D) -> f32 {
		self.X*other.X + self.Y*other.Y + self.Z*other.Z
	}
	
	fn mag(&self) -> f32 {
		(self.X*self.X + self.Y*self.Y + self.Z*self.Z).sqrt()
	}
	
	fn normalize(&self) -> Vector3D {
		let mag = self.mag();
		Vector3D { X: self.X / mag, Y: self.Y / mag, Z: self.Z / mag }
	}
	
	fn cross(&self, other: Vector3D) -> Vector3D {
		Vector3D {
			X: self.Y*other.Z - self.Z*other.Y,
			Y: self.Z*other.X - self.X*other.Z,
			Z: self.X*other.Y - self.Y*other.X
		}
	}
	
	fn lerp(&self, other: Vector3D, fac: f32) -> Vector3D {
		self.add(other.sub(*self).mul(fac))
	}
	
	// reflect self across other
	fn reflect(&self, other: Vector3D) -> Vector3D {
		let axis = other.normalize();
		self.sub(axis.mul(2.0).mul(self.dot(axis)))
	}
}

#[derive(Copy, Clone, Debug)]
struct Color {
	RGB: (f32, f32, f32) // r, g, b are stored as ranges 0-1
}

impl Color {
	fn RGB(R: f32, G: f32, B: f32) -> Color { Color { RGB: (R, G, B) } }
	
	fn black() -> Color { Color { RGB: (0.0, 0.0, 0.0) } }
	
	fn to_24bit(&self) -> (usize, usize, usize) {
		(
			(self.RGB.0*255.0) as usize,
			(self.RGB.1*255.0) as usize,
			(self.RGB.2*255.0) as usize
		)
	}
	
	fn lerp(&self, other: Color, fac: f32) -> Color {
		Color { RGB: (
			self.RGB.0 + (other.RGB.0 - self.RGB.0)*fac,
			self.RGB.1 + (other.RGB.1 - self.RGB.1)*fac,
			self.RGB.2 + (other.RGB.2 - self.RGB.2)*fac
		)}
	}
	
	fn hadamard(&self, other: Color) -> Color {
		Color { RGB: (self.RGB.0 * other.RGB.0, self.RGB.1 * other.RGB.1, self.RGB.2 * other.RGB.2) }
	}
	
	fn mul(&self, fac: f32) -> Color {
		Color { RGB: (fac*self.RGB.0, fac*self.RGB.1, fac*self.RGB.2) }
	}
	
	fn add(&self, other: Color) -> Color {
		Color { RGB: (
			clamp(0.0, 1.0, self.RGB.0 + other.RGB.0),
			clamp(0.0, 1.0, self.RGB.1 + other.RGB.1),
			clamp(0.0, 1.0, self.RGB.2 + other.RGB.2)
		)}
	}
}

fn clamp(min: f32, max: f32, val: f32) -> f32 {
	if val >= max { max }else if val < min { min }else { val }
}


fn load_bitmap(filename: &str) -> std::io::Result<Texture> {
	println!("importing image: {filename}");
	let mut file = File::open(format!("./textures/{filename}.ppm"))?;
	let mut image_data = String::new();
	file.read_to_string(&mut image_data)?;
	let to_usize = |s: &str| s.to_string().parse::<usize>().unwrap();
	
	let match_header = Regex::new("P3[\n ](?<w>[0-9]+)[\n ](?<h>[0-9]+)[\n ]255").unwrap();
	let match_pixel = Regex::new("(?<r>[0-9]{1,3})[ ]+(?<g>[0-9]{1,3})[ ]+(?<b>[0-9]{1,3})").unwrap();
	
	print!("extracting header...");
	let (header, [w, h]) = if let Some(capture) = match_header.captures(&image_data) { capture.extract() }
	else {
		println!("error: unable to recognize header, check if the image is ppm version 3");
		return Ok(Texture::missing(10, 10, 1));
	};
	let (width, height) = (to_usize(w), to_usize(h));
	image_data = (&image_data[header.len()..]).to_string();
	println!("done!");
	
	print!("extracting color data...");
	let (mut pix_buf, mut pix_row) = (Vec::new(), Vec::new());
	for (i, c) in match_pixel.captures_iter(&image_data).enumerate() {
		pix_row.push(Color::RGB(to_usize(&c["r"]) as f32 / 255.0, to_usize(&c["g"]) as f32 / 255.0, to_usize(&c["b"]) as f32 / 255.0));
		if (i+1) % width == 0 { pix_buf.push(pix_row.clone()); pix_row.clear();}
	}
	println!("done!");
	println!("texture imported successfully!");
	
	Ok(Texture::new(width, height, pix_buf))
}


fn load_material(filename: String) -> std::io::Result<(Material, Texture)> {
	println!("importing material: {filename}");
	let mut mtl = File::open(format!("./materials/{filename}"))?;
	let mut mtl_data = String::new();
	mtl.read_to_string(&mut mtl_data);

	let attrib_patterns = vec![
		("header", Regex::new("newmtl (?<result>[a-zA-Z0-9_-]+)\n").unwrap()),
		("ambient", Regex::new("Ka (?<result>[0-9]+.[0-9]+ [0-9]+.[0-9]+ [0-9]+.[0-9]+)\n").unwrap()),
		("diffuse", Regex::new("Kd (?<result>[0-9]+.[0-9]+ [0-9]+.[0-9]+ [0-9]+.[0-9]+)\n").unwrap()),
		("specular", Regex::new("Ks (?<result>[0-9]+.[0-9]+ [0-9]+.[0-9]+ [0-9]+.[0-9]+)\n").unwrap()),
		("highlights", Regex::new("Ns (?<result>[0-9]+.?[0-9]*)\n").unwrap()),
		("opacity", Regex::new("d (?<result>[0-9]+.?[0-9]*)\n").unwrap()),
		("texture", Regex::new("map_Kd (?<result>[a-zA-Z0-9_-]+).ppm").unwrap())
	];
	
	let mut string_components = Vec::new();
	let mut material = Material::missing();
	let mut texture = Texture::missing(10, 10, 1);
	
	for attrib in attrib_patterns.iter() {
		print!("reading material component {}... ", attrib.0);
		if let Some(capture) = attrib.1.captures(&mtl_data) {
			let component = capture["result"].to_string();
			println!("{component}");
			string_components.push((attrib.0, component));
		}else {
			println!("component missing, setting to default");
	}}
	let unpack_color = |component: String| {
		let RGB: Vec<f32> = component.split(" ").map(|s| s.parse::<f32>().unwrap()).collect();
		Color::RGB(RGB[0], RGB[1], RGB[2])
	};
	
	for component in string_components {
		match component.0 {
			"ambient" => { material.ambient = unpack_color(component.1); },
			"diffuse" => { material.diffuse = unpack_color(component.1); },
			"specular" => { material.specular = unpack_color(component.1);},
			"highlights" => { material.highlights = component.1.parse::<f32>().unwrap(); },
			"opacity" => { material.opacity = component.1.parse::<f32>().unwrap(); },
			"texture" => { texture = load_bitmap(&component.1)?; },
			"header" => (),
			other => {
				println!("error: unrecognized component: {other}");
				return Ok((Material::missing(), Texture::missing(10, 10, 1)));
			}
	}}
	println!("material imported successfully!");
	Ok((material, texture))
}


fn load_object(filename: &str) -> std::io::Result<Mesh> {
	println!("importing object: {filename}.obj");
	let mut obj_file = File::open(format!("./objects/{filename}.obj"))?;
	let mut obj_data = String::new();
	obj_file.read_to_string(&mut obj_data)?;
	
	let num = r"-?[0-9]+.[0-9]+e?(\+|-)?[0-9]*";
	let match_mtl_filename = Regex::new("mtllib (?<mtlfile>[a-zA-Z0-9_-]+.mtl)").unwrap();
	let match_geometry_vertex = Regex::new(&format!("v {num} {num} {num}")).unwrap();
	let match_texture_coord = Regex::new(&format!("vt {num} {num}")).unwrap();
	
	let detect_tri = Regex::new("f [0-9]+/?(?<tx>[0-9]*)/?(?<vn>[0-9]*)").unwrap();
	
	let to_usize = |s: &str| s.parse::<usize>().unwrap();
	let to_f32 = |s: &str| s.parse::<f32>().unwrap();

	print!("detecting material file... ");
	
	let mtl_filename = if let Some(capture) = match_mtl_filename.captures(&obj_data) { Some(capture["mtlfile"].to_string()) }
	else { None };
	if mtl_filename.is_some() { println!("{}", mtl_filename.clone().unwrap()); }else { println!("no material file"); }
	
	print!("detecting triangle data format... ");
	let (_, [tex, norm]) = if let Some(capture) = detect_tri.captures(&obj_data) { capture.extract() }
	else {
		println!("error: unable to recognize triangle data!");
		return Ok(Mesh::empty());
	};
	
	let mut tri = "[0-9]+".to_string();
	let (tex_coords_included, normals_included) = (tex.len() != 0, norm.len() != 0);
	if tex_coords_included { tri.push_str("/[0-9]+"); }
	if normals_included {
		if tex_coords_included { tri.push_str("/[0-9]+"); }else { tri.push_str("//[0-9]+"); }
	}
	let match_face_data = Regex::new(&format!("f {tri} {tri} {tri}")).unwrap();
	
	println!("normals: {normals_included}, texture coordinates: {tex_coords_included}");
	
	// normals can be easily derived from other mesh data
	// vn values can be either vertex or face normals, not worth the extra implementation complexity tbh
	// I'll implement a system to handle this logic if it ever becomes necessary
	let mut vertices: Vec<Vector3D> = Vec::new();
	let mut tex_coords: Vec<Point2D> = Vec::new();
	let mut triangles: Vec<Triangle> = Vec::new();
	let mut tex_tris: Vec<Triangle> = Vec::new();
	
	print!("reading vertex data... ");
	for v in match_geometry_vertex.captures_iter(&obj_data) {
		let vertex: Vec<&str> = v.get(0).unwrap().as_str().split(" ").collect();
		vertices.push(Vector3D::XYZ(to_f32(vertex[1]), to_f32(vertex[2]), to_f32(vertex[3])));
	}
	println!("done!");

	if tex_coords_included {
		print!("reading texture coordinate data... ");
		for vt in match_texture_coord.captures_iter(&obj_data) {
			let texcoord: Vec<&str> = vt.get(0).unwrap().as_str().split(" ").collect();
			tex_coords.push((to_f32(texcoord[1]), to_f32(texcoord[2])));
		}
		println!("done!");
	}else {
		tex_coords.push((0.0, 0.0));
	}
	
	print!("reading triangle data... ");
	for f in match_face_data.captures_iter(&obj_data) {
		let tri_verts: Vec<&str> = f.get(0).unwrap().as_str().split(" ").skip(1).collect();
		let mut triangle_data = Vec::new();
		
		for v in tri_verts {
			let data: Vec<&str> = v.split("/").collect();
			let vertex_id = to_usize(data[0]);
			
			let uv_id = if tex_coords_included { to_usize(data[1]) }else { 1 };
			triangle_data.push([vertex_id, uv_id]);
		}
		triangles.push((triangle_data[0][0]-1, triangle_data[1][0]-1, triangle_data[2][0]-1));
		tex_tris.push((triangle_data[0][1]-1, triangle_data[1][1]-1, triangle_data[2][1]-1));
	}
	println!("done!");
	
	let (mut material, mut texture) = (Material::missing(), Texture::missing(10, 10, 1));
	if mtl_filename.is_some() {
		(material, texture) = load_material(mtl_filename.unwrap())?;
	}

	let mut object = Mesh{
		vertices: vertices.clone(),
		triangles: triangles.clone(),
		tex_coords,
		tex_tris,
		face_normals: vec![Vector3D::zero(); triangles.len()],
		vertex_normals: vec![Vector3D::zero(); vertices.len()],
		origin: Vector3D::zero(),
		texture,
		material
	};
	print!("deriving mesh properties... ");
	object.recalculate_normals();
	object.origin = object.center();
	println!("done!");
	
	println!("object imported successfully!\n");
	Ok(object)
}


fn main() {
    let mut screen = Viewport::new(160, 120, 120.0, Color::RGB(0.251, 0.263, 0.655)); //64, 67, 167
	let mut cube = load_object("column").unwrap();
	let tex = load_bitmap("space_1").unwrap();
	cube.texture = tex;
	
	cube.transform(Transform::Translate(Vector3D::XYZ(0.0, -5.0, -5.0)));
	cube.transform(Transform::Scale(Vector3D::XYZ(2.0, 2.0, 2.0)));
	cube.material.mode = LightingMode::Smooth;
	cube.material.diffuse = Color::RGB(0.1, 0.3, 0.9);
	
	screen.lights.push(LightSource::new(Color::RGB(0.9, 0.9, 0.9), Vector3D::XYZ(30.0, 20.0, -5.0)));
	screen.lights.push(LightSource::new(Color::RGB(0.9, 0.9, 0.9), Vector3D::XYZ(-30.0, -20.0, -5.0)));
	
	cube.transform(Transform::Rotate(Vector3D::XYZ(1.0, 0.6, -0.01), Vector3D::XYZ(1.0, -0.5, 0.3)));
	screen.draw_mesh(&cube);
	screen.display();

	for i in 0..2 {
		cube.transform(Transform::Rotate(Vector3D::XYZ(1.0, 0.01, -0.01), Vector3D::XYZ(1.0, 0.02, 0.0)));
		//cube2.transform(Transform::Rotate(Vector3D::XYZ(0.02, -1.02, 0.01), Vector3D::XYZ(0.0, 1.02, 0.0)));
		
		//screen.draw_mesh(&cube2);
		let mut clipped_cube = cube.clone();
		screen.clip_against_plane(&mut clipped_cube, Vector3D::XYZ(0.0, 0.0, -3.0), Vector3D::XYZ(0.0, 0.0, -1.0));
		
		screen.draw_mesh(&clipped_cube);
		//screen.draw_mesh(&cube);
		//screen.draw_wireframe(&cube);
		screen.display();
		screen.clear_screen();
		thread::sleep(time::Duration::from_millis(50));
	}

}

