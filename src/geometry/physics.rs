use crate::geometry::object::{CollisionShape, Object};
use crate::maths::vec3::Vec3;

/// How much velocity is retained after a collision. 0 = no bounce, 1 = perfectly elastic.
const RESTITUTION: f32 = 0.4;

/// Run one collision-detection-and-response pass over all objects.
pub(crate) fn step(objects: &mut [Object]) {
    let n = objects.len();
    for i in 0..n {
        for j in (i + 1)..n {
            if objects[i].is_static && objects[j].is_static {
                continue;
            }
            if let Some((normal, depth)) = detect(&objects[i], &objects[j]) {
                resolve(objects, i, j, normal, depth);
            }
        }
    }
}

/// Dispatches to the appropriate narrow-phase test based on each object's collision shape.
/// Returns `Some((normal, depth))` where `normal` points from j toward i.
fn detect(a: &Object, b: &Object) -> Option<(Vec3, f32)> {
    match (&a.collision_shape, &b.collision_shape) {
        (CollisionShape::Sphere { radius: ra }, CollisionShape::Sphere { radius: rb }) => {
            sphere_vs_sphere(a.transform.position, *ra, b.transform.position, *rb)
        }
        (CollisionShape::Sphere { radius }, CollisionShape::Aabb) => {
            let (aabb_min, aabb_max) = b.bounding_box()?;
            sphere_vs_aabb(a.transform.position, *radius, aabb_min, aabb_max)
        }
        (CollisionShape::Aabb, CollisionShape::Sphere { radius }) => {
            let (aabb_min, aabb_max) = a.bounding_box()?;
            // Compute sphere (b) vs aabb (a), then flip so normal still points from j (b) toward i (a).
            let (normal, depth) =
                sphere_vs_aabb(b.transform.position, *radius, aabb_min, aabb_max)?;
            Some((-normal, depth))
        }
        (CollisionShape::Aabb, CollisionShape::Aabb) => aabb_vs_aabb(a, b),
    }
}

// ---------------------------------------------------------------------------
// Shape tests
// ---------------------------------------------------------------------------

/// Normal points from b toward a.
fn sphere_vs_sphere(pos_a: Vec3, ra: f32, pos_b: Vec3, rb: f32) -> Option<(Vec3, f32)> {
    let diff = pos_a - pos_b;
    let dist_sq = diff.x * diff.x + diff.y * diff.y + diff.z * diff.z;
    let sum_radii = ra + rb;
    if dist_sq >= sum_radii * sum_radii {
        return None;
    }
    let dist = dist_sq.sqrt();
    let normal = if dist > 0.0 {
        diff / dist
    } else {
        Vec3::new(0.0, 1.0, 0.0) // degenerate: same position, push upward
    };
    Some((normal, sum_radii - dist))
}

/// Normal points from the AABB surface toward the sphere center (i.e. away from the AABB).
fn sphere_vs_aabb(
    sphere_pos: Vec3,
    radius: f32,
    aabb_min: Vec3,
    aabb_max: Vec3,
) -> Option<(Vec3, f32)> {
    let closest = Vec3::new(
        sphere_pos.x.clamp(aabb_min.x, aabb_max.x),
        sphere_pos.y.clamp(aabb_min.y, aabb_max.y),
        sphere_pos.z.clamp(aabb_min.z, aabb_max.z),
    );
    let diff = sphere_pos - closest;
    let dist_sq = diff.x * diff.x + diff.y * diff.y + diff.z * diff.z;
    if dist_sq >= radius * radius {
        return None;
    }
    let dist = dist_sq.sqrt();
    if dist > 0.0 {
        Some((diff / dist, radius - dist))
    } else {
        // Sphere center is inside the AABB — push out through the nearest face.
        Some(nearest_face_normal(sphere_pos, aabb_min, aabb_max))
    }
}

/// Normal points from j toward i.
fn aabb_vs_aabb(a: &Object, b: &Object) -> Option<(Vec3, f32)> {
    let (min_a, max_a) = a.bounding_box()?;
    let (min_b, max_b) = b.bounding_box()?;

    let ox = max_a.x.min(max_b.x) - min_a.x.max(min_b.x);
    let oy = max_a.y.min(max_b.y) - min_a.y.max(min_b.y);
    let oz = max_a.z.min(max_b.z) - min_a.z.max(min_b.z);

    if ox <= 0.0 || oy <= 0.0 || oz <= 0.0 {
        return None;
    }

    Some(min_penetration_axis(
        a.transform.position,
        b.transform.position,
        ox,
        oy,
        oz,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the (normal, depth) for the axis of minimum penetration.
/// Normal points from pos_b toward pos_a.
fn min_penetration_axis(pos_a: Vec3, pos_b: Vec3, ox: f32, oy: f32, oz: f32) -> (Vec3, f32) {
    if ox <= oy && ox <= oz {
        let sign = if pos_a.x >= pos_b.x { 1.0 } else { -1.0 };
        (Vec3::new(sign, 0.0, 0.0), ox)
    } else if oy <= ox && oy <= oz {
        let sign = if pos_a.y >= pos_b.y { 1.0 } else { -1.0 };
        (Vec3::new(0.0, sign, 0.0), oy)
    } else {
        let sign = if pos_a.z >= pos_b.z { 1.0 } else { -1.0 };
        (Vec3::new(0.0, 0.0, sign), oz)
    }
}

/// When a point is inside an AABB, find the face it's closest to and return a
/// normal pointing outward through that face plus the penetration depth.
fn nearest_face_normal(pos: Vec3, aabb_min: Vec3, aabb_max: Vec3) -> (Vec3, f32) {
    let candidates = [
        (pos.x - aabb_min.x, Vec3::new(-1.0, 0.0, 0.0)),
        (aabb_max.x - pos.x, Vec3::new(1.0, 0.0, 0.0)),
        (pos.y - aabb_min.y, Vec3::new(0.0, -1.0, 0.0)),
        (aabb_max.y - pos.y, Vec3::new(0.0, 1.0, 0.0)),
        (pos.z - aabb_min.z, Vec3::new(0.0, 0.0, -1.0)),
        (aabb_max.z - pos.z, Vec3::new(0.0, 0.0, 1.0)),
    ];
    candidates
        .iter()
        .copied()
        .min_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap())
        .map(|(depth, normal)| (normal, depth))
        .unwrap()
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

/// Apply positional correction and velocity impulse.
/// `normal` points from j toward i.
fn resolve(objects: &mut [Object], i: usize, j: usize, normal: Vec3, depth: f32) {
    match (objects[i].is_static, objects[j].is_static) {
        (true, true) => {}

        (true, false) => {
            objects[j].transform.position = objects[j].transform.position - normal * depth;
            let v = objects[j].velocity;
            let vn = v.dot(normal);
            if vn > 0.0 {
                objects[j].velocity = v - normal * (vn * (1.0 + RESTITUTION));
            }
        }

        (false, true) => {
            objects[i].transform.position = objects[i].transform.position + normal * depth;
            let v = objects[i].velocity;
            let vn = v.dot(normal);
            if vn < 0.0 {
                objects[i].velocity = v - normal * (vn * (1.0 + RESTITUTION));
            }
        }

        (false, false) => {
            let mi = objects[i].mass;
            let mj = objects[j].mass;
            let total = mi + mj;
            objects[i].transform.position =
                objects[i].transform.position + normal * (depth * mj / total);
            objects[j].transform.position =
                objects[j].transform.position - normal * (depth * mi / total);

            let vi = objects[i].velocity;
            let vj = objects[j].velocity;
            let vn = (vi - vj).dot(normal);
            if vn < 0.0 {
                let impulse = -(1.0 + RESTITUTION) * vn / (1.0 / mi + 1.0 / mj);
                let imp = normal * impulse;
                objects[i].velocity = vi + imp * (1.0 / mi);
                objects[j].velocity = vj - imp * (1.0 / mj);
            }
        }
    }
}
