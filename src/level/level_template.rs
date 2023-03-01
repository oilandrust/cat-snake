use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    utils::BoxedFuture,
};

use serde::{Deserialize, Serialize};

use crate::gameplay::{level_entities::EntityType, snake_plugin::SnakeTemplate};

#[derive(Deserialize, Serialize, Debug)]
pub enum DefaultModel {
    Food,
    Spike,
    Wall,
    Box,
    Trigger,
    Goal,
}

impl From<EntityType> for DefaultModel {
    fn from(value: EntityType) -> Self {
        match value {
            EntityType::Food => DefaultModel::Food,
            EntityType::Spike => DefaultModel::Spike,
            EntityType::Wall => DefaultModel::Wall,
            EntityType::Box => DefaultModel::Box,
            EntityType::Trigger => DefaultModel::Trigger,
            EntityType::Snake => todo!(),
            EntityType::Goal => DefaultModel::Goal,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Model {
    Default(DefaultModel),
    Asset(String),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EntityTemplate {
    pub entity_type: EntityType,
    pub model: Model,
    pub grid_position: IVec3,
}

#[derive(Resource, Deserialize, Serialize, TypeUuid, Debug, Default)]
#[uuid = "39cadc56-aa9c-4543-8640-a018b74b5052"]
pub struct LevelTemplate {
    pub snakes: Vec<SnakeTemplate>,
    pub entities: Vec<EntityTemplate>,
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
