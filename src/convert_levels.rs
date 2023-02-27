// use cat_snake::level::{
//     level_instance::EntityType,
//     level_template::{DefaultModel, EntityTemplate, LevelTemplate, LevelTemplateV2, Model},
// };
// use ron::ser::PrettyConfig;
// use std::{
//     fs::{self, File},
//     io::Write,
// };

fn main() {}

// fn main() {
//     let paths = fs::read_dir("./assets/levels/").unwrap();

//     for path in paths {
//         let path = path.unwrap().path();
//         let res = {
//             let this = path.extension();
//             match this {
//                 None => false,
//                 Some(x) => x == "lvlv2",
//             }
//         };
//         if res {
//             continue;
//         }

//         println!("Converting level {}...", path.clone().display());
//         let bytes = std::fs::read(path.clone()).unwrap();
//         let level_template: LevelTemplate = ron::de::from_bytes(&bytes).unwrap();

//         let mut entities = Vec::new();

//         for wall in &level_template.walls {
//             entities.push(EntityTemplate {
//                 entity_type: EntityType::Wall,
//                 model: Model::Default(DefaultModel::Wall),
//                 grid_position: *wall,
//             });
//         }

//         for food in &level_template.foods {
//             entities.push(EntityTemplate {
//                 entity_type: EntityType::Food,
//                 model: Model::Default(DefaultModel::Food),
//                 grid_position: *food,
//             });
//         }

//         for food in &level_template.spikes {
//             entities.push(EntityTemplate {
//                 entity_type: EntityType::Spike,
//                 model: Model::Default(DefaultModel::Spike),
//                 grid_position: *food,
//             });
//         }

//         for food in &level_template.boxes {
//             entities.push(EntityTemplate {
//                 entity_type: EntityType::Box,
//                 model: Model::Default(DefaultModel::Box),
//                 grid_position: *food,
//             });
//         }

//         for food in &level_template.triggers {
//             entities.push(EntityTemplate {
//                 entity_type: EntityType::Trigger,
//                 model: Model::Default(DefaultModel::Trigger),
//                 grid_position: *food,
//             });
//         }

//         if let Some(food) = level_template.goal {
//             entities.push(EntityTemplate {
//                 entity_type: EntityType::Goal,
//                 model: Model::Default(DefaultModel::Goal),
//                 grid_position: food,
//             });
//         }

//         let template_v2 = LevelTemplateV2 {
//             snakes: level_template.snakes,
//             entities,
//         };

//         let ron_string = ron::ser::to_string_pretty(&template_v2, PrettyConfig::default()).unwrap();
//         let level_asset_path = format!("{}v2", path.display());
//         println!("Saving level v2 {}", level_asset_path);

//         File::create(level_asset_path)
//             .and_then(|mut file| file.write(ron_string.as_bytes()))
//             .expect("Error while writing scene to file");
//     }
// }
