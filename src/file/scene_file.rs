use crate::scenes::lights::spot_light::SpotLight;
use crate::{
    geometry::{obj_loader::ObjLoader, object::Object, plane::Plane, transform::Transform},
    renderer::Renderer,
    scenes::{
        lights::{Light, pointlight::PointLight},
        material::Material,
        scene::Scene,
    },
};
use schemars::{JsonSchema, Schema};
use serde::Deserialize;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
/// This file defines the schema for json files which specify a given scene to render
///
/// We also define the reading and validation
use std::{fs, io, path::Path};

fn default_colour() -> [u8; 4] {
    [255, 255, 255, 255]
}

fn default_subdivisions() -> u32 {
    8
}

/// Schema for an individual object.
///
/// Use `"type": "mesh"` for OBJ files and `"type": "plane"` for a flat quad primitive.
#[derive(JsonSchema, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ObjectSchema {
    Mesh {
        obj_path: PathBuf,
        transform: Transform,
        #[serde(default = "default_colour")]
        colour: [u8; 4],
        update: Option<Transform>,
    },
    Plane {
        size: f32,
        #[serde(default = "default_subdivisions")]
        subdivisions: u32,
        transform: Transform,
        #[serde(default = "default_colour")]
        colour: [u8; 4],
    },
}

impl ObjectSchema {
    fn into_object(self) -> Result<Object, Box<dyn Error>> {
        match self {
            ObjectSchema::Mesh {
                obj_path,
                transform,
                colour,
                update,
            } => {
                let mesh = ObjLoader::load(obj_path)?;
                let object = Object::new(mesh, transform, Material::Color(colour));
                let object = if let Some(upd) = update {
                    object.with_update(move |t| {
                        *t = *t * upd;
                    })
                } else {
                    object
                };
                Ok(object)
            }
            ObjectSchema::Plane {
                size,
                subdivisions,
                transform,
                colour,
            } => Ok(Object::new(
                Plane::mesh(size, subdivisions),
                transform,
                Material::Color(colour),
            )),
        }
    }
}

/// Enum covering every supported light type.
///
/// Each variant is identified in JSON by a `"type"` field, e.g. `"type": "point"`.
#[derive(Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
enum LightSchema {
    Point(PointLight),
    Spot(SpotLight),
}

impl LightSchema {
    fn into_arc_light(self) -> Arc<dyn Light> {
        match self {
            LightSchema::Point(pl) => Arc::new(pl),
            LightSchema::Spot(sp) => Arc::new(sp),
        }
    }
}

/// This is a struct representing the whole file
///
/// For the initial implementation there are only lights and objects no skybox defined
///
/// TODO: Add skybox
#[derive(JsonSchema, Deserialize)]
pub struct SceneFile {
    objects: Vec<ObjectSchema>,
    lights: Vec<LightSchema>,
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
        let data = fs::read_to_string(path)?;
        let scene: SceneFile = serde_json::from_str(&data)?;

        let objs: Vec<Object> = scene
            .objects
            .into_iter()
            .map(|obj| obj.into_object())
            .collect::<Result<Vec<_>, _>>()?;

        let lights: Vec<Arc<dyn Light>> = scene
            .lights
            .into_iter()
            .map(|l| l.into_arc_light())
            .collect();

        Ok(Scene::new(
            window_width,
            window_height,
            objs,
            lights,
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
