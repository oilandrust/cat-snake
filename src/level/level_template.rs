use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};

use serde::{Deserialize, Serialize};

use crate::gameplay::snake_pluggin::SnakeTemplate;

#[derive(Reflect, Resource, Deserialize, Serialize, TypeUuid, Debug, Default)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct LevelTemplate {
    pub snakes: Vec<SnakeTemplate>,
    pub foods: Vec<IVec3>,
    pub walls: Vec<IVec3>,
    pub spikes: Vec<IVec3>,
    pub boxes: Vec<IVec3>,
    pub triggers: Vec<IVec3>,
    pub goal: Option<IVec3>,
}

#[derive(Resource)]
pub struct LoadingLevel(pub Handle<LevelTemplate>);

#[derive(Resource)]
pub struct LoadedLevel(pub Handle<LevelTemplate>);

#[derive(Default)]
pub struct LevelTemplateLoader;

impl AssetLoader for LevelTemplateLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let custom_asset = ron::de::from_bytes::<LevelTemplate>(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(custom_asset));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["lvl"]
    }
}
