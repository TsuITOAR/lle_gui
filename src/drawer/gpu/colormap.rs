use colorous::Gradient;

pub const COLORMAP: Gradient = colorous::VIRIDIS;

use eframe::wgpu::TextureFormat;
pub fn get_colormap<const L: usize>(c: Gradient, format: TextureFormat) -> [[u8; 4]; L] {
    let mut colormap = [[0; 4]; L];
    match format {
        TextureFormat::Rgba8Unorm => {
            for (i, color) in (0..L).map(|l| c.eval_rational(l, L - 1)).enumerate() {
                colormap[i] = [color.r, color.g, color.b, 255];
            }
            colormap
        }
        TextureFormat::Bgra8Unorm => {
            for (i, color) in (0..L).map(|l| c.eval_rational(l, L - 1)).enumerate() {
                colormap[i] = [color.b, color.g, color.r, 255];
            }
            colormap
        }
        _ => unimplemented!(),
    }
}
