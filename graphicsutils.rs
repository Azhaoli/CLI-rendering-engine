
use crate::{ Color, Vector3D };
use crate::viewport::Viewport;
use crate::mesh::Texture;

use std::f32::consts::PI;
use rand::Rng;

pub fn ambient_occlusion(viewport: &mut Viewport, num_samples: usize, num_noise: usize, radius: f32, bias: f32) {
	let mut samples = Vec::new();
	let mut noise_tex = Vec::new();
	for s in 0..num_samples {
		let sample = Vector3D::XYZ(
			rand::thread_rng().gen_range(-1.0..1.0),
			rand::thread_rng().gen_range(-1.0..1.0),
			rand::thread_rng().gen_range(0.0..1.0)
		);
		let pos = (s as f32) / (num_samples as f32);
		let scale = 0.1 + 0.9*pos*pos;
		samples.push(sample.normalize().mul(rand::thread_rng().gen_range(0.0..1.0)).mul(scale));
	}
	for n in 0..num_noise {
		let noise = Vector3D::XYZ(
			rand::thread_rng().gen_range(-1.0..1.0),
			rand::thread_rng().gen_range(-1.0..1.0),
			0.0
		);
		noise_tex.push(noise.normalize());
	}
	
	for h in 0..viewport.height {
		for w in 0..viewport.width {
			let normal = viewport.data_buffer[h][w].normal.normalize();
			let depth = viewport.data_buffer[h][w].inv_depth;
			let position = viewport.data_buffer[h][w].worldXYZ.div(depth);
			
			let random = noise_tex[(w + 13*h) % num_noise];
			let tangent = random.sub(normal.mul(random.dot(normal))).normalize();
			let binormal = normal.cross(tangent).normalize();
			
			let mut occlusion = num_samples as f32;
			for s in 0..num_samples {
				let transformed_sample = Vector3D::XYZ(
					samples[s].X*tangent.X + samples[s].Y*binormal.X + samples[s].Z*normal.X,
					samples[s].X*tangent.Y + samples[s].Y*binormal.Y + samples[s].Z*normal.Y,
					samples[s].X*tangent.Z + samples[s].Y*binormal.Z + samples[s].Z*normal.Z
				);
				let sample_position = position.add(transformed_sample.mul(radius));
				let sample_screenXY = viewport.project(sample_position);
				
				if (sample_screenXY[0] < 0.0) || (sample_screenXY[0] >= viewport.width as f32) { continue; }
				if (sample_screenXY[1] < 0.0) || (sample_screenXY[1] >= viewport.height as f32) { continue; }
				let sample_depth = viewport.data_buffer[sample_screenXY[1] as usize][sample_screenXY[0] as usize].inv_depth;
				
				let mut occluded = if sample_depth + bias <= depth { 0.0 }else { 1.0 };
				let val = radius/(depth-sample_depth);
				let intensity = if val < 0.0 { 0.0 } else if val > 1.0 { 1.0 }else { 3.0*val*val - 2.0*val*val*val }; // smoothstep function
				occluded *= intensity;
				occlusion -= occluded;
			}
			occlusion /= num_samples as f32;
			
			viewport.pixel_buffer[h][w] = viewport.pixel_buffer[h][w].mul(occlusion);

	}}
}

pub fn gaussian_blur(tex: &mut Texture, radius: f32) {
	let mut weights = Vec::new();

	for x in 0..(2.0*radius) as usize {
		let X = (x as f32) - radius;
		weights.push(((0.0-X*X)/(2.0*radius*radius)).exp() / (2.0*PI*radius*radius).sqrt());
	}
	let total = weights.iter().fold(0.0, |total, x| x+total);
	weights = weights.iter().map(|x| x/total).collect();

	// take advantage of the gaussian's scale symmetry to decompose the 2d loop over the convolution matrix :3
	let mut blurred_horiz = Texture::missing(tex.width, tex.height, 20);
	let mut blurred = Texture::missing(tex.width, tex.height, 20);
	// blur horizontally
	for y in 0..tex.height {
		for x in 0..tex.width {
			let mut color = Color::RGB(0.0, 0.0, 0.0);
			for r in 0..(2.0*radius) as usize {
				if (x+r < radius as usize) || (x+r >= tex.width+radius as usize) { continue; }
				color = color.add(tex.bitmap[y][x+r-radius as usize].mul(weights[r]));
			}
			blurred_horiz.bitmap[y][x] = color;
	}}
	// blur vertically
	for y in 0..tex.height {
		for x in 0..tex.width {
			let mut color = Color::RGB(0.0, 0.0, 0.0);
			for r in 0..(2.0*radius) as usize {
				if (y+r < radius as usize) || (y+r >= tex.height+radius as usize) { continue; }
				color = color.add(blurred_horiz.bitmap[y+r-radius as usize][x].mul(weights[r]));
			}
			blurred.bitmap[y][x] = color;
	}}
	tex.bitmap = blurred.bitmap;
}
	
