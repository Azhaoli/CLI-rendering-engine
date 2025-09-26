use crate::{ Color, Vector3D, Point2D };
use crate::clamp;

#[derive(Clone)]
pub struct Texture {
	pub width: usize,
	pub height: usize,
	pub bitmap: Vec<Vec<Color>>
}

impl Texture {
	pub fn new(width: usize, height: usize, bitmap: Vec<Vec<Color>>) -> Texture { Texture { width, height, bitmap } }
	
	pub fn missing(width: usize, height: usize, size: usize) -> Texture {
		let (mut bit_row1, mut bit_row2, mut bitmap) = (Vec::new(), Vec::new(), Vec::new());
		let c1 = Color::RGB(0.6, 0.6, 0.6);
		let c2 = Color::RGB(0.9, 0.9, 0.9);
		for w in 0..width {
			if (w/size) % 2 == 0 {
				bit_row1.push(c1);
				bit_row2.push(c2);
			}else {
				bit_row1.push(c2);
				bit_row2.push(c1);
		}}
		for h in 0..height {
			if (h/size) % 2 == 0 { bitmap.push(bit_row1.clone()); }else { bitmap.push(bit_row2.clone()); }
		}
		Texture{ width, height, bitmap }
	}
	
	pub fn sample(&self, UV: Point2D) -> Color {
		// clamp U and V
		let u = clamp(0.0, 1.0, UV.0);
		let v = clamp(0.0, 1.0, UV.1);
		let (tx, ty) = (u * (self.width-1) as f32, v * (self.height-1) as f32);
		
		let (u_fac, v_fac) = (tx.fract(), ty.fract());
		let (c0, c1, c2, c3) = (
			self.bitmap[ty.floor() as usize][tx.floor() as usize],
			self.bitmap[ty.floor() as usize][tx.ceil() as usize],
			self.bitmap[ty.ceil() as usize][tx.floor() as usize],
			self.bitmap[ty.ceil() as usize][tx.ceil() as usize]
		);
		let (c01, c23) = (c0.lerp(c1, u_fac), c2.lerp(c3, u_fac));
		c01.lerp(c23, v_fac)
	}
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
pub enum LightingMode {
	Flat,
	Smooth,
	None
}

#[derive(Copy, Clone)]
pub struct LightSource {
	pub color: Color,
	pub position: Vector3D
}

impl LightSource {
	pub fn new(color: Color, position: Vector3D) -> LightSource { LightSource{ color, position } }
	pub fn magenta(position: Vector3D) -> LightSource { LightSource{ color: Color::RGB(1.0, 0.0, 1.0), position } }
}

