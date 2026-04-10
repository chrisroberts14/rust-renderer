use crate::geometry::animation::DeltaAnimation;
use crate::geometry::update_thread::ThreadedUpdate;
use crate::scenes::lights::spot_light::SpotLight;
use crate::scenes::texture::Texture;
use crate::{
    geometry::{
        obj_loader::ObjLoader, object::Object, plane::Plane, sphere::Sphere, transform::Transform,
    },
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
use std::{fs, path::Path};

fn default_colour() -> [u8; 4] {
    [255, 255, 255, 255]
}

fn default_subdivisions() -> u32 {
    8
}

fn default_stacks_slices() -> u32 {
    16
}

/// The default ambient light if none is specified in the file
fn default_ambient_light() -> f32 {
    0.15
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
    Sphere {
        radius: f32,
        #[serde(default = "default_stacks_slices")]
        stacks: u32,
        #[serde(default = "default_stacks_slices")]
        slices: u32,
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
                    object.with_animation(DeltaAnimation {
                        rotation: upd.rotation,
                        position: upd.position,
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
            )
            .as_static()),
            ObjectSchema::Sphere {
                radius,
                stacks,
                slices,
                transform,
                colour,
            } => Ok(Object::new(
                Sphere::mesh(radius, stacks, slices),
                transform,
                Material::Color(colour),
            )
            .with_sphere_collider(radius)),
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

/// Struct representing a skybox
///
/// This can either be a path to a file containing the texture or a solid colour
#[derive(Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SkyboxSchema {
    File {
        path: PathBuf,
    },
    SolidColour {
        #[serde(default = "default_colour")]
        colour: [u8; 4],
    },
}

/// This is a struct representing the whole file
#[derive(JsonSchema, Deserialize)]
pub struct SceneFile {
    objects: Vec<ObjectSchema>,
    lights: Option<Vec<LightSchema>>,
    #[serde(default = "default_ambient_light")]
    ambient: f32,
    skybox: Option<SkyboxSchema>,
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
            .unwrap_or_default()
            .into_iter()
            .map(|l| l.into_arc_light())
            .collect();

        let mut skybox: Option<Texture> = None;

        if let Some(scene_skybox) = scene.skybox {
            skybox = Some(match scene_skybox {
                SkyboxSchema::File { path } => {
                    Texture::load(&path).map_err(|e| Box::new(e) as Box<dyn Error>)?
                }
                SkyboxSchema::SolidColour { colour } => Texture::new(1, 1, colour),
            });
        }

        Ok(Scene::new(
            window_width,
            window_height,
            objs,
            lights,
            scene.ambient,
            ThreadedUpdate,
            skybox,
        ))
    }
}
