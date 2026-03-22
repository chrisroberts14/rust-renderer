/// This file defines the schema for json files which specify a given scene to render
///
/// We also define the reading and validation
use std::{
    fs, io,
    path::{Path, PathBuf},
};

use schemars::{JsonSchema, Schema};
use serde::Deserialize;
use serde_json::Error as SerdeError;

use crate::{
    geometry::{obj_loader::ObjLoader, object::Object, transform::Transform},
    scenes::{material::Material, pointlight::PointLight, scene::Scene},
};

/// This is a struct representing the whole file
///
/// For the initial implementation there are only lights and objects no skybox defined
///
/// TODO: Add skybox
#[derive(JsonSchema, Deserialize)]
pub struct SceneFile {
    // We limit the scene files to paths to obj files
    pub objects: Vec<ObjectSchema>,
    pub lights: Vec<PointLight>,
}

/// Schema for an individual object.
///
/// Initial implementation only allows for pre-defined obj files to be used in the scene
///
/// TODO: Add some way to create arbritrary shapes
#[derive(JsonSchema, Deserialize)]
pub struct ObjectSchema {
    obj_path: PathBuf,
    transform: Transform,
}

impl SceneFile {
    /// Generate the schema
    pub fn schema() -> Schema {
        schemars::schema_for!(SceneFile)
    }

    /// Create an actual `Scene` object from a file
    pub fn to_scene<P: AsRef<Path>>(path: P) -> Result<Scene, SerdeError> {
        // Read in the data from the file
        let data = fs::read_to_string(path).unwrap();
        let scene: SceneFile = serde_json::from_str(&data)?;

        // Create object vec from the object schema objects read from the JSON file
        // TODO: Add way of reading in materials at the moment the Arc pointer to textures make this
        // inconvenient
        let objs: Vec<Object> = scene
            .objects
            .iter()
            .map(|obj| {
                Object::new(
                    ObjLoader::load(obj.obj_path.clone()),
                    obj.transform.clone(),
                    Material::Color([255, 255, 255, 255]),
                )
            })
            .collect();
        Ok(Scene::new(800.0, 600.0, objs, scene.lights))
    }
}

/// Get all files in the assets/scene_defs directory
pub fn get_all_scene_files() -> io::Result<Vec<PathBuf>> {
    fs::read_dir("assets/scene_defs")?
        .map(|res| res.map(|entry| entry.path()))
        .filter(|res| {
            res.as_ref()
                .map(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
                .unwrap_or(true)
        })
        .collect()
}
