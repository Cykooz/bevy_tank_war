use angular_units::Deg;
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, BoxedFuture, LoadContext};
use bevy::prelude::*;
use prisma::{FromColor, Hsv, Rgb};

#[derive(Default)]
pub struct ImageWithHueAssetLoader;

impl AssetLoader for ImageWithHueAssetLoader {
    type Asset = Image;
    type Settings = ();
    type Error = String;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let path_buf = load_context.path().to_path_buf();
            let file_name = path_buf.file_name().unwrap().to_string_lossy();
            let hue_offset: u16 = file_name
                .strip_suffix(".hue_offset")
                .unwrap_or("0")
                .parse()
                .unwrap_or_default();

            if let Some(parent_path) = path_buf.parent() {
                let parent_path = parent_path.to_path_buf();
                let image_asset = load_context
                    .load_direct(parent_path)
                    .await
                    .map_err(|e| e.error.to_string())?;
                if image_asset.asset_type_name() == "Image" {
                    if let Some(image) = image_asset.get::<Image>() {
                        let mut new_image = image.clone();
                        for pixel in new_image.data.chunks_exact_mut(4) {
                            let mut rgb = Rgb::new(
                                pixel[0] as f32 / 255.,
                                pixel[1] as f32 / 255.,
                                pixel[2] as f32 / 255.,
                            );
                            let mut hsv: Hsv<f32, Deg<f32>> = Hsv::from_color(&rgb);
                            hsv.set_hue(hsv.hue() + Deg(hue_offset as f32));
                            rgb = Rgb::from_color(&hsv);
                            pixel[0] = (rgb.red() * 255.0).min(255.).round() as u8;
                            pixel[1] = (rgb.green() * 255.0).min(255.).round() as u8;
                            pixel[2] = (rgb.blue() * 255.0).min(255.).round() as u8;
                        }
                        return Ok(new_image);
                    }
                }
            }

            Err("Error during load image with Hue Offset".to_string())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["hue_offset"]
    }
}
