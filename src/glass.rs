//! glassdrawのCPU処理。ホストの画像は呼び出し中にコピーし、ポインタを保持しない。
use std::{f64::consts::PI, sync::Arc};

use aviutl2::module::{AsScriptModuleUserData, FromScriptModuleParam, ScriptModuleUserData};

type V3 = [f64; 3];
type V2 = [f64; 2];

pub struct Image {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixels: Vec<u8>,
}

pub struct ImageLease(Option<Arc<Image>>);
impl AsScriptModuleUserData for ImageLease {}
pub type Lease = ScriptModuleUserData<ImageLease>;

pub(crate) fn owned(image: Image) -> Lease {
    ImageLease(Some(Arc::new(image))).into()
}

pub(crate) fn image(lease: &Lease) -> anyhow::Result<Arc<Image>> {
    lease
        .lock()
        .unwrap()
        .0
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Glass image already released"))
}

pub fn release(lease: Lease) {
    lease.lock().unwrap().0 = None;
}

pub fn data(lease: Lease) -> anyhow::Result<(*const u8, usize, usize)> {
    let image = image(&lease)?;
    Ok((image.pixels.as_ptr(), image.width, image.height))
}

/// `data` は width * height * 4 バイトの読み取り可能なRGBA画像を指すこと。
pub unsafe fn capture(data: *const u8, width: usize, height: usize) -> anyhow::Result<Lease> {
    anyhow::ensure!(
        !data.is_null() && width > 0 && height > 0,
        "Invalid glass image"
    );
    let len = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(4))
        .filter(|&n| n <= isize::MAX as usize)
        .ok_or_else(|| anyhow::anyhow!("Glass image is too large"))?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(len)?;
    // SAFETY: Lua側でgetpixeldataの直後に呼び、別のホスト関数を挟まずコピーする。
    pixels.extend_from_slice(unsafe { std::slice::from_raw_parts(data, len) });
    Ok(owned(Image {
        width,
        height,
        pixels,
    }))
}

// Lua側で元DLLの既定値・数値変換を適用済み。型の読み取りはSDKのderiveに任せる。
#[derive(Clone, FromScriptModuleParam)]
pub struct Settings {
    pub color: i32,
    pub reverse: i32,
    pub boundary: i32,
    pub lens: i32,
    pub culling: bool,
    pub refractive: f64,
    pub offset_z: f64,
    pub inverse_zoom: f64,
}

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

fn project(
    corners: [V3; 4],
    view: &View,
    settings: &Settings,
    size: [usize; 2],
) -> anyhow::Result<Option<[V2; 4]>> {
    let (center, normal) = surface(corners)?;
    let facing = dot(
        if view.mode == 0 {
            view.forward
        } else {
            sub(center, view.eye)
        },
        normal,
    );
    if facing >= 0. && settings.culling {
        return Ok(None);
    }
    let sign = if facing < 0. { -1. } else { 1. };
    let denominator = if facing == 0. {
        f64::MIN_POSITIVE
    } else {
        facing.abs()
    };
    let shift: V2 = if settings.refractive == 0. || settings.offset_z == 0. {
        [0.; 2]
    } else {
        let amount = settings.refractive * (settings.offset_z / denominator);
        [
            sign * dot(view.right, normal) * amount,
            -sign * dot(view.up, normal) * amount,
        ]
    };
    let (s, c) = view.roll.sin_cos();
    let mut projected = [[0.; 2]; 4];
    for (out, corner) in projected.iter_mut().zip(corners) {
        let relative = sub(corner, view.eye);
        let depth = dot(view.forward, relative);
        let depth = if depth == 0. {
            f64::MIN_POSITIVE
        } else {
            depth
        };
        let x = dot(view.right, relative) * (view.focal / depth);
        let y = -dot(view.up, relative) * (view.focal / depth);
        *out = [x * c - y * s + shift[0], x * s + y * c + shift[1]];
    }
    let center: V2 = std::array::from_fn(|i| projected.iter().map(|p| p[i]).sum::<f64>() * 0.25);
    for p in &mut projected {
        for i in 0..2 {
            let offset = (p[i] - center[i]) * settings.inverse_zoom;
            let reverse = settings.reverse & if i == 0 { 2 } else { 1 } != 0;
            p[i] = center[i] + if reverse { -offset } else { offset } + size[i] as f64 * 0.5;
        }
    }
    anyhow::ensure!(
        projected.iter().flatten().all(|v| v.is_finite()),
        "Non-finite glass projection"
    );
    Ok(Some(projected))
}

fn boundary(value: f64, extent: usize, mode: i32) -> usize {
    let p = value.trunc();
    let n = extent as f64;
    match mode {
        1 => p.rem_euclid(n) as usize,
        2 => {
            let p = p.rem_euclid(n * 2.);
            if p < n {
                p as usize
            } else {
                (2. * n - 1. - p) as usize
            }
        }
        _ => p.clamp(0., n - 1.) as usize,
    }
}

fn lens_delta(t: f64, lens: i32) -> f64 {
    match lens {
        1 => {
            let warped = 0.5 * (PI * t).sin();
            (if t > 0.5 { 1. - warped } else { warped }) - t
        }
        2 => 0.5 * (1. - (PI * t).cos()) - t,
        _ => 0.,
    }
}

fn sample(
    original: &Image,
    background: &Image,
    corners: [V2; 4],
    settings: &Settings,
) -> anyhow::Result<Image> {
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(original.pixels.len())?;
    let tint = [
        (settings.color >> 16) & 255,
        (settings.color >> 8) & 255,
        settings.color & 255,
    ]
    .map(f64::from);
    for y in 0..original.height {
        let v = if original.height == 1 {
            0.5
        } else {
            y as f64 / (original.height - 1) as f64
        };
        for x in 0..original.width {
            let u = if original.width == 1 {
                0.5
            } else {
                x as f64 / (original.width - 1) as f64
            };
            let uv = [
                u + lens_delta(u, settings.lens) * (PI * v).sin(),
                v + lens_delta(v, settings.lens) * (PI * u).sin(),
            ];
            let point: V2 = std::array::from_fn(|i| {
                let left = corners[0][i] * (1. - uv[1]) + corners[3][i] * uv[1];
                let right = corners[1][i] * (1. - uv[1]) + corners[2][i] * uv[1];
                left * (1. - uv[0]) + right * uv[0]
            });
            anyhow::ensure!(
                point.iter().all(|v| v.is_finite()),
                "Non-finite glass sample"
            );
            let sx = boundary(point[0], background.width, settings.boundary);
            let sy = boundary(point[1], background.height, settings.boundary);
            let index = (sy * background.width + sx) * 4;
            let mut rgb = [
                background.pixels[index],
                background.pixels[index + 1],
                background.pixels[index + 2],
            ];
            if settings.color != -1 {
                let difference = (f64::from(rgb[0]) - tint[0]) * 0.298912
                    + (f64::from(rgb[1]) - tint[1]) * 0.586611
                    + (f64::from(rgb[2]) - tint[2]) * 0.114478;
                rgb = tint.map(|t| (t + difference).trunc().clamp(0., 255.) as u8);
            }
            pixels.extend_from_slice(&rgb);
            pixels.push(original.pixels[(y * original.width + x) * 4 + 3]);
        }
    }
    Ok(Image {
        width: original.width,
        height: original.height,
        pixels,
    })
}

#[allow(clippy::too_many_arguments)] // Lua側の画像、設定、ホスト状態を独立した引数で受け取る。
pub fn render(
    original: Lease,
    background: Lease,
    settings: Settings,
    pose: Pose,
    camera: Camera,
    args: Vec<f64>,
    groups: Vec<f64>,
) -> anyhow::Result<Option<Lease>> {
    let original = image(&original)?;
    let background = image(&background)?;
    let view = camera.view()?;
    let quad = world_quad(
        original.width,
        original.height,
        &pose,
        &args,
        &groups,
        &view,
    )?;
    let Some(corners) = project(
        quad,
        &view,
        &settings,
        [background.width, background.height],
    )?
    else {
        return Ok(None);
    };
    Ok(Some(owned(sample(
        &original,
        &background,
        corners,
        &settings,
    )?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings {
            color: -1,
            reverse: 0,
            boundary: 0,
            lens: 0,
            culling: false,
            refractive: 0.,
            offset_z: 300.,
            inverse_zoom: 1.,
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

    fn close(a: &[f64], b: &[f64]) {
        for (a, b) in a.iter().zip(b) {
            assert!((a - b).abs() < 1e-9, "{a} != {b}");
        }
    }

    #[test]
    fn boundary_wraps_negative_tiles_and_truncates_towards_zero() {
        for (p, clamp, wrap, mirror) in [
            (-9., 0, 3, 0),
            (-8., 0, 0, 0),
            (-5., 0, 3, 3),
            (-4., 0, 0, 3),
            (-1., 0, 3, 0),
            (-0.9, 0, 0, 0),
            (0., 0, 0, 0),
            (3.9, 3, 3, 3),
            (4., 3, 0, 3),
            (7., 3, 3, 0),
            (8., 3, 0, 0),
        ] {
            assert_eq!(boundary(p, 4, 0), clamp);
            assert_eq!(boundary(p, 4, 1), wrap);
            assert_eq!(boundary(p, 4, 2), mirror);
        }
        for mode in 0..3 {
            for p in [-100., -1., 0., 1., 100.] {
                assert_eq!(boundary(p, 1, mode), 0);
            }
        }
    }

    #[test]
    fn sampling_uses_background_rgb_and_original_alpha() -> anyhow::Result<()> {
        let original = Image {
            width: 2,
            height: 2,
            pixels: vec![
                99, 98, 97, 0, 99, 98, 97, 64, 99, 98, 97, 128, 99, 98, 97, 255,
            ],
        };
        let background = Image {
            width: 2,
            height: 2,
            pixels: vec![1, 2, 3, 0, 11, 12, 13, 0, 21, 22, 23, 0, 31, 32, 33, 0],
        };
        let corners = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let result = sample(&original, &background, corners, &settings())?;
        assert_eq!(
            result.pixels,
            vec![1, 2, 3, 0, 11, 12, 13, 64, 21, 22, 23, 128, 31, 32, 33, 255]
        );
        let mut tint = settings();
        tint.color = 0xff0000;
        let original = Image {
            width: 1,
            height: 1,
            pixels: vec![0, 0, 0, 73],
        };
        let background = Image {
            width: 1,
            height: 1,
            pixels: vec![100, 150, 200, 0],
        };
        assert_eq!(
            sample(&original, &background, corners, &tint)?.pixels,
            vec![255, 64, 64, 73]
        );
        Ok(())
    }

    #[test]
    fn lenses_and_one_pixel_axes_use_the_expected_sample() -> anyhow::Result<()> {
        let background = Image {
            width: 11,
            height: 1,
            pixels: (0..11).flat_map(|x| [x, 0, 0, 0]).collect(),
        };
        let original = Image {
            width: 5,
            height: 1,
            pixels: vec![255; 20],
        };
        let corners = [[0., 0.], [10., 0.], [10., 0.], [0., 0.]];
        for (lens, expected) in [
            (0, vec![0, 2, 5, 7, 10]),
            // sin(PI)の浮動小数点誤差も、元DLL同様に切り捨てる。
            (1, vec![0, 3, 5, 6, 9]),
            (2, vec![0, 1, 4, 8, 10]),
        ] {
            let mut options = settings();
            options.lens = lens;
            let result = sample(&original, &background, corners, &options)?;
            assert_eq!(
                result
                    .pixels
                    .chunks_exact(4)
                    .map(|p| p[0])
                    .collect::<Vec<_>>(),
                expected
            );
        }
        let original = Image {
            width: 1,
            height: 1,
            pixels: vec![255; 4],
        };
        assert_eq!(
            sample(&original, &background, corners, &settings())?.pixels,
            [5, 0, 0, 255]
        );
        Ok(())
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

    #[test]
    fn projection_handles_zoom_reverse_refraction_and_culling() -> anyhow::Result<()> {
        let view = camera().view()?;
        let quad = world_quad(4, 2, &pose(), &[], &[], &view)?;
        let mut options = settings();
        let projected = project(quad, &view, &options, [20, 10])?.unwrap();
        assert_eq!(projected, [[8., 4.], [12., 4.], [12., 6.], [8., 6.]]);
        options.inverse_zoom = 0.5;
        options.reverse = 3;
        assert_eq!(
            project(quad, &view, &options, [20, 10])?.unwrap(),
            [[11., 5.5], [9., 5.5], [9., 4.5], [11., 4.5]]
        );
        let mut p = pose();
        p.ry = 45.;
        let quad = world_quad(4, 2, &p, &[], &[], &view)?;
        options = settings();
        let before = project(quad, &view, &options, [20, 10])?.unwrap();
        options.refractive = 0.5;
        options.offset_z = 10.;
        let after = project(quad, &view, &options, [20, 10])?.unwrap();
        close(
            &[after[0][0] - before[0][0], after[0][1] - before[0][1]],
            &[5., 0.],
        );
        options.culling = true;
        p.ry = 180.;
        assert!(
            project(
                world_quad(4, 2, &p, &[], &[], &view)?,
                &view,
                &options,
                [20, 10]
            )?
            .is_none()
        );
        Ok(())
    }

    #[test]
    fn camera_roll_and_billboard_align_the_plane() -> anyhow::Result<()> {
        let mut cam = camera();
        cam.mode = 1;
        cam.x = -1024.;
        cam.z = 0.;
        let view = cam.view()?;
        let mut p = pose();
        p.billboard = 3;
        let quad = world_quad(4, 2, &p, &[], &[], &view)?;
        let projected = project(quad, &view, &settings(), [20, 10])?.unwrap();
        close(&projected[0], &[8., 4.]);
        let groups = [0., 0., 0., 0., 0., 0., 0., 30., 45., 1., 1., 1.];
        let grouped = world_quad(4, 2, &p, &[], &groups, &view)?;
        for (a, b) in quad.iter().zip(grouped) {
            close(a, &b);
        }
        let groups = [
            0., 0., 0., 0., 0., 0., 0., 30., 45., 2., 3., 1., 0., 0., 0., 0., 0., 0., 20., 40.,
            60., 3., 2., 1.,
        ];
        let grouped = world_quad(4, 2, &p, &[], &groups, &view)?;
        close(
            &project(grouped, &view, &settings(), [20, 10])?.unwrap()[0],
            &[-2., -1.],
        );
        cam = camera();
        cam.rz = 90.;
        let view = cam.view()?;
        let quad = world_quad(4, 2, &pose(), &[], &[], &view)?;
        close(
            &project(quad, &view, &settings(), [20, 10])?.unwrap()[0],
            &[11., 3.],
        );
        Ok(())
    }

    #[test]
    fn capture_owns_pixels_and_release_invalidates_the_lease() -> anyhow::Result<()> {
        let mut source = [1, 2, 3, 4];
        // SAFETY: sourceは4バイトのRGBA画像。
        let lease = unsafe { capture(source.as_ptr(), 1, 1)? };
        source.fill(0);
        assert_eq!(image(&lease)?.pixels, [1, 2, 3, 4]);
        let snapshot = image(&lease)?;
        release(lease);
        assert_eq!(snapshot.pixels, [1, 2, 3, 4]);
        // SAFETY: 不正な入力は参照する前にエラーにする。
        assert!(unsafe { capture(std::ptr::null(), 1, 1) }.is_err());
        Ok(())
    }
}
