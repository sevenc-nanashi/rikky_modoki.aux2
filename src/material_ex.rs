//! materialdrawExの照明。画像と光源はLuaオブジェクトの寿命に合わせて所有する。
use std::sync::Arc;

use aviutl2::module::{AsScriptModuleUserData, FromScriptModuleParam, ScriptModuleUserData};

use crate::glass::{self, Camera, Image, Lease, Pose};

type V3 = [f64; 3];
fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: V3, b: V3) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(v: V3) -> anyhow::Result<V3> {
    let length = v[0].hypot(v[1]).hypot(v[2]);
    anyhow::ensure!(
        length.is_finite() && length > 0.,
        "Invalid materialdrawEx light axis"
    );
    Ok(v.map(|v| v / length))
}
fn direction(v: V3) -> anyhow::Result<V3> {
    // 原実装が定義する、光源・視点と評価点が一致する場合の方向。
    if v == [0.; 3] {
        Ok([0., 0., -1.])
    } else {
        unit(v)
    }
}

#[derive(FromScriptModuleParam)]
pub struct Settings {
    pub ambient_r: f64,
    pub ambient_g: f64,
    pub ambient_b: f64,
    pub emissive_r: f64,
    pub emissive_g: f64,
    pub emissive_b: f64,
    pub damping: f64,
    pub hq: bool,
    pub partition: usize,
}

#[derive(Clone, FromScriptModuleParam)]
pub struct LightInput {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub specular_r: f64,
    pub specular_g: f64,
    pub specular_b: f64,
    pub shininess: f64,
    pub kind: i32,
    pub double: bool,
    pub degree: f64,
    pub degree2: f64,
    pub nx: f64,
    pub ny: f64,
    pub nz: f64,
    pub nx2: Option<f64>,
    pub ny2: Option<f64>,
    pub nz2: Option<f64>,
    pub wx: f64,
    pub wy: f64,
    pub wz: f64,
    pub layer_rotation: bool,
    pub rx: f64,
    pub ry: f64,
    pub rz: f64,
    pub width: usize,
    pub height: usize,
    pub partition: usize,
    pub alpha: f64,
}

struct Cell {
    bounds: [f64; 4],
    color: V3,
    blocked: u8,
    opaque: bool,
}
enum Kind {
    Point,
    Spot {
        front: V3,
        back: Option<V3>,
        cosine: f64,
        cosine2: f64,
    },
    Area {
        normal: V3,
        width: V3,
        height: V3,
        cosine: f64,
        double: bool,
        cells: Vec<Cell>,
    },
}
struct Light {
    position: V3,
    color: V3,
    specular: V3,
    shininess: f64,
    kind: Kind,
}
pub struct Material {
    original: Arc<Image>,
    settings: Settings,
    lights: Vec<Light>,
}
impl AsScriptModuleUserData for Material {}
pub type Handle = ScriptModuleUserData<Material>;

pub fn create(original: Lease, settings: Settings) -> anyhow::Result<Handle> {
    anyhow::ensure!(
        settings.partition > 0 && settings.damping.is_finite(),
        "Invalid materialdrawEx settings"
    );
    Ok(Material {
        original: glass::image(&original)?,
        settings,
        lights: Vec::new(),
    }
    .into())
}

// 中央の整数座標を基準に分割し、端のセルだけを切り詰める。
fn grid(size: usize, partition: usize) -> Vec<(usize, usize)> {
    let first = (size / 2) % partition;
    let mut left = 0;
    let mut right = if first == 0 { partition } else { first };
    let mut result = Vec::new();
    while left < size {
        right = right.min(size);
        result.push((left, right));
        left = right;
        right = right.saturating_add(partition);
    }
    result
}

fn texture_cells(image: &Image, partition: usize, alpha: f64) -> anyhow::Result<Vec<Cell>> {
    anyhow::ensure!(partition > 0, "Invalid materialdrawEx texture partition");
    let xs = grid(image.width, partition);
    let ys = grid(image.height, partition);
    let mut cells = Vec::new();
    cells.try_reserve_exact(xs.len() * ys.len())?;
    for &(top, bottom) in &ys {
        for &(left, right) in &xs {
            let mut sum = [0.; 4];
            for y in top..bottom {
                for x in left..right {
                    let pixel = &image.pixels[(y * image.width + x) * 4..][..4];
                    for i in 0..3 {
                        sum[i] += f64::from(pixel[i]);
                    }
                    sum[3] += (f64::from(pixel[3]) * (alpha * 255.).ceil() / 255.).trunc();
                }
            }
            let divisor = ((right - left) * (bottom - top)) as f64 * 255.;
            cells.push(Cell {
                bounds: [
                    left as f64 - image.width as f64 / 2.,
                    top as f64 - image.height as f64 / 2.,
                    right as f64 - image.width as f64 / 2.,
                    bottom as f64 - image.height as f64 / 2.,
                ],
                color: std::array::from_fn(|i| sum[i] / divisor * sum[3] / divisor),
                blocked: 0,
                opaque: sum[3] > 0.,
            });
        }
    }
    // 未初期化の境界フラグを読む元DLLの不具合は再現しない。
    for y in 0..ys.len() {
        for x in 0..xs.len() {
            let index = y * xs.len() + x;
            let mut blocked = 0;
            for (exists, neighbor, bit) in [
                (x > 0, index.wrapping_sub(1), 1),
                (y > 0, index.wrapping_sub(xs.len()), 2),
                (x + 1 < xs.len(), index + 1, 4),
                (y + 1 < ys.len(), index + xs.len(), 8),
            ] {
                if exists && cells[neighbor].opaque {
                    blocked |= bit;
                }
            }
            cells[index].blocked = blocked;
        }
    }
    cells.retain(|cell| cell.opaque);
    Ok(cells)
}

pub fn add_light(
    material: &Handle,
    input: LightInput,
    texture: Option<Lease>,
) -> anyhow::Result<()> {
    let color = [input.r, input.g, input.b];
    let angles = [
        input.rx,
        input.ry,
        if input.kind == 1 { 0. } else { input.rz },
    ];
    let normal = || {
        if input.layer_rotation {
            Ok(glass::rotate([0., 0., -1.], angles))
        } else {
            unit([input.nx, input.ny, input.nz])
        }
    };
    let cosine = input.degree.to_radians().cos();
    let kind = match input.kind {
        0 => Kind::Point,
        1 => {
            let front = normal()?;
            let back = if input.double {
                Some(if input.layer_rotation {
                    front.map(|v| -v)
                } else {
                    // 未指定の成分だけ、正規化後の表面ベクトルを反転する。
                    unit([
                        input.nx2.unwrap_or(-front[0]),
                        input.ny2.unwrap_or(-front[1]),
                        input.nz2.unwrap_or(-front[2]),
                    ])?
                })
            } else {
                None
            };
            Kind::Spot {
                front,
                back,
                cosine,
                cosine2: input.degree2.to_radians().cos(),
            }
        }
        2 => {
            let normal = normal()?;
            let width = if input.layer_rotation {
                glass::rotate([1., 0., 0.], angles)
            } else {
                unit([input.wx, input.wy, input.wz])?
            };
            anyhow::ensure!(
                dot(normal, width).abs() < 1e-8,
                "materialdrawEx: normal and width axes must be perpendicular"
            );
            let height = unit(cross(width, normal))?;
            let cells = if let Some(texture) = texture {
                texture_cells(
                    glass::image(&texture)?.as_ref(),
                    input.partition,
                    input.alpha,
                )?
            } else {
                anyhow::ensure!(
                    input.width > 0 && input.height > 0,
                    "materialdrawEx: area dimensions must be positive"
                );
                vec![Cell {
                    bounds: [
                        -(input.width as f64) / 2.,
                        -(input.height as f64) / 2.,
                        input.width as f64 / 2.,
                        input.height as f64 / 2.,
                    ],
                    color,
                    blocked: 0,
                    opaque: true,
                }]
            };
            Kind::Area {
                normal,
                width,
                height,
                cosine,
                double: input.double,
                cells,
            }
        }
        _ => anyhow::bail!("Invalid materialdrawEx light kind"),
    };
    let mut material = material.lock().unwrap();
    material.lights.try_reserve(1)?;
    material.lights.push(Light {
        position: [input.x, input.y, input.z],
        color,
        specular: [input.specular_r, input.specular_g, input.specular_b],
        shininess: input.shininess,
        kind,
    });
    Ok(())
}

fn cone(direction: V3, outward: V3, cosine: f64) -> f64 {
    let axial = (-dot(direction, outward)).clamp(-1., 1.);
    if axial < cosine {
        0.
    } else if cosine == 1. {
        1.
    } else {
        (axial - cosine) / (1. - cosine)
    }
}
fn attenuation(delta: V3, damping: f64) -> f64 {
    let squared = damping * damping;
    let distance = dot(delta, delta);
    if squared as f32 == 0. || distance == 0. {
        1.
    } else {
        (squared / distance).min(1.)
    }
}
impl Light {
    fn add(
        &self,
        coefficient: &mut V3,
        normal: V3,
        view: V3,
        direction: V3,
        color: V3,
        attenuation: f64,
    ) -> anyhow::Result<()> {
        let lambert = dot(normal, direction);
        if lambert <= 0. || attenuation == 0. {
            return Ok(());
        }
        let reflection = direction_of_reflection(normal, direction, lambert)?;
        let phong = dot(reflection, view).max(0.).powf(self.shininess);
        let luminance = dot(color, [0.298912, 0.586611, 0.114478]);
        for i in 0..3 {
            coefficient[i] +=
                (lambert + self.specular[i] * luminance * phong) * color[i] * attenuation.min(1.);
        }
        Ok(())
    }
}
fn direction_of_reflection(normal: V3, incoming: V3, lambert: f64) -> anyhow::Result<V3> {
    direction(std::array::from_fn(|i| {
        2. * lambert * normal[i] - incoming[i]
    }))
}
impl Material {
    fn lighting(&self, point: V3, normal: V3, eye: V3) -> anyhow::Result<V3> {
        let mut coefficient = [
            self.settings.emissive_r,
            self.settings.emissive_g,
            self.settings.emissive_b,
        ];
        let view = direction(sub(eye, point))?;
        for light in self.lights.iter().rev() {
            let delta = sub(light.position, point);
            match &light.kind {
                Kind::Point | Kind::Spot { .. } => {
                    let direction = direction(delta)?;
                    let attenuation = attenuation(delta, self.settings.damping);
                    if let Kind::Spot {
                        front,
                        back,
                        cosine,
                        cosine2,
                    } = &light.kind
                    {
                        light.add(
                            &mut coefficient,
                            normal,
                            view,
                            direction,
                            light.color,
                            attenuation * cone(direction, *front, *cosine),
                        )?;
                        if let Some(back) = back {
                            light.add(
                                &mut coefficient,
                                normal,
                                view,
                                direction,
                                light.color,
                                attenuation * cone(direction, *back, *cosine2),
                            )?;
                        }
                    } else {
                        light.add(
                            &mut coefficient,
                            normal,
                            view,
                            direction,
                            light.color,
                            attenuation,
                        )?;
                    }
                }
                Kind::Area {
                    normal: outward,
                    width,
                    height,
                    cosine,
                    double,
                    cells,
                } => {
                    let u = -dot(delta, *width);
                    let v = -dot(delta, *height);
                    for cell in cells {
                        let [left, top, right, bottom] = cell.bounds;
                        // 隣接する不透明セルの共通境界は片方だけに属する。
                        if (u == right && cell.blocked & 4 != 0)
                            || (v == bottom && cell.blocked & 8 != 0)
                        {
                            continue;
                        }
                        let edges = u8::from(u < left)
                            | (u8::from(v < top) << 1)
                            | (u8::from(u > right) << 2)
                            | (u8::from(v > bottom) << 3);
                        if edges & cell.blocked != 0 {
                            continue;
                        }
                        let nearest = std::array::from_fn(|i| {
                            delta[i]
                                + width[i] * u.clamp(left, right)
                                + height[i] * v.clamp(top, bottom)
                        });
                        let distance = attenuation(nearest, self.settings.damping);
                        for side in 0..if *double { 2 } else { 1 } {
                            let outward = outward.map(|v| if side == 0 { v } else { -v });
                            if edges == 0 {
                                if dot(nearest, outward) >= 0. {
                                    continue;
                                }
                                light.add(
                                    &mut coefficient,
                                    normal,
                                    view,
                                    outward.map(|v| -v),
                                    cell.color,
                                    distance,
                                )?;
                            } else {
                                let direction = direction(nearest)?;
                                light.add(
                                    &mut coefficient,
                                    normal,
                                    view,
                                    direction,
                                    cell.color,
                                    distance * cone(direction, outward, *cosine),
                                )?;
                            }
                        }
                    }
                }
            }
        }
        anyhow::ensure!(
            coefficient.iter().all(|v| v.is_finite()),
            "Non-finite materialdrawEx lighting"
        );
        Ok(coefficient)
    }
}

fn world_at(quad: [V3; 4], width: usize, height: usize, x: usize, y: usize) -> V3 {
    let u = if width == 1 {
        0.5
    } else {
        x as f64 / (width - 1) as f64
    };
    let v = if height == 1 {
        0.5
    } else {
        y as f64 / (height - 1) as f64
    };
    std::array::from_fn(|i| {
        (quad[0][i] * (1. - u) + quad[1][i] * u) * (1. - v)
            + (quad[3][i] * (1. - u) + quad[2][i] * u) * v
    })
}
fn shade(pixel: &mut [u8], coefficient: V3, ambient: V3) {
    for i in 0..3 {
        pixel[i] = ((f64::from(pixel[i]) * 0.00392156862 * coefficient[i] + ambient[i]) * 255.)
            .clamp(0., 255.) as u8;
    }
}

pub fn render(
    material: Handle,
    pose: Pose,
    camera: Camera,
    args: Vec<f64>,
    groups: Vec<f64>,
) -> anyhow::Result<Lease> {
    let material = material.lock().unwrap();
    let original = &material.original;
    let quad = glass::world_quad(
        original.width,
        original.height,
        &pose,
        &args,
        &groups,
        &camera.view()?,
    )?;
    let (center, mut normal) = glass::surface(quad)?;
    let eye = [camera.x, camera.y, camera.z];
    if dot(direction(sub(eye, center))?, normal) < 0. {
        normal = normal.map(|v| -v);
    }
    let settings = &material.settings;
    let ambient = [settings.ambient_r, settings.ambient_g, settings.ambient_b];
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(original.pixels.len())?;
    pixels.extend_from_slice(&original.pixels);
    let mut paint = |bounds: [usize; 4], point: V3| -> anyhow::Result<()> {
        let coefficient = material.lighting(point, normal, eye)?;
        for y in bounds[1]..bounds[3] {
            for x in bounds[0]..bounds[2] {
                shade(
                    &mut pixels[(y * original.width + x) * 4..][..4],
                    coefficient,
                    ambient,
                );
            }
        }
        Ok(())
    };
    if settings.hq && args.len() <= 8 {
        let columns = grid(original.width, settings.partition);
        for (top, bottom) in grid(original.height, settings.partition) {
            for &(left, right) in &columns {
                let a = world_at(quad, original.width, original.height, left, top);
                let point = if settings.partition == 1 {
                    a
                } else {
                    let b = world_at(quad, original.width, original.height, right, bottom);
                    std::array::from_fn(|i| (a[i] + b[i]) / 2.)
                };
                paint([left, top, right, bottom], point)?;
            }
        }
    } else {
        let mut bounds = [0, 0, original.width, original.height];
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
                let size = [original.width, original.height][axis] as f64;
                bounds[axis] = (min - 2.).clamp(0., size).ceil() as usize;
                bounds[axis + 2] = (max + 2.).clamp(0., size).ceil() as usize;
            }
        }
        paint(bounds, center)?;
    }
    Ok(glass::owned(Image {
        width: original.width,
        height: original.height,
        pixels,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(width: usize, height: usize) -> Material {
        Material {
            original: Arc::new(Image {
                width,
                height,
                pixels: [100, 150, 200, 73].repeat(width * height),
            }),
            settings: Settings {
                ambient_r: 0.,
                ambient_g: 0.,
                ambient_b: 0.,
                emissive_r: 0.1,
                emissive_g: 0.1,
                emissive_b: 0.1,
                damping: 0.,
                hq: false,
                partition: 1,
            },
            lights: Vec::new(),
        }
    }
    fn light(kind: Kind) -> Light {
        Light {
            position: [0., 0., -10.],
            color: [1.; 3],
            specular: [0.; 3],
            shininess: 10.,
            kind,
        }
    }
    fn sample(m: &Material, point: V3) -> V3 {
        m.lighting(point, [0., 0., -1.], [0., 0., -100.]).unwrap()
    }
    fn close(a: V3, b: V3) {
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 1e-9, "{a:?} != {b:?}");
        }
    }
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
    fn area(cells: Vec<Cell>, double: bool) -> Kind {
        Kind::Area {
            normal: [0., 0., 1.],
            width: [1., 0., 0.],
            height: [0., -1., 0.],
            cosine: 0.5,
            double,
            cells,
        }
    }
    fn cell() -> Cell {
        Cell {
            bounds: [-2., -2., 2., 2.],
            color: [1.; 3],
            blocked: 0,
            opaque: true,
        }
    }

    #[test]
    fn ambient_is_added_and_emissive_multiplies_preserving_alpha() {
        let mut pixel = [100, 150, 200, 73];
        shade(&mut pixel, [0.1, 0.5, 1.], [0., 0.2, 0.]);
        assert_eq!(pixel, [9, 125, 199, 73]);
        shade(&mut pixel, [100.; 3], [0.; 3]);
        assert_eq!(pixel, [255, 255, 255, 73]);
    }

    #[test]
    fn unlimited_point_lights_attenuate_and_color_specular() {
        let mut m = material(1, 1);
        for _ in 0..8 {
            m.lights.push(light(Kind::Point));
        }
        close(sample(&m, [0.; 3]), [8.1; 3]);
        m.settings.damping = 5.;
        close(sample(&m, [0.; 3]), [2.1; 3]);
        m.lights.truncate(1);
        m.lights[0].color = [1., 0., 0.];
        m.lights[0].specular = [2.; 3];
        close(
            sample(&m, [0.; 3]),
            [0.1 + (1. + 2. * 0.298912) * 0.25, 0.1, 0.1],
        );
        assert_eq!(attenuation([0.; 3], 5.), 1.);
        assert_eq!(attenuation([0., 0., -100.], 0.), 1.);
    }

    #[test]
    fn spot_front_back_and_zero_degree_are_finite() {
        let mut m = material(1, 1);
        m.lights.push(light(Kind::Spot {
            front: [0., 0., 1.],
            back: None,
            cosine: 1.,
            cosine2: 1.,
        }));
        close(sample(&m, [0.; 3]), [1.1; 3]);
        close(sample(&m, [1., 0., 0.]), [0.1; 3]);
        m.lights[0].kind = Kind::Spot {
            front: [0., 0., -1.],
            back: Some([0., 0., 1.]),
            cosine: 1.,
            cosine2: 1.,
        };
        close(sample(&m, [0.; 3]), [1.1; 3]);
        assert_eq!(cone([1., 0., 0.], [0., 0., 1.], 0.5), 0.);
    }

    #[test]
    fn area_projects_to_face_edges_and_corners() {
        let mut m = material(1, 1);
        m.lights.push(light(area(vec![cell()], false)));
        close(sample(&m, [0.; 3]), [1.1; 3]);
        let edge = sample(&m, [4., 0., 0.])[0];
        let corner = sample(&m, [4., 4., 0.])[0];
        assert!(0.1 < corner && corner < edge && edge < 1.1);
        close(sample(&m, [100., 0., 0.]), [0.1; 3]);
        // 面の裏側から見る対象でも、光源を両面にした場合だけ光が届く。
        let back = |m: &Material| m.lighting([0., 0., -20.], [0., 0., 1.], [0.; 3]).unwrap();
        close(back(&m), [0.1; 3]);
        m.lights[0].kind = area(vec![cell()], true);
        close(back(&m), [1.1; 3]);
    }

    #[test]
    fn texture_averages_rgb_and_alpha_separately_and_keeps_black_neighbors() {
        let image = Image {
            width: 4,
            height: 1,
            pixels: vec![255, 0, 0, 255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 0, 255],
        };
        let cells = texture_cells(&image, 2, 1.).unwrap();
        assert_eq!(cells.len(), 2);
        close(cells[0].color, [0.25, 0., 0.25]);
        assert_eq!(cells[0].blocked, 4);
        assert_eq!(cells[1].blocked, 1);
        close(cells[1].color, [0.; 3]);
        let dim = texture_cells(&image, 2, 0.5).unwrap();
        close(dim[0].color, [128. / 1020., 0., 128. / 1020.]);
        assert!(texture_cells(&image, 2, 0.).unwrap().is_empty());
    }

    #[test]
    fn texture_shared_boundaries_do_not_multiply_light() {
        let image = Image {
            width: 4,
            height: 4,
            pixels: [255; 4].repeat(16),
        };
        let mut m = material(1, 1);
        m.lights
            .push(light(area(texture_cells(&image, 2, 1.).unwrap(), false)));
        for point in [[0.; 3], [0., 1., 0.], [1., 1., 0.], [-1., 0., 0.]] {
            close(sample(&m, point), [1.1; 3]);
        }
    }

    #[test]
    fn centered_grid_clips_edges_and_single_pixel_coordinates_are_finite() {
        assert_eq!(grid(7, 3), vec![(0, 3), (3, 6), (6, 7)]);
        assert_eq!(grid(8, 3), vec![(0, 1), (1, 4), (4, 7), (7, 8)]);
        assert_eq!(grid(1, 300), vec![(0, 1)]);
        let quad = [[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]];
        assert_eq!(world_at(quad, 1, 1, 0, 0), [0.; 3]);
        assert_eq!(world_at(quad, 3, 3, 3, 3), [2., 2., 0.]);
    }

    #[test]
    fn hq_samples_pixels_and_blocks_without_modifying_source() -> anyhow::Result<()> {
        let mut m = material(5, 3);
        m.settings.hq = true;
        let mut lamp = light(Kind::Point);
        lamp.position = [-2.5, 0., -1.];
        m.lights.push(lamp);
        let saved = m.original.clone();
        let image = glass::image(&render(m.into(), pose(), camera(), vec![], vec![])?)?;
        assert!(image.pixels[20] > image.pixels[36]);
        assert!(image.pixels.chunks_exact(4).all(|p| p[3] == 73));
        assert_eq!(saved.pixels, [100, 150, 200, 73].repeat(15));
        let mut m = material(8, 3);
        m.settings.hq = true;
        m.settings.partition = 3;
        m.lights.push(light(Kind::Point));
        let image = glass::image(&render(m.into(), pose(), camera(), vec![], vec![])?)?;
        assert_eq!(&image.pixels[4..8], &image.pixels[12..16]);
        let mut m = material(1, 1);
        m.settings.hq = true;
        assert_eq!(
            glass::image(&render(m.into(), pose(), camera(), vec![], vec![])?)?.pixels,
            [9, 14, 19, 73]
        );
        Ok(())
    }

    #[test]
    fn polygon_ignores_hq_and_only_shades_uv_bounds() -> anyhow::Result<()> {
        let mut m = material(10, 10);
        m.settings.hq = true;
        let args = vec![
            -5., -5., 0., 5., -5., 0., 5., 5., 0., -5., 5., 0., 4., 4., 5., 4., 5., 5., 4., 5., 0.5,
        ];
        let image = glass::image(&render(m.into(), pose(), camera(), args, vec![])?)?;
        for y in 0..10 {
            for x in 0..10 {
                let expected = if (2..7).contains(&x) && (2..7).contains(&y) {
                    [9, 14, 19, 73]
                } else {
                    [100, 150, 200, 73]
                };
                assert_eq!(&image.pixels[(y * 10 + x) * 4..][..4], expected);
            }
        }
        Ok(())
    }

    #[test]
    fn light_builder_normalizes_axes_and_copies_texture_cells() -> anyhow::Result<()> {
        let m: Handle = material(1, 1).into();
        let mut input = LightInput {
            x: 0.,
            y: 0.,
            z: -10.,
            r: 1.,
            g: 1.,
            b: 1.,
            specular_r: 0.,
            specular_g: 0.,
            specular_b: 0.,
            shininess: 10.,
            kind: 1,
            double: true,
            degree: 45.,
            degree2: 30.,
            nx: 0.,
            ny: 0.,
            nz: 2.,
            nx2: Some(1.),
            ny2: None,
            nz2: None,
            wx: 1.,
            wy: 0.,
            wz: 0.,
            layer_rotation: false,
            rx: 0.,
            ry: 0.,
            rz: 0.,
            width: 4,
            height: 4,
            partition: 2,
            alpha: 1.,
        };
        add_light(&m, input.clone(), None)?;
        {
            let m = m.lock().unwrap();
            let Kind::Spot { front, back, .. } = m.lights[0].kind else {
                panic!("Expected spotlight")
            };
            close(front, [0., 0., 1.]);
            close(back.unwrap(), [0.5f64.sqrt(), 0., -0.5f64.sqrt()]);
        }
        input.layer_rotation = true;
        input.ry = 180.;
        input.rz = 45.; // スポット光源はZ回転を使わない。
        add_light(&m, input.clone(), None)?;
        {
            let m = m.lock().unwrap();
            let Kind::Spot { front, back, .. } = m.lights[1].kind else {
                panic!("Expected spotlight")
            };
            close(front, [0., 0., 1.]);
            close(back.unwrap(), [0., 0., -1.]);
        }
        input.kind = 2;
        let texture = glass::owned(Image {
            width: 4,
            height: 4,
            pixels: [255, 0, 0, 255].repeat(16),
        });
        let image = glass::image(&texture)?;
        let weak = Arc::downgrade(&image);
        drop(image);
        add_light(&m, input.clone(), Some(texture))?;
        assert!(
            weak.upgrade().is_none(),
            "Texture pixels need not outlive cell construction"
        );
        {
            let m = m.lock().unwrap();
            let Kind::Area {
                normal,
                width,
                height,
                cells,
                ..
            } = &m.lights[2].kind
            else {
                panic!("Expected area")
            };
            assert!(dot(*normal, *width).abs() < 1e-9 && dot(*normal, *height).abs() < 1e-9);
            assert_eq!(cells.len(), 4);
            close(cells[0].color, [1., 0., 0.]);
        }
        input.layer_rotation = false;
        input.wz = 1.;
        assert!(add_light(&m, input.clone(), None).is_err());
        input.wz = 0.;
        input.nz = 0.;
        assert!(add_light(&m, input, None).is_err());
        assert_eq!(m.lock().unwrap().lights.len(), 3);
        Ok(())
    }
}
