// ----------------------------------------------------------------------------
use bevy::{
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::{Image, TextureFormatPixelInfo},
    },
    utils::HashMap,
};
use bevy_egui::EguiContext;
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct UiImages {
    id: u64,
    images: HashMap<String, u64>,
    handles: HashMap<String, Handle<Image>>,
}
// ----------------------------------------------------------------------------
impl UiImages {
    // ------------------------------------------------------------------------
    pub fn set(
        &mut self,
        egui_ctx: &mut EguiContext,
        id: impl Into<String>,
        handle: Handle<Image>,
    ) {
        let id = id.into();
        let egui_id = if let Some(egui_id) = self.images.get(&id) {
            // remove prev texture from egui
            egui_ctx.remove_egui_texture(*egui_id);
            *egui_id
        } else {
            self.id += 1;
            self.images.insert(id, self.id);
            self.id
        };
        egui_ctx.set_egui_texture(egui_id, handle);
    }
    // ------------------------------------------------------------------------
    pub fn add_image(
        &mut self,
        egui_ctx: &mut EguiContext,
        images: &mut Assets<Image>,
        id: impl Into<String>,
        format: TextureFormat,
        size: (u32, u32),
        data: &[u8],
    ) {
        let id = id.into();

        let format = match format {
            TextureFormat::R16Uint => TextureFormat::R16Unorm,
            _ => format,
        };
        // println!("add_image format: {:?}", format);
        let new_img = Image::new(
            Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data.to_vec(),
            format,
        );

        let handle = images.add(new_img);
        self.set(egui_ctx, id.clone(), handle.clone_weak());
        // adding image means it's only for UI usage -> hold strong reference
        // also removes old handle (if anything with same id was used)!
        self.handles.insert(id, handle);
    }
    // ------------------------------------------------------------------------
    pub fn update_image(&mut self, images: &mut Assets<Image>, id: &str, data: &[u8]) {
        if let Some(handle) = self.handles.get(id) {
            if let Some(img) = images.get_mut(handle) {
                let s = &img.texture_descriptor.size;
                let expected_size =
                    (s.width * s.height) as usize * img.texture_descriptor.format.pixel_size();

                assert!(data.len() == expected_size);
                img.data.copy_from_slice(data);
            }
        }
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    pub fn remove(&mut self, egui_ctx: &mut EguiContext, id: &str) {
        if let Some(imageid) = self.images.remove(id) {
            egui_ctx.remove_egui_texture(imageid);
        }
        self.handles.remove(id);
    }
    // ------------------------------------------------------------------------
    pub fn get_imageid(&self, id: &str) -> bevy_egui::egui::TextureId {
        use bevy_egui::egui::TextureId;

        TextureId::User(self.images.get(id).cloned().unwrap_or_default())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
