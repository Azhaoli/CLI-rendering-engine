// the functions listed here are no longer accessible through the last version's API, load_bitmap, load_material and load_object have been replaced
// with newer versions in main.rs for easier debugging, I'm still working on optimizing write_bitmap and write_object
// this code and the file export API will be fully replaced with a newer version when new implementations of these functions are availible

use crate::mesh::{ Mesh, Texture, Material };
use crate::{ Vector3D, Point2D, Triangle, Color };
use std::fs::File;
use std::io::{ Read, Write };

use regex::Regex;

pub fn load_bitmap(filename: String) -> std::io::Result<Texture> {
	println!("importing image: {filename}.ppm");
	let mut img = File::open(format!("{filename}.ppm"))?;
	let mut content = String::new();
	img.read_to_string(&mut content)?;

	
	let inline = content.replace("\n", " ");
	let arr: Vec<&str> = inline.split(" ").filter(|c| !c.is_empty()).collect();
	
	print!("verifying format... ");
	if arr[0] != "P3" {
		println!("error: unrecongized format: {}", arr[0]);
		return Ok(Texture::missing(1, 1, 1));
	}
	println!("done!");
	let data: Vec<usize> = arr.iter().skip(1).map(|num| num.parse::<usize>().unwrap()).collect();
	let (width, height) = (data[0], data[1]);
	let (mut pix_row, mut pix_buf) = (Vec::new(), Vec::new());
	
	for h in 0..height {
		for w in (0..width*3).step_by(3) {
			let R = data[w+3 + width*h*3];
			let G = data[w+4 + width*h*3];
			let B = data[w+5 + width*h*3];
			pix_row.push(Color::RGB((R as f32)/255.0, (G as f32)/255.0, (B as f32)/255.0));
		}
		pix_buf.push(pix_row.clone());
		pix_row.clear();
	}
	println!("image imported successfully!\n");
	Ok(Texture::new(width, height, pix_buf))
}

pub fn write_bitmap(filename: String, tex: Texture) -> std::io::Result<()> {
	println!("exporting image: {filename}.ppm");
	let mut header = format!("P3 {} {} 255\n", tex.width, tex.height);
	let mut file = File::create(format!("{filename}.ppm"))?;
	
	let mut color_data = String::new();
	for h in 0..tex.height {
		for w in 0..tex.width {
			let pixel = tex.bitmap[h][w].RGB;
			let string = format!("{} {} {} ", (pixel[0]*255.0) as usize, (pixel[1]*255.0) as usize, (pixel[2]*255.0) as usize);
			color_data.push_str(&string);
		}
		color_data.push_str("\n");
	}
	header.push_str(&color_data);
	file.write_all(header.as_bytes())?;
	
	println!("image exported successfuly!\n");
	Ok(())
}

fn load_material(filename: String) -> std::io::Result<(Material, Texture)> {
	println!("importing material: {filename}");
	let mut mtl = File::open(format!("{}", filename))?;
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
	
	for component in string_components {
		match component.0 {
			"ambient" => {
				let RGB: Vec<&str> = component.1.split(" ").collect();
				material.ambient = Color::RGB(RGB[0].parse::<f32>().unwrap(), RGB[1].parse::<f32>().unwrap(), RGB[2].parse::<f32>().unwrap());
			},
			"diffuse" => {
				let RGB: Vec<&str> = component.1.split(" ").collect();
				material.diffuse = Color::RGB(RGB[0].parse::<f32>().unwrap(), RGB[1].parse::<f32>().unwrap(), RGB[2].parse::<f32>().unwrap());
			},
			"specular" => {
				let RGB: Vec<&str> = component.1.split(" ").collect();
				material.specular = Color::RGB(RGB[0].parse::<f32>().unwrap(), RGB[1].parse::<f32>().unwrap(), RGB[2].parse::<f32>().unwrap());
			},
			"highlights" => { material.highlights = component.1.parse::<f32>().unwrap(); },
			"opacity" => { material.opacity = component.1.parse::<f32>().unwrap(); },
			"texture" => { texture = load_bitmap(component.1)?; },
			"header" => (),
			other => {
				println!("error: unrecognized component: {other}");
				return Ok((Material::missing(), Texture::missing(10, 10, 1)));
			}
	}}
	println!("material imported successfully!");
	Ok((material, texture))
}


pub fn load_object(filename: String) -> std::io::Result<Mesh> {
	println!("importing object: {filename}.obj");
	let mut obj = File::open(format!("{filename}.obj"))?;
	let mut obj_data = String::new();
	obj.read_to_string(&mut obj_data)?;
	
	let match_mtl_filename = Regex::new("mtllib (?<mtlfile>[a-zA-Z0-9_-]+.mtl)").unwrap();
	
	let num = r"-?[0-9]+.[0-9]+e?(\+|-)?[0-9]*";
	let match_geometry_vertex = Regex::new(&format!("v (?<x>{num}) (?<y>{num}) (?<z>{num})")).unwrap();
	let match_vertex_normal = Regex::new(&format!("vn (?<x>{num}) (?<y>{num}) (?<z>{num})")).unwrap();
	let match_texcoord = Regex::new(&format!("vt (?<u>{num}) (?<v>{num})")).unwrap();
	
	let match_face = Regex::new("f (?<v1>[0-9]+/?[0-9]*/?[0-9]*) (?<v2>[0-9]+/?[0-9]*/?[0-9]*) (?<v3>[0-9]+/?[0-9]*/?[0-9]*)").unwrap();

	print!("detecting material file... ");
	let mtl_filename: Option<String> = match match_mtl_filename.captures(&obj_data) {
		Some(capture) => {
			let capture_string = capture["mtlfile"].to_string();
			println!("{capture_string}");
			Some(capture_string)
		},
		None => {
			println!("no material file");
			None
	}};
	
	print!("detecting triangle data format... ");
	let first_face = match match_face.captures(&obj_data) {
		Some(capture) => [capture["v1"].to_string(), capture["v2"].to_string(), capture["v3"].to_string()],
		None => {
			println!("error: unable to recognize mesh data!\n");
			return Ok(Mesh::empty());
	}};
	
	let face_data: Vec<&str> = first_face[0].split("/").collect();
	let tex_coords_included = if face_data.len() >= 2 { face_data[1].len() != 0 }else { false };
	let normals_included = if face_data.len() == 3 { face_data[2].len() != 0 }else { false };
	
	println!("normals: {normals_included}, texture coordinates: {tex_coords_included}");
	
	let mut vertices: Vec<Vector3D> = Vec::new();
	let mut vertex_normals: Vec<Vector3D> = Vec::new();
	let mut tex_coords: Vec<Point2D> = Vec::new();
	let mut triangles: Vec<Triangle> = Vec::new();
	let mut tex_tris: Vec<Triangle> = Vec::new();
	
	// vertex normals need to have the  came id as vertices so they're properly loaded into the mesh struct
	let mut sorted_vertex_normals: Vec<Vector3D> = Vec::new();
	
	print!("reading vertex data... ");
	for v in match_geometry_vertex.captures_iter(&obj_data) {
		let capture = [v["x"].to_string(), v["y"].to_string(), v["z"].to_string()];
		let vector = Vector3D::XYZ(
			capture[0].parse::<f32>().unwrap(),
			capture[1].parse::<f32>().unwrap(),
			capture[2].parse::<f32>().unwrap()
		);
		vertices.push(vector);
		sorted_vertex_normals.push(Vector3D::zero());
		
	}
	println!("done!");
	
	if normals_included {
		print!("reading vertex normal data... ");
		for v in match_vertex_normal.captures_iter(&obj_data) {
			let capture = [v["x"].to_string(), v["y"].to_string(), v["z"].to_string()];
			let vector = Vector3D::XYZ(
				capture[0].parse::<f32>().unwrap(),
				capture[1].parse::<f32>().unwrap(),
				capture[2].parse::<f32>().unwrap()
			);
			vertex_normals.push(vector);
		}
		println!("done!");
	}else {
		vertex_normals.push(Vector3D::zero());
	}
	
	if tex_coords_included {
		print!("reading texture coordinate data... ");
		for v in match_texcoord.captures_iter(&obj_data) {
			let capture = [v["u"].to_string(), v["v"].to_string()];
			let vector = [
				capture[0].parse::<f32>().unwrap(),
				capture[1].parse::<f32>().unwrap()
			];
			tex_coords.push(vector);
		}
		println!("done!");
	}else {
		tex_coords.push([0.0; 2]);
	}
	
	print!("reading triangle data... ");
	for t in match_face.captures_iter(&obj_data) {
		let captured_verts = [t["v1"].to_string(), t["v2"].to_string(), t["v3"].to_string()];
		let mut triangle_data = Vec::new();
		
		for c in captured_verts.iter() {
			let data: Vec<&str> = c.split("/").collect();
			
			let has_tex_coord = if data.len() >= 2 { data[1].len() != 0 }else { false };
			let has_normal = if data.len() == 3 { data[2].len() != 0 }else { false };

			if (has_tex_coord ^ tex_coords_included) || (has_normal ^ normals_included) { // xor is true if values don't match
				println!("error: triangle has incorrect data, all triangles must have the name parameters\n");
				return Ok(Mesh::empty());
			}

			let mut attributes = [1, 1];
			
			let vertex_id = data[0].parse::<usize>().unwrap();
			if vertex_id > vertices.len() {
				println!("error: vertex index is {} but there are {} vertices\n",vertex_id, vertices.len());
				return Ok(Mesh::empty());
			}else {
				attributes[0] = vertex_id;
			}
			
			if tex_coords_included {
				let uv_id = data[1].parse::<usize>().unwrap();
				if uv_id > tex_coords.len() {
					println!("error: texture coordinate index is {} but there are {} texture coordinates\n", uv_id, tex_coords.len());
					return Ok(Mesh::empty());
				}else {
					attributes[1] = uv_id;
			}}
			if normals_included {
				let norm_id = data[2].parse::<usize>().unwrap();
				if norm_id > vertex_normals.len() {
					println!("error: vertex normal index is {} but there are {} vertex normals\n", norm_id, vertex_normals.len());
					return Ok(Mesh::empty());
				}else {
					sorted_vertex_normals[attributes[0]-1] = vertex_normals[norm_id-1];
			}}
			triangle_data.push(attributes);
		}
		triangles.push([triangle_data[0][0]-1, triangle_data[1][0]-1, triangle_data[2][0]-1]);
		tex_tris.push([triangle_data[0][1]-1, triangle_data[1][1]-1, triangle_data[2][1]-1]);
	}
	println!("done!");
	
	let (mut material, mut texture) = (Material::missing(), Texture::missing(1, 1, 1));
	if mtl_filename.is_some() {
		(material, texture) = load_material(mtl_filename.unwrap())?;
	}
	
	let mut unset_face_normals = Vec::new();
	for t in 0..triangles.len() { unset_face_normals.push(Vector3D::zero()); }
	
	println!("object imported successfully!\n");
	Ok(Mesh{
		vertices,
		triangles,
		tex_coords,
		tex_tris,
		face_normals: unset_face_normals,
		vertex_normals: sorted_vertex_normals,
		origin: Vector3D::zero(),
		texture,
		material,
		cull_backfaces: true
	})
}


pub fn write_object(filename: String, mesh: Mesh, write_mtl: bool) -> std::io::Result<()> {
	println!("exporting object: {filename}.obj");
	let mut obj = File::create(format!("{filename}.obj"))?;
	let mut obj_content = String::new();
	
	print!("writing vertex data... ");
	for v in mesh.vertices.iter() {
		let string = format!("v {:.6} {:.6} {:.6}\n", v.X, v.Y, v.Z);
		obj_content.push_str(&string);
	}
	println!("done!");
	print!("writing vertex normal data... ");
	for v in mesh.vertex_normals.iter() {
		let string = format!("vn {:.6} {:.6} {:.6}\n", v.X, v.Y, v.Z);
		obj_content.push_str(&string);
	}
	println!("done!");
	if !write_mtl {
		println!("skipping material file");
		print!("writing triangle data... ");
		for t in 0..mesh.triangles.len() {
			let p1 = [mesh.triangles[t][0], mesh.triangles[t][0]];
			let p2 = [mesh.triangles[t][1], mesh.triangles[t][1]];
			let p3 = [mesh.triangles[t][2], mesh.triangles[t][2]];
			
			let string = format!("f {}//{} {}//{} {}//{}\n",
				p1[0]+1, p1[1]+1,
				p2[0]+1, p2[1]+1,
				p3[0]+1, p3[1]+1
			);
			obj_content.push_str(&string);
		}
		println!("done!");
		obj.write_all(obj_content.as_bytes())?;
		
		println!("object exported successfully!");
		return Ok(());
	}

	print!("writing texure coordinate data... ");
	for v in mesh.tex_coords.iter() {
		let string = format!("vt {:.6} {:.6}\n", v[0], v[1]);
		obj_content.push_str(&string);
	}
	println!("done!");
	print!("writing triangle data... ");
	for t in 0..mesh.triangles.len() {
		let p1 = [mesh.triangles[t][0], mesh.tex_tris[t][0], mesh.triangles[t][0]];
		let p2 = [mesh.triangles[t][1], mesh.tex_tris[t][1], mesh.triangles[t][1]];
		let p3 = [mesh.triangles[t][2], mesh.tex_tris[t][2], mesh.triangles[t][2]];
		
		let string = format!("f {}/{}/{} {}/{}/{} {}/{}/{}\n",
			p1[0]+1, p1[1]+1, p1[2]+1,
			p2[0]+1, p2[1]+1, p2[2]+1,
			p3[0]+1, p3[1]+1, p3[2]+1
		);
		obj_content.push_str(&string);
	}
	println!("done!");
	
	println!("exporting material file: {filename}.mtl");
	let mut mtl = File::create(format!("{filename}_material.mtl"))?;
	let mtl_header = format!("newmtl {filename}_material\n");
	let mut mtl_content = String::new();
	
	let obj_header = format!("mtllib {filename}_material.mtl\n");
	obj_content = obj_header + &obj_content;
	
	let ka = mesh.material.ambient.RGB;
	let kd = mesh.material.diffuse.RGB;
	let ks = mesh.material.specular.RGB;
	let ns = mesh.material.highlights;
	let d = mesh.material.opacity;
	
	let ka_str = format!("Ka {:.2} {:.2} {:.2}\n", ka[0], ka[1], ka[2]);
	let kd_str = format!("Kd {:.2} {:.2} {:.2}\n", kd[0], kd[1], kd[2]);
	let ks_str = format!("Ks {:.2} {:.2} {:.2}\n", ks[0], ks[1], ks[2]);
	let other_args = format!("Ns {ns}\nd {d}\nmap_Kd {filename}_texture.ppm\n");
	
	mtl_content.push_str(&mtl_header);
	mtl_content.push_str(&ka_str);
	mtl_content.push_str(&kd_str);
	mtl_content.push_str(&ks_str);
	mtl_content.push_str(&other_args);
	
	write_bitmap(format!("{filename}_texture"), mesh.texture)?;
	
	mtl.write_all(mtl_content.as_bytes())?;
	println!("material exported successfully!");
	obj.write_all(obj_content.as_bytes())?;
	println!("object exported successfully!");
	Ok(())
}

