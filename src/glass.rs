//! GPU描画に渡す座標と定数。画像はLua側のGPUバッファに保持する。
use aviutl2::module::FromScriptModuleParam;
type V3 = [f64; 3];
#[derive(Clone, FromScriptModuleParam)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub ox: f64,
    pub oy: f64,
    pub oz: f64,
    pub cx: f64,
    pub cy: f64,
    pub cz: f64,
    pub rx: f64,
    pub ry: f64,
    pub rz: f64,
    pub sx: f64,
    pub sy: f64,
    pub sz: f64,
    pub base_sx: f64,
    pub base_sy: f64,
    pub base_sz: f64,
    pub billboard: i32,
}

#[derive(Clone, FromScriptModuleParam)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub tx: f64,
    pub ty: f64,
    pub tz: f64,
    pub ux: f64,
    pub uy: f64,
    pub uz: f64,
    pub rz: f64,
    pub d: f64,
    pub mode: i32,
}

fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] * b[i])
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(a: V3) -> anyhow::Result<V3> {
    let length = a[0].hypot(a[1]).hypot(a[2]);
    anyhow::ensure!(
        length.is_finite() && length > 0.0,
        "Degenerate glass geometry"
    );
    Ok(a.map(|v| v / length))
}

// AviUtlのオイラー角はZ→Y→X。角度はホストから度数法で受け取る。
pub(crate) fn rotate(p: V3, angles: V3) -> V3 {
    let [x, y, z] = angles.map(f64::to_radians);
    let (sx, cx) = x.sin_cos();
    let (sy, cy) = y.sin_cos();
    let (sz, cz) = z.sin_cos();
    let [a, b, c] = [cz * p[0] - sz * p[1], sz * p[0] + cz * p[1], p[2]];
    let [a, b, c] = [cy * a + sy * c, b, -sy * a + cy * c];
    [a, cx * b - sx * c, sx * b + cx * c]
}

pub(crate) struct View {
    eye: V3,
    right: V3,
    up: V3,
    forward: V3,
    roll: f64,
    focal: f64,
    mode: i32,
}

impl Camera {
    pub(crate) fn view(&self) -> anyhow::Result<View> {
        let eye = [self.x, self.y, self.z];
        let forward = unit(sub([self.tx, self.ty, self.tz], eye))?;
        let right = unit(cross(forward, [self.ux, self.uy, self.uz]))?;
        let up = unit(cross(right, forward))?;
        anyhow::ensure!(
            self.d.is_finite() && self.d > 0.0,
            "Invalid glass camera distance"
        );
        Ok(View {
            eye,
            right,
            up,
            forward,
            roll: self.rz.to_radians(),
            focal: self.d,
            mode: self.mode,
        })
    }
}

// 各グループは position, center, rotation, scale の12要素。直前→上位の順。
pub(crate) fn group_point(mut p: V3, groups: &[f64]) -> V3 {
    for g in groups.chunks_exact(12) {
        p = add(
            rotate(
                scale(sub(p, [g[3], g[4], g[5]]), [g[9], g[10], g[11]]),
                [g[6], g[7], g[8]],
            ),
            [g[0], g[1], g[2]],
        );
    }
    p
}

fn billboard(p: V3, mode: i32, view: &View) -> V3 {
    let yaw = view.forward[0].atan2(view.forward[2]).to_degrees();
    let pitch = (-view.forward[1])
        .atan2(view.forward[0].hypot(view.forward[2]))
        .to_degrees();
    match mode {
        1 => rotate(p, [0., yaw, 0.]),
        2 => rotate(p, [pitch, 0., 0.]),
        3 => {
            let (s, c) = view.roll.sin_cos();
            let x = p[0] * c + p[1] * s;
            let y = -p[0] * s + p[1] * c;
            add(
                add(scale(view.right, [x; 3]), scale(view.up, [-y; 3])),
                scale(view.forward, [p[2]; 3]),
            )
        }
        _ => p,
    }
}

pub(crate) fn world_quad(
    width: usize,
    height: usize,
    pose: &Pose,
    args: &[f64],
    groups: &[f64],
    view: &View,
) -> anyhow::Result<[V3; 4]> {
    anyhow::ensure!(
        matches!(args.len(), 0..=8 | 12 | 20 | 21),
        "Invalid glassdraw argument count"
    );
    anyhow::ensure!(groups.len().is_multiple_of(12), "Invalid glass group data");
    anyhow::ensure!(
        args.iter().chain(groups).all(|v| v.is_finite()),
        "Non-finite glass geometry"
    );
    let polygon = args.len() > 8;
    let mut draw = [0., 0., 0., 1., 1., 0., 0., 0.];
    if !polygon {
        draw[..args.len()].copy_from_slice(args);
    }
    let zoom = if polygon { 1. } else { draw[3] };
    let local_scale = [pose.sx * zoom, pose.sy * zoom, pose.sz * zoom];
    let full_scale = scale(local_scale, [pose.base_sx, pose.base_sy, pose.base_sz]);
    let center = scale([pose.cx, pose.cy, pose.cz], full_scale);
    let mut corners = if polygon {
        std::array::from_fn(|i| scale([args[i * 3], args[i * 3 + 1], args[i * 3 + 2]], local_scale))
    } else {
        let w = width as f64 * 0.5;
        let h = height as f64 * 0.5;
        [[-w, -h, 0.], [w, -h, 0.], [w, h, 0.], [-w, h, 0.]].map(|p| scale(p, full_scale))
    };
    let translation = add(
        [pose.x + pose.ox, pose.y + pose.oy, pose.z + pose.oz],
        if polygon {
            [0.; 3]
        } else {
            [draw[0], draw[1], draw[2]]
        },
    );
    let angles = [pose.rx + draw[5], pose.ry + draw[6], pose.rz + draw[7]];
    for p in &mut corners {
        let mut offset = rotate(sub(*p, center), angles);
        if view.mode != 0 && pose.billboard != 0 {
            // 位置はグループで変換し、面の向きはカメラで決める。
            // 回転の逆行列を先に掛けると、軸別拡大率が異なる場合に面が歪む。
            for g in groups.chunks_exact(12) {
                offset = scale(offset, [g[9], g[10], g[11]]);
            }
            *p = add(
                group_point(translation, groups),
                billboard(offset, pose.billboard, view),
            );
        } else {
            *p = group_point(add(offset, translation), groups);
        }
    }
    anyhow::ensure!(
        corners.iter().flatten().all(|v| v.is_finite()),
        "Non-finite glass vertices"
    );
    Ok(corners)
}

pub(crate) fn surface(corners: [V3; 4]) -> anyhow::Result<(V3, V3)> {
    let center = corners.into_iter().fold([0.; 3], add).map(|v| v * 0.25);
    // 頂点が時計回りの面は-Z向き。三角形（重複頂点）も扱う。
    let mut normal = cross(sub(corners[2], corners[0]), sub(corners[1], corners[0]));
    if dot(normal, normal) == 0.0 {
        normal = cross(sub(corners[3], corners[0]), sub(corners[2], corners[0]));
    }
    let normal = unit(normal)?;
    Ok((center, normal))
}

/// HLSLのfloat4単位。shader/geometry.hlslと同じ順序にする。
pub fn constants(
    width: usize,
    height: usize,
    pose: Pose,
    camera: Camera,
    args: Vec<f64>,
    groups: Vec<f64>,
) -> anyhow::Result<Vec<f64>> {
    anyhow::ensure!(width > 0 && height > 0, "Invalid drawing dimensions");
    let view = camera.view()?;
    let quad = world_quad(width, height, &pose, &args, &groups, &view)?;
    let (center, normal) = surface(quad)?;
    let facing = dot(
        normal,
        if view.mode == 0 {
            view.forward
        } else {
            sub(center, view.eye)
        },
    );
    let mut output = Vec::with_capacity(44);
    for point in quad {
        output.extend([point[0], point[1], point[2], 0.]);
    }
    output.extend([normal[0], normal[1], normal[2], facing]);
    for vector in [view.eye, view.right, view.up, view.forward] {
        output.extend([vector[0], vector[1], vector[2], 0.]);
    }
    output.extend([
        view.focal,
        view.roll.cos(),
        view.roll.sin(),
        f64::from(view.mode),
    ]);
    let mut bounds = [0., 0., width as f64, height as f64];
    if args.len() >= 20 {
        for axis in 0..2 {
            let min = args[12 + axis..20]
                .iter()
                .step_by(2)
                .copied()
                .fold(f64::INFINITY, f64::min);
            let max = args[12 + axis..20]
                .iter()
                .step_by(2)
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            let size = [width, height][axis] as f64;
            bounds[axis] = (min - 2.).clamp(0., size).ceil();
            bounds[axis + 2] = (max + 2.).clamp(0., size).ceil();
        }
    }
    output.extend(bounds);
    anyhow::ensure!(
        output.iter().all(|v| (*v as f32).is_finite()),
        "Drawing constants exceed GPU float range"
    );
    Ok(output)
}

pub fn layer_position(position: Vec<f64>, groups: Vec<f64>) -> anyhow::Result<Vec<f64>> {
    let position: V3 = position
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid light position"))?;
    anyhow::ensure!(groups.len().is_multiple_of(12), "Invalid light groups");
    Ok(group_point(position, &groups).to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pose() -> Pose {
        Pose {
            x: 0.,
            y: 0.,
            z: 0.,
            ox: 0.,
            oy: 0.,
            oz: 0.,
            cx: 0.,
            cy: 0.,
            cz: 0.,
            rx: 0.,
            ry: 0.,
            rz: 0.,
            sx: 1.,
            sy: 1.,
            sz: 1.,
            base_sx: 1.,
            base_sy: 1.,
            base_sz: 1.,
            billboard: 0,
        }
    }

    fn camera() -> Camera {
        Camera {
            x: 0.,
            y: 0.,
            z: -1024.,
            tx: 0.,
            ty: 0.,
            tz: 0.,
            ux: 0.,
            uy: -1.,
            uz: 0.,
            rz: 0.,
            d: 1024.,
            mode: 0,
        }
    }

    fn close(a: &[f64], b: &[f64]) {
        for (a, b) in a.iter().zip(b) {
            assert!((a - b).abs() < 1e-9, "{a} != {b}");
        }
    }

    #[test]
    fn draw_and_drawpoly_agree_and_apply_object_transforms() -> anyhow::Result<()> {
        let view = camera().view()?;
        let p = pose();
        let quad = world_quad(4, 2, &p, &[], &[], &view)?;
        assert_eq!(
            quad,
            [[-2., -1., 0.], [2., -1., 0.], [2., 1., 0.], [-2., 1., 0.]]
        );
        let polygon: Vec<_> = quad.into_iter().flatten().collect();
        assert_eq!(quad, world_quad(4, 2, &p, &polygon, &[], &view)?);
        let mut p = pose();
        p.cx = 1.;
        p.base_sx = 2.;
        p.sy = 3.;
        p.rz = 90.;
        p.x = 10.;
        let quad = world_quad(4, 2, &p, &[2., 3., 4., 2.], &[], &view)?;
        close(&quad[0], &[18., -9., 4.]);
        assert!(world_quad(4, 2, &p, &[0.; 9], &[], &view).is_err());
        assert!(world_quad(4, 2, &p, &[f64::NAN], &[], &view).is_err());
        Ok(())
    }

    #[test]
    fn nested_groups_compose_in_order_around_their_centers() -> anyhow::Result<()> {
        let groups = [
            10., 0., 0., 1., 0., 0., 0., 0., 90., 2., 3., 1., 0., 20., 0., 0., 1., 0., 0., 0., 90.,
            1., 2., 1.,
        ];
        // (2,1) -> (-3,2)+(10,0)=(7,2) -> (-2,7)+(0,20)=(-2,27)
        close(&group_point([2., 1., 0.], &groups), &[-2., 27., 0.]);
        let mut reversed = groups[12..].to_vec();
        reversed.extend_from_slice(&groups[..12]);
        assert_ne!(
            group_point([2., 1., 0.], &groups),
            group_point([2., 1., 0.], &reversed)
        );
        let view = camera().view()?;
        let quad = world_quad(2, 2, &pose(), &[], &groups, &view)?;
        close(&quad[2], &group_point([1., 1., 0.], &groups));
        Ok(())
    }
}
