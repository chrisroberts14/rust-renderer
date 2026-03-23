use crate::{
    geometry::{obj_loader::ObjLoader, object::Object, transform::Transform},
    renderer::Renderer,
    scenes::{material::Material, pointlight::PointLight, scene::Scene},
};
use schemars::{JsonSchema, Schema};
use serde::Deserialize;
use std::error::Error;
use std::sync::Arc;
/// This file defines the schema for json files which specify a given scene to render
///
/// We also define the reading and validation
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// This is a struct representing the whole file
///
/// For the initial implementation there are only lights and objects no skybox defined
///
/// TODO: Add skybox
#[derive(JsonSchema, Deserialize)]
pub struct SceneFile {
    // We limit the scene files to paths to obj files
    objects: Vec<ObjectSchema>,
    lights: Vec<PointLight>,
}

/// Schema for an individual object.
///
/// Initial implementation only allows for pre-defined obj files to be used in the scene
///
/// TODO: Add some way to create arbitrary shapes
#[derive(JsonSchema, Deserialize)]
struct ObjectSchema {
    obj_path: PathBuf,
    transform: Transform,
}

impl SceneFile {
    /// Generate the schema
    pub fn schema() -> Schema {
        schemars::schema_for!(SceneFile)
    }

    /// Create an actual `Scene` object from a file
    pub fn from_file<P: AsRef<Path>>(
        path: P,
        window_width: f32,
        window_height: f32,
        renderer: Arc<dyn Renderer>,
    ) -> Result<Scene, Box<dyn Error>> {
        // Read in the data from the file
        let data = fs::read_to_string(path)?;
        let scene: SceneFile = serde_json::from_str(&data)?;

        // Create object vec from the object schema objects read from the JSON file
        // TODO: Add way of reading in materials at the moment the Arc pointer to textures make this
        // inconvenient
        let objs: Vec<Object> = scene
            .objects
            .iter()
            .map(|obj| {
                let obj_from_file = ObjLoader::load(obj.obj_path.clone())?;
                Ok::<Object, Box<dyn Error>>(Object::new(
                    obj_from_file,
                    obj.transform.clone(),
                    Material::Color([255, 255, 255, 255]),
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Scene::new(
            window_width,
            window_height,
            objs,
            scene.lights,
            renderer,
        ))
    }
}

/// Get all files in the assets/scene_defs directory
pub fn get_all_scene_files() -> io::Result<Vec<PathBuf>> {
    fs::read_dir("assets/scene_defs")?
        .map(|res| res.map(|entry| entry.path()))
        .filter(|res| {
            res.as_ref()
                .map(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
                .unwrap_or(false)
        })
        .collect()
}
