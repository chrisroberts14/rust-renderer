use crate::geometry::object::Object;
use crate::maths::vec3::Vec3;

/// How much velocity is retained after a collision. 0 = no bounce, 1 = perfectly elastic.
const RESTITUTION: f32 = 0.4;

/// Run one collision-detection-and-response pass over all objects.
///
/// For every pair of objects whose AABBs overlap, compute the minimum penetration
/// axis and apply a positional correction plus an impulse to separate them.
pub(crate) fn step(objects: &mut [Object]) {
    let n = objects.len();
    for i in 0..n {
        for j in (i + 1)..n {
            if objects[i].is_static && objects[j].is_static {
                continue;
            }

            let (Some((min_i, max_i)), Some((min_j, max_j))) =
                (objects[i].bounding_box(), objects[j].bounding_box())
            else {
                continue;
            };

            let overlap_x = max_i.x.min(max_j.x) - min_i.x.max(min_j.x);
            let overlap_y = max_i.y.min(max_j.y) - min_i.y.max(min_j.y);
            let overlap_z = max_i.z.min(max_j.z) - min_i.z.max(min_j.z);

            if overlap_x <= 0.0 || overlap_y <= 0.0 || overlap_z <= 0.0 {
                continue;
            }

            // Normal points from j toward i; depth is the penetration distance.
            let (depth, normal) = minimum_penetration_axis(
                objects[i].transform.position,
                objects[j].transform.position,
                overlap_x,
                overlap_y,
                overlap_z,
            );

            resolve(objects, i, j, normal, depth);
        }
    }
}

/// Returns the (depth, normal) for the axis with the smallest overlap.
/// The normal always points from j toward i.
fn minimum_penetration_axis(pos_i: Vec3, pos_j: Vec3, ox: f32, oy: f32, oz: f32) -> (f32, Vec3) {
    if ox <= oy && ox <= oz {
        let sign = if pos_i.x >= pos_j.x { 1.0 } else { -1.0 };
        (ox, Vec3::new(sign, 0.0, 0.0))
    } else if oy <= ox && oy <= oz {
        let sign = if pos_i.y >= pos_j.y { 1.0 } else { -1.0 };
        (oy, Vec3::new(0.0, sign, 0.0))
    } else {
        let sign = if pos_i.z >= pos_j.z { 1.0 } else { -1.0 };
        (oz, Vec3::new(0.0, 0.0, sign))
    }
}

/// Apply positional correction and velocity impulse for a detected collision.
/// `normal` points from j toward i; positive depth means objects overlap by that amount.
fn resolve(objects: &mut [Object], i: usize, j: usize, normal: Vec3, depth: f32) {
    match (objects[i].is_static, objects[j].is_static) {
        (true, true) => {}

        // i is static, j is dynamic: push j in the -normal direction.
        (true, false) => {
            objects[j].transform.position = objects[j].transform.position - normal * depth;
            let v = objects[j].velocity;
            let vn = v.dot(normal);
            if vn > 0.0 {
                objects[j].velocity = v - normal * (vn * (1.0 + RESTITUTION));
            }
        }

        // j is static, i is dynamic: push i in the +normal direction.
        (false, true) => {
            objects[i].transform.position = objects[i].transform.position + normal * depth;
            let v = objects[i].velocity;
            let vn = v.dot(normal);
            if vn < 0.0 {
                objects[i].velocity = v - normal * (vn * (1.0 + RESTITUTION));
            }
        }

        // Both dynamic: split correction by mass and exchange impulse.
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
