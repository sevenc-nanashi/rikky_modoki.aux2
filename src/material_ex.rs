//! 光源の軸を解決し、GPU用の定数へ変換する。
use crate::glass;
use aviutl2::module::FromScriptModuleParam;
fn unit(v: [f64; 3]) -> anyhow::Result<[f64; 3]> {
    let length = v[0].hypot(v[1]).hypot(v[2]);
    anyhow::ensure!(
        length > 0. && length.is_finite(),
        "Invalid material light axis"
    );
    Ok(v.map(|v| v / length))
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

pub fn constants(input: LightInput, textured: bool) -> anyhow::Result<Vec<f64>> {
    anyhow::ensure!((0..=2).contains(&input.kind), "Invalid material light type");
    let angles = [
        input.rx,
        input.ry,
        if input.kind == 1 { 0. } else { input.rz },
    ];
    let normal = if input.kind == 0 {
        [0., 0., -1.]
    } else if input.layer_rotation {
        glass::rotate([0., 0., -1.], angles)
    } else {
        unit([input.nx, input.ny, input.nz])?
    };
    let back = if input.kind != 1 || !input.double || input.layer_rotation {
        normal.map(|v| -v)
    } else {
        unit([
            input.nx2.unwrap_or(-normal[0]),
            input.ny2.unwrap_or(-normal[1]),
            input.nz2.unwrap_or(-normal[2]),
        ])?
    };
    let width = if input.kind != 2 {
        [1., 0., 0.]
    } else if input.layer_rotation {
        glass::rotate([1., 0., 0.], angles)
    } else {
        unit([input.wx, input.wy, input.wz])?
    };
    let height = if input.kind == 2 {
        anyhow::ensure!(
            width
                .iter()
                .zip(normal)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                .abs()
                < 1e-8,
            "Area light axes must be perpendicular"
        );
        anyhow::ensure!(
            input.width > 0 && input.height > 0 && input.partition > 0,
            "Invalid area light size or partition"
        );
        unit([
            width[1] * normal[2] - width[2] * normal[1],
            width[2] * normal[0] - width[0] * normal[2],
            width[0] * normal[1] - width[1] * normal[0],
        ])?
    } else {
        [0., 1., 0.]
    };
    let data = vec![
        input.x,
        input.y,
        input.z,
        f64::from(input.kind),
        input.r,
        input.g,
        input.b,
        input.shininess,
        input.specular_r,
        input.specular_g,
        input.specular_b,
        f64::from(input.double),
        normal[0],
        normal[1],
        normal[2],
        input.degree.to_radians().cos(),
        back[0],
        back[1],
        back[2],
        input.degree2.to_radians().cos(),
        width[0],
        width[1],
        width[2],
        input.width as f64,
        height[0],
        height[1],
        height[2],
        input.height as f64,
        input.partition as f64,
        input.alpha,
        f64::from(textured),
        0.,
    ];
    anyhow::ensure!(
        data.iter().all(|v| (*v as f32).is_finite()),
        "Light constants exceed GPU float range"
    );
    Ok(data)
}
