//! 旧materialdraw。面の中心で求めた照明係数を画像全体（またはUV矩形）へ適用する。
use crate::glass::{self, Camera, Image, Lease, Pose};
use aviutl2::module::FromScriptModuleParam;

type Vector = [f64; 3];

#[derive(Clone, FromScriptModuleParam)]
pub struct Settings {
    pub ambient_r: f64,
    pub ambient_g: f64,
    pub ambient_b: f64,
    pub specular_r: f64,
    pub specular_g: f64,
    pub specular_b: f64,
    pub emissive_r: f64,
    pub emissive_g: f64,
    pub emissive_b: f64,
    pub shininess: f64,
}

fn dot(a: Vector, b: Vector) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}

fn direction(v: Vector) -> anyhow::Result<Vector> {
    let length = v[0].hypot(v[1]).hypot(v[2]);
    anyhow::ensure!(length.is_finite(), "Non-finite material light direction");
    // 原実装では、面と光源/カメラが同位置の場合の方向を-Zと定義している。
    if length == 0. {
        return Ok([0., 0., -1.]);
    }
    Ok(v.map(|value| value / length))
}

fn coefficients(
    settings: &Settings,
    center: Vector,
    mut normal: Vector,
    eye: Vector,
    lights: &[f64],
) -> anyhow::Result<(Vector, Vector)> {
    anyhow::ensure!(
        lights.len().is_multiple_of(6) && lights.len() <= 24,
        "Invalid material lights"
    );
    let view = direction(sub(eye, center))?;
    if dot(view, normal) < 0. {
        normal = normal.map(|v| -v);
    }
    let mut diffuse = [0.; 3];
    let mut specular = [0.; 3];
    let specular_color = [
        settings.specular_r,
        settings.specular_g,
        settings.specular_b,
    ];
    for light in lights.chunks_exact(6).rev() {
        let to_light = direction(sub([light[0], light[1], light[2]], center))?;
        let lambert = dot(normal, to_light);
        if lambert <= 0. {
            continue;
        }
        let reflection = direction(std::array::from_fn(|i| {
            2. * lambert * normal[i] - to_light[i]
        }))?;
        let highlight = dot(reflection, view).max(0.).powf(settings.shininess);
        for i in 0..3 {
            diffuse[i] += light[i + 3] * lambert;
            // 鏡面反射には光源色を掛けず、ライトごとに整数へ切り捨てる。
            specular[i] = (specular[i] + specular_color[i] * highlight).trunc();
        }
    }
    let ambient = [settings.ambient_r, settings.ambient_g, settings.ambient_b];
    let emissive = [
        settings.emissive_r,
        settings.emissive_g,
        settings.emissive_b,
    ];
    Ok((
        std::array::from_fn(|i| ambient[i] + diffuse[i]),
        std::array::from_fn(|i| emissive[i] + specular[i]),
    ))
}

fn shade(original: &Image, multiply: Vector, add: Vector, uv: &[f64]) -> anyhow::Result<Image> {
    anyhow::ensure!(
        uv.is_empty() || uv.len() == 8,
        "Invalid material UV coordinates"
    );
    anyhow::ensure!(
        uv.iter()
            .chain(multiply.iter())
            .chain(add.iter())
            .all(|v| v.is_finite()),
        "Non-finite material coefficients or UV"
    );
    let mut bounds = [0, 0, original.width, original.height];
    if !uv.is_empty() {
        // AviUtl2のobj.w/hとRGBAの寸法は一致する。元DLLと同じ±2ピクセルの余白。
        for axis in 0..2 {
            let min = uv
                .iter()
                .skip(axis)
                .step_by(2)
                .copied()
                .fold(f64::INFINITY, f64::min);
            let max = uv
                .iter()
                .skip(axis)
                .step_by(2)
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            let size = [original.width, original.height][axis] as f64;
            bounds[axis] = (((min - 2.) / size).clamp(0., 1.) * size).ceil() as usize;
            bounds[axis + 2] = (((max + 2.) / size).clamp(0., 1.) * size).ceil() as usize;
        }
    }
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(original.pixels.len())?;
    pixels.extend_from_slice(&original.pixels);
    for y in bounds[1]..bounds[3] {
        for x in bounds[0]..bounds[2] {
            let pixel = &mut pixels[(y * original.width + x) * 4..][..4];
            for i in 0..3 {
                pixel[i] = (f64::from(pixel[i]) * multiply[i] + add[i])
                    .trunc()
                    .clamp(0., 255.) as u8;
            }
        }
    }
    Ok(Image {
        width: original.width,
        height: original.height,
        pixels,
    })
}

#[allow(clippy::too_many_arguments)] // glassdrawと同じ描画コンテキストを受け取る。
pub fn render(
    original: Lease,
    settings: Settings,
    pose: Pose,
    camera: Camera,
    args: Vec<f64>,
    groups: Vec<f64>,
    lights: Vec<f64>,
) -> anyhow::Result<Lease> {
    let original = glass::image(&original)?;
    let view = camera.view()?;
    let quad = glass::world_quad(
        original.width,
        original.height,
        &pose,
        &args,
        &groups,
        &view,
    )?;
    let (center, normal) = glass::surface(quad)?;
    let (multiply, add) = coefficients(
        &settings,
        center,
        normal,
        [camera.x, camera.y, camera.z],
        &lights,
    )?;
    let uv = if args.len() >= 20 { &args[12..20] } else { &[] };
    Ok(glass::owned(shade(&original, multiply, add, uv)?))
}

pub fn layer_position(position: Vec<f64>, groups: Vec<f64>) -> anyhow::Result<Vec<f64>> {
    let position: Vector = position
        .try_into()
        .map_err(|_| anyhow::anyhow!("Invalid light position"))?;
    anyhow::ensure!(
        groups.len().is_multiple_of(12),
        "Invalid material group data"
    );
    Ok(glass::group_point(position, &groups).to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings {
            ambient_r: 0.,
            ambient_g: 0.,
            ambient_b: 0.,
            specular_r: 0.,
            specular_g: 0.,
            specular_b: 0.,
            emissive_r: 0.,
            emissive_g: 0.,
            emissive_b: 0.,
            shininess: 0.,
        }
    }

    fn lighting(settings: &Settings, lights: &[f64]) -> anyhow::Result<(Vector, Vector)> {
        coefficients(settings, [0.; 3], [0., 0., -1.], [0., 0., -10.], lights)
    }

    #[test]
    fn ambient_emissive_and_alpha_are_independent() -> anyhow::Result<()> {
        let mut settings = settings();
        settings.ambient_r = 0.5;
        settings.ambient_g = 1.;
        settings.emissive_b = 100.;
        let (multiply, add) = lighting(&settings, &[])?;
        let original = Image {
            width: 2,
            height: 1,
            pixels: vec![101, 200, 150, 0, 255, 0, 0, 127],
        };
        let output = shade(&original, multiply, add, &[])?;
        assert_eq!(output.pixels, [50, 200, 100, 0, 127, 0, 100, 127]);
        assert_eq!(original.pixels, [101, 200, 150, 0, 255, 0, 0, 127]);
        Ok(())
    }

    #[test]
    fn four_lights_accumulate_without_distance_attenuation() -> anyhow::Result<()> {
        let lights = [
            0., 0., -10., 1., 0., 0., 0., 0., -1000., 0., 1., 0., 0., 0., -1., 0., 0., 1., 0., 0.,
            -50., 0.5, 0.5, 0.5,
        ];
        let (multiply, add) = lighting(&settings(), &lights)?;
        assert_eq!(multiply, [1.5; 3]);
        let original = Image {
            width: 1,
            height: 1,
            pixels: vec![200, 100, 1, 231],
        };
        assert_eq!(
            shade(&original, multiply, add, &[])?.pixels,
            [255, 150, 1, 231]
        );
        let back = coefficients(&settings(), [0.; 3], [0., 0., 1.], [0., 0., -10.], &lights)?;
        assert_eq!(back, (multiply, add));
        assert_eq!(
            lighting(&settings(), &[0., 0., 10., 1., 1., 1.])?.0,
            [0.; 3]
        );
        assert!(lighting(&settings(), &[0.; 30]).is_err());
        Ok(())
    }

    #[test]
    fn specular_ignores_light_color_and_truncates_for_each_light() -> anyhow::Result<()> {
        let mut settings = settings();
        settings.specular_r = 1.9;
        settings.specular_g = 3.7;
        settings.emissive_r = 5.;
        let lights = [0., 0., -10., 0., 0., 0.].repeat(4);
        assert_eq!(lighting(&settings, &lights)?, ([0.; 3], [9., 12., 0.]));
        settings.specular_r = 100.;
        settings.specular_g = 0.;
        settings.emissive_r = 0.;
        settings.shininess = 2.;
        let result = lighting(&settings, &[3., 0., -4., 0., 0., 0.])?;
        assert_eq!(result.1, [64., 0., 0.]);
        settings.shininess = 0.;
        assert_eq!(
            lighting(&settings, &[3., 0., -4., 0., 0., 0.])?.1,
            [100., 0., 0.]
        );
        // 同位置の光源とカメラは、旧版の-Z方向の定義を使う。
        assert_eq!(
            coefficients(&settings, [0.; 3], [0., 0., -1.], [0.; 3], &[0.; 6])?.1,
            [100., 0., 0.]
        );
        Ok(())
    }

    #[test]
    fn explicit_uv_only_changes_the_expanded_rectangle() -> anyhow::Result<()> {
        let original = Image {
            width: 12,
            height: 10,
            pixels: [10, 20, 30, 41].repeat(120),
        };
        // [3.2,5.2] x [4.2,6.2] に±2、ceilを適用: [2,8) x [3,9)。
        let output = shade(
            &original,
            [0.; 3],
            [100.; 3],
            &[3.2, 4.2, 5.2, 4.2, 5.2, 6.2, 3.2, 6.2],
        )?;
        for y in 0..10 {
            for x in 0..12 {
                let expected = if (2..8).contains(&x) && (3..9).contains(&y) {
                    [100, 100, 100, 41]
                } else {
                    [10, 20, 30, 41]
                };
                assert_eq!(
                    &output.pixels[(y * 12 + x) * 4..][..4],
                    &expected,
                    "{x}, {y}"
                );
            }
        }
        assert_eq!(
            shade(&original, [0.; 3], [100.; 3], &[-20.; 8])?.pixels,
            original.pixels
        );
        assert_eq!(
            shade(&original, [0.; 3], [100.; 3], &[100.; 8])?.pixels,
            original.pixels
        );
        assert!(shade(&original, [0.; 3], [0.; 3], &[f64::NAN; 8]).is_err());
        Ok(())
    }

    #[test]
    fn layer_positions_use_all_three_components_and_nested_groups() -> anyhow::Result<()> {
        let groups = vec![
            10., 0., 0., 1., 0., 0., 0., 0., 90., 2., 3., 1., 0., 20., 0., 0., 1., 0., 0., 0., 90.,
            1., 2., 1.,
        ];
        let position = layer_position(vec![2., 1., 7.], groups)?;
        for (got, expected) in position.into_iter().zip([-2., 27., 7.]) {
            assert!((got - expected).abs() < 1e-9);
        }
        Ok(())
    }

    #[test]
    fn render_uses_snapshot_geometry_and_retains_original_pixels() -> anyhow::Result<()> {
        let original = glass::owned(Image {
            width: 2,
            height: 2,
            pixels: [100, 120, 140, 64].repeat(4),
        });
        let saved = glass::image(&original)?;
        let pose = Pose {
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
        };
        let camera = Camera {
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
        };
        let output = render(
            original,
            settings(),
            pose,
            camera,
            vec![],
            vec![],
            vec![0., 0., -10., 1., 0.5, 0.],
        )?;
        assert_eq!(glass::image(&output)?.pixels, [100, 60, 0, 64].repeat(4));
        assert_eq!(saved.pixels, [100, 120, 140, 64].repeat(4));
        glass::release(output);
        Ok(())
    }
}
