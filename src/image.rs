use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};

use aviutl2::module::{AsScriptModuleUserData, ScriptModuleUserData};
use windows::{Win32::Graphics::GdiPlus as gdip, core::GUID};

#[derive(Clone)]
struct Image {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

#[derive(Default)]
struct ImageStore {
    images: HashMap<String, Arc<Image>>,
    exported: HashMap<usize, Arc<Image>>,
}

// ponytail: 画像操作を単一Mutexで直列化。競合が問題になったらID単位に分割する。
static STORE: LazyLock<Mutex<ImageStore>> = LazyLock::new(Default::default);

pub struct ImageLease(Option<Arc<Image>>);
impl AsScriptModuleUserData for ImageLease {}

pub type ImageRead = (*const u8, usize, usize, ScriptModuleUserData<ImageLease>);

pub type FillAreaRead = (
    *const u8,
    usize,
    usize,
    ScriptModuleUserData<ImageLease>,
    Vec<usize>,
);

fn fill_component(pixel: &[u8], component: usize) -> f64 {
    let [r, g, b] = [pixel[0], pixel[1], pixel[2]].map(f64::from);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    match component {
        0 => f64::from(pixel[3]),
        1 if delta == 0.0 => 0.0,
        1 => {
            let sector = if max == r {
                (g - b) / delta
            } else if max == g {
                (b - r) / delta + 2.0
            } else {
                (r - g) / delta + 4.0
            };
            (sector * 60.0).rem_euclid(360.0)
        }
        2 if max == 0.0 => 0.0,
        2 => delta / max * 100.0,
        3 => max / 255.0 * 100.0,
        4 => 0.299 * r + 0.587 * g + 0.114 * b,
        _ => unreachable!("Invalid fill component"),
    }
}

fn fill_area(
    source: &[u8],
    width: usize,
    height: usize,
    start: [usize; 2],
    mode: usize,
    threshold: f64,
) -> anyhow::Result<Option<(Image, Vec<usize>)>> {
    anyhow::ensure!(mode <= 24, "Invalid fill mode");
    let component = mode / 5;
    let maximum = [255.0, 360.0, 100.0, 100.0, 255.0][component];
    anyhow::ensure!(
        (0.0..=maximum).contains(&threshold),
        "Invalid fill threshold"
    );
    let [x, y] = start;
    if x >= width || y >= height {
        return Ok(None);
    }
    let origin = y * width + x;
    let base = fill_component(&source[origin * 4..origin * 4 + 4], component);
    let matches = |index: usize| {
        let value = fill_component(&source[index * 4..index * 4 + 4], component);
        match mode % 5 {
            0 => value >= threshold,
            1 => value <= threshold,
            2 => value >= base,
            3 => value <= base,
            4 => value >= base - threshold && value <= base + threshold,
            _ => unreachable!(),
        }
    };
    if !matches(origin) {
        return Ok(None);
    }
    let mut pixels = vec![0; source.len()];
    // マスクのアルファを訪問済みフラグに兼用し、各選択画素を一度だけ積む。
    let mut pending = vec![origin];
    pixels[origin * 4..origin * 4 + 4].fill(255);
    let (mut left, mut top, mut right, mut bottom) = (x, y, x, y);
    while let Some(index) = pending.pop() {
        let (x, y) = (index % width, index / width);
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
        let neighbors = [
            x.checked_sub(1).map(|x| y * width + x),
            (x + 1 < width).then_some(index + 1),
            y.checked_sub(1).map(|y| y * width + x),
            (y + 1 < height).then_some(index + width),
        ];
        for neighbor in neighbors.into_iter().flatten() {
            if pixels[neighbor * 4 + 3] == 0 && matches(neighbor) {
                pixels[neighbor * 4..neighbor * 4 + 4].fill(255);
                pending.push(neighbor);
            }
        }
    }
    Ok(Some((
        Image {
            width,
            height,
            pixels,
        },
        vec![left, top, right - left + 1, bottom - top + 1],
    )))
}

/// data は width * height * 4 バイトの読み取り可能なRGBAデータを指すこと。
pub unsafe fn fillarea(
    data: *const u8,
    width: usize,
    height: usize,
    start: [usize; 2],
    mode: usize,
    threshold: f64,
) -> anyhow::Result<Option<FillAreaRead>> {
    let len = byte_len(width, height)?;
    anyhow::ensure!(!data.is_null(), "Image data is null");
    // SAFETY: Lua側でgetpixeldataの直後に呼び、出力の生成中は入力を変更しない。
    let source = unsafe { std::slice::from_raw_parts(data, len) };
    let Some((image, bounds)) = fill_area(source, width, height, start, mode, threshold)? else {
        return Ok(None);
    };
    let image = Arc::new(image);
    Ok(Some((
        image.pixels.as_ptr(),
        width,
        height,
        ImageLease(Some(image)).into(),
        bounds,
    )))
}

impl ImageStore {
    fn read(&mut self, image: Arc<Image>, export: bool) -> ImageRead {
        let data = image.pixels.as_ptr();
        if export {
            self.exported
                .entry(data as usize)
                .or_insert_with(|| image.clone());
        }
        (
            data,
            image.width,
            image.height,
            ImageLease(Some(image)).into(),
        )
    }
}

fn byte_len(width: usize, height: usize) -> anyhow::Result<usize> {
    anyhow::ensure!(
        width > 0 && height > 0 && width <= i32::MAX as usize && height <= i32::MAX as usize,
        "Invalid image dimensions"
    );
    width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(4))
        .filter(|n| *n <= isize::MAX as usize)
        .ok_or_else(|| anyhow::anyhow!("Image size overflow"))
}

fn export_path(file: &str, format: &str) -> anyhow::Result<std::path::PathBuf> {
    anyhow::ensure!(
        !file.is_empty() && !file.contains('\0'),
        "Invalid image filename"
    );
    let mut path = std::path::PathBuf::from(file);
    anyhow::ensure!(path.file_name().is_some(), "Expected an image filename");
    if !path
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case(format)
                || (format == "jpg" && ext.eq_ignore_ascii_case("jpeg"))
        })
    {
        path = path.with_added_extension(format);
    }
    if path.is_relative() {
        let executable = std::env::current_exe()?;
        let directory = executable
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Missing executable directory"))?;
        path = directory.join(path);
    }
    Ok(path)
}

struct ExportBitmap {
    token: usize,
    bitmap: *mut gdip::GpBitmap,
}

impl Drop for ExportBitmap {
    fn drop(&mut self) {
        // SAFETY: この保存処理が所有する画像を破棄してからGDI+を終了する。
        unsafe {
            if !self.bitmap.is_null() {
                gdip::GdipDisposeImage(self.bitmap.cast());
            }
            gdip::GdiplusShutdown(self.token);
        }
    }
}

/// data は width * height * 4 バイトの読み取り可能なRGBAデータを指すこと。
pub unsafe fn save_file(
    file: &str,
    format: &str,
    data: *const u8,
    width: usize,
    height: usize,
    mut quality: u32,
) -> anyhow::Result<()> {
    // Windows標準エンコーダーのCLSID。ファイルの拡張子とは独立して形式を指定する。
    let encoder = GUID::from_u128(match format {
        "png" => 0x557cf406_1a04_11d3_9a73_0000f81ef32e,
        "jpg" => 0x557cf401_1a04_11d3_9a73_0000f81ef32e,
        "bmp" => 0x557cf400_1a04_11d3_9a73_0000f81ef32e,
        _ => anyhow::bail!("Unknown image format: {format}"),
    });
    let len = byte_len(width, height)?;
    anyhow::ensure!(!data.is_null(), "Image data is null");
    anyhow::ensure!((1..=100).contains(&quality), "Invalid JPEG quality");
    let path = export_path(file, format)?;
    let png = format == "png";
    let channels = if png { 4 } else { 3 };
    let stride = (width * channels).next_multiple_of(4);
    let native_stride = i32::try_from(stride)?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(stride * height)?;
    pixels.resize(stride * height, 0);
    // SAFETY: getpixeldataのポインタが有効な間に、GDI+用のBGR(A)へコピーする。
    let rgba = unsafe { std::slice::from_raw_parts(data, len) };
    for (source, destination) in rgba
        .chunks_exact(width * 4)
        .zip(pixels.chunks_exact_mut(stride))
    {
        for (src, dst) in source
            .as_chunks::<4>()
            .0
            .iter()
            .zip(destination.chunks_exact_mut(channels))
        {
            if png {
                dst.copy_from_slice(&[src[2], src[1], src[0], src[3]]);
            } else {
                // 元のJPG/BMP出力と同じく、透明部分は黒背景に合成する。
                for (channel, value) in dst.iter_mut().zip([src[2], src[1], src[0]]) {
                    *channel = (u16::from(value) * u16::from(src[3]) / 255) as u8;
                }
            }
        }
    }
    let filename = windows::core::HSTRING::from(path.as_os_str());
    let input = gdip::GdiplusStartupInput {
        GdiplusVersion: 1,
        ..Default::default()
    };
    let mut token = 0;
    // SAFETY: 各構造体・画素・パスのメモリは保存完了まで保持する。
    unsafe {
        let status = gdip::GdiplusStartup(&mut token, &input, std::ptr::null_mut());
        anyhow::ensure!(status == gdip::Ok, "GDI+ startup failed: {status:?}");
        let mut image = ExportBitmap {
            token,
            bitmap: std::ptr::null_mut(),
        };
        // PixelFormat32bppARGB / PixelFormat24bppRGB（Windows SDKのマクロ）。
        let pixel_format = if png { 0x26200a } else { 0x21808 };
        let status = gdip::GdipCreateBitmapFromScan0(
            width as i32,
            height as i32,
            native_stride,
            pixel_format,
            Some(pixels.as_ptr()),
            &mut image.bitmap,
        );
        anyhow::ensure!(
            status == gdip::Ok,
            "Cannot create export bitmap: {status:?}"
        );
        let parameters = gdip::EncoderParameters {
            Count: 1,
            Parameter: [gdip::EncoderParameter {
                Guid: gdip::EncoderQuality,
                NumberOfValues: 1,
                Type: gdip::EncoderParameterValueTypeLong.0 as u32,
                Value: (&mut quality as *mut u32).cast(),
            }],
        };
        let parameters = if format == "jpg" {
            &parameters
        } else {
            std::ptr::null()
        };
        let status =
            gdip::GdipSaveImageToFile(image.bitmap.cast(), &filename, &encoder, parameters);
        anyhow::ensure!(
            status == gdip::Ok,
            "Cannot save image to {}: {status:?}",
            path.display()
        );
    }
    Ok(())
}

/// data は width * height * 4 バイトの読み取り可能なRGBAデータを指すこと。
pub unsafe fn write(
    id: String,
    data: *const u8,
    width: usize,
    height: usize,
    alpha: f64,
    only_empty: bool,
) -> anyhow::Result<bool> {
    let len = byte_len(width, height)?;
    anyhow::ensure!(!data.is_null(), "Image data is null");
    anyhow::ensure!(alpha.is_finite(), "Invalid image alpha");
    let mut store = STORE.lock().unwrap();
    if only_empty && store.images.contains_key(&id) {
        return Ok(false);
    }
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(len)?;
    // SAFETY: 呼び出し元が保証する入力を、ポインタが有効な間に所有メモリへコピーする。
    pixels.extend_from_slice(unsafe { std::slice::from_raw_parts(data, len) });
    if alpha != 1.0 {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel[3] = (f64::from(pixel[3]) * alpha.clamp(0.0, 1.0)).round() as u8;
        }
    }
    store.images.insert(
        id,
        Arc::new(Image {
            width,
            height,
            pixels,
        }),
    );
    Ok(true)
}

pub fn copy(destination: String, source: &str, only_empty: bool) -> bool {
    let mut store = STORE.lock().unwrap();
    if only_empty && store.images.contains_key(&destination) {
        return false;
    }
    let Some(image) = store.images.get(source).cloned() else {
        return false;
    };
    store.images.insert(destination, image);
    true
}

pub fn read(id: &str, export: bool) -> Option<ImageRead> {
    let mut store = STORE.lock().unwrap();
    let image = store.images.get(id)?.clone();
    Some(store.read(image, export))
}

pub fn release(lease: ScriptModuleUserData<ImageLease>) {
    lease.lock().unwrap().0.take();
}

pub fn delete(id: Option<&str>) -> bool {
    let mut store = STORE.lock().unwrap();
    if let Some(id) = id {
        store.images.remove(id).is_some()
    } else {
        store.images.clear();
        store.exported.clear();
        true
    }
}

pub fn ids(count: usize) -> anyhow::Result<Vec<String>> {
    let store = STORE.lock().unwrap();
    if count == 0 {
        return Ok(store.images.keys().cloned().collect());
    }
    let mut result = Vec::new();
    result.try_reserve_exact(count)?;
    for id in 0_u64.. {
        let key = format!("n:{id}");
        if !store.images.contains_key(&key) {
            result.push(key);
            if result.len() == count {
                break;
            }
        }
    }
    Ok(result)
}

pub fn merge(back: &str, front: &str, x: i32, y: i32, export: bool) -> Option<ImageRead> {
    let mut store = STORE.lock().unwrap();
    let back = store.images.get(back)?;
    let front = store.images.get(front)?;
    let mut result = (**back).clone();
    let (x, y) = (i64::from(x), i64::from(y));
    let left = x.max(0);
    let top = y.max(0);
    let right = (x + front.width as i64).min(back.width as i64);
    let bottom = (y + front.height as i64).min(back.height as i64);
    if left < right && top < bottom {
        let len = (right - left) as usize * 4;
        for row in top..bottom {
            let dst = (row as usize * back.width + left as usize) * 4;
            let src = ((row - y) as usize * front.width + (left - x) as usize) * 4;
            // アルファに関係なくRGBA全体を置換する。
            result.pixels[dst..dst + len].copy_from_slice(&front.pixels[src..src + len]);
        }
    }
    Some(store.read(Arc::new(result), export))
}

pub fn channels(
    lease: ScriptModuleUserData<ImageLease>,
) -> anyhow::Result<(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)> {
    let lease = lease.lock().unwrap();
    let image = lease
        .0
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Image already released"))?;
    let mut channels: [Vec<u8>; 4] =
        std::array::from_fn(|_| Vec::with_capacity(image.width * image.height));
    for pixel in image.pixels.chunks_exact(4) {
        for (channel, value) in channels.iter_mut().zip(pixel) {
            channel.push(*value);
        }
    }
    let [r, g, b, a] = channels;
    Ok((r, g, b, a))
}

/// data は width * height * 4 バイトの読み取り可能なRGBAデータを指すこと。
pub unsafe fn pixel(
    data: *const u8,
    width: usize,
    height: usize,
    index: f64,
) -> anyhow::Result<Option<(u8, u8, u8, u8)>> {
    let len = byte_len(width, height)?;
    anyhow::ensure!(!data.is_null(), "Image data is null");
    anyhow::ensure!(
        index.is_finite() && index.fract() == 0.0,
        "Invalid pixel index"
    );
    if index < 0.0 || index >= (len / 4) as f64 {
        return Ok(None);
    }
    // SAFETY: 入力バッファの契約と上の範囲検査により、この4バイトを読み取れる。
    let rgba = unsafe { std::slice::from_raw_parts(data.add(index as usize * 4), 4) };
    Ok(Some((rgba[0], rgba[1], rgba[2], rgba[3])))
}

/// data は width * height * 4 バイトの読み取り可能なRGBAデータを指すこと。
/// 輪郭ごとのピクセル位置を連結した配列と、各輪郭の点数を返す。
pub unsafe fn bordering(
    data: *const u8,
    width: usize,
    height: usize,
    skip: usize,
    threshold: f64,
    hq: bool,
) -> anyhow::Result<(Vec<usize>, Vec<usize>)> {
    anyhow::ensure!(
        (0.0..=100.0).contains(&threshold),
        "Invalid alpha threshold"
    );
    if width == 0 || height == 0 {
        return Ok((Vec::new(), Vec::new()));
    }
    let len = byte_len(width, height)?;
    anyhow::ensure!(!data.is_null(), "Image data is null");
    // SAFETY: getpixeldataのポインタが有効な間に輪郭を抽出する。
    let pixels = unsafe { std::slice::from_raw_parts(data, len) };
    let threshold = ((threshold * 2.55) as u8).min(254);
    let opaque = |index: usize| pixels[index * 4 + 3] > threshold;
    // 左上から反時計回り。外周と穴の向きは最初に探す方向で決まる。
    let directions = [
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
        (1, 0),
        (1, -1),
        (0, -1),
    ];
    let neighbor = |index: usize, direction: usize| {
        let (dx, dy) = directions[direction];
        let x = (index % width).checked_add_signed(dx)?;
        let y = (index / width).checked_add_signed(dy)?;
        (x < width && y < height).then_some(y * width + x)
    };
    let mut boundary: Vec<bool> = (0..len / 4)
        .map(|index| {
            opaque(index)
                && [1, 3, 5, 7]
                    .into_iter()
                    .any(|direction| neighbor(index, direction).is_none_or(|next| !opaque(next)))
        })
        .collect();
    let mut visited = vec![0_u8; boundary.len()];
    let mut contour = Vec::new();
    let mut points = Vec::new();
    let mut counts = Vec::new();
    for start in 0..boundary.len() {
        if !boundary[start] {
            continue;
        }
        contour.clear();
        contour.push(start);
        let mut current = start;
        let mut direction = if neighbor(start, 7).is_none_or(|above| !opaque(above)) {
            2
        } else {
            4
        };
        loop {
            // 枝分かれした輪郭でも、同じ位置・方向の巡回で停止できるようにする。
            if visited[current] & (1 << direction) != 0 {
                break;
            }
            visited[current] |= 1 << direction;
            let Some((mut next, found)) = (0..8).find_map(|offset| {
                let candidate = (direction + offset) % 8;
                let next = neighbor(current, candidate)?;
                boundary[next].then_some((next, candidate))
            }) else {
                break;
            };
            direction = (found + 6 + found % 2) % 8;
            if hq && found % 2 == 0 {
                let adjacent = neighbor(current, (found + 1) % 8).unwrap();
                if opaque(adjacent) {
                    next = adjacent;
                    direction = (found + 7) % 8;
                }
            }
            if next == start {
                break;
            }
            contour.push(next);
            current = next;
        }
        // 元の実装と同じく、間引き前に5点以上ある輪郭だけを返す。
        if contour.len() > 4 {
            let before = points.len();
            points.extend(contour.iter().step_by(skip.min(5000) + 1).copied());
            counts.push(points.len() - before);
        }
        for &index in &contour {
            boundary[index] = false;
            visited[index] = 0;
        }
    }
    Ok((points, counts))
}

/// data は width * height * 4 バイトの読み取り可能なRGBAデータを指すこと。
pub unsafe fn linedetection(
    data: *const u8,
    width: usize,
    height: usize,
    scale: f64,
    background: u32,
) -> anyhow::Result<Vec<f64>> {
    anyhow::ensure!(scale.is_finite() && scale > 0.0, "Invalid detection scale");
    anyhow::ensure!(background <= 0xffffff, "Invalid background color");
    if width == 0 || height == 0 {
        return Ok(Vec::new());
    }
    let len = byte_len(width, height)?;
    anyhow::ensure!(!data.is_null(), "Image data is null");
    let scaled_width = (width as f64 * scale).ceil();
    let scaled_height = (height as f64 * scale).ceil();
    anyhow::ensure!(
        scaled_width <= (i32::MAX - 8) as f64
            && scaled_height <= (i32::MAX - 8) as f64
            && scaled_width * scaled_height <= i32::MAX as f64,
        "Scaled image is too large"
    );
    // 検出器は周囲3pxを除外するため、幅または高さが6px以下なら線分はない。
    if scaled_width <= 6.0 || scaled_height <= 6.0 {
        return Ok(Vec::new());
    }
    // SAFETY: getpixeldataのポインタが有効な間に入力画像を参照する。
    let pixels = unsafe { std::slice::from_raw_parts(data, len) };
    let base = 0.299 * f64::from((background >> 16) & 255)
        + 0.587 * f64::from((background >> 8) & 255)
        + 0.114 * f64::from(background & 255);
    let gray = |x: usize, y: usize| {
        let p = &pixels[(y * width + x) * 4..][..4];
        base + (0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2]) - base)
            * f64::from(p[3])
            / 255.0
    };
    let (scaled_width, scaled_height) = (scaled_width as usize, scaled_height as usize);
    let mut resized = Vec::new();
    resized.try_reserve_exact(scaled_width * scaled_height)?;
    for y in 0..scaled_height {
        let sy = (y as f64 / scale).min((height - 1) as f64);
        let y0 = sy as usize;
        let y1 = (y0 + 1).min(height - 1);
        let fy = sy - y0 as f64;
        for x in 0..scaled_width {
            let sx = (x as f64 / scale).min((width - 1) as f64);
            let x0 = sx as usize;
            let x1 = (x0 + 1).min(width - 1);
            let fx = sx - x0 as f64;
            let top = gray(x0, y0) * (1.0 - fx) + gray(x1, y0) * fx;
            let bottom = gray(x0, y1) * (1.0 - fx) + gray(x1, y1) * fx;
            resized.push((top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8);
        }
    }
    Ok(sweeplsd::detect(&resized, scaled_width, scaled_height)?
        .into_iter()
        .flatten()
        .map(|coordinate| f64::from(coordinate) / scale)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_modes_include_boundaries_and_keep_the_seed_reference() -> anyhow::Result<()> {
        // 各成分が左から 0, 中間値, 最大値になる画像。
        let rows = [
            [[0, 0, 0, 0], [0, 0, 0, 128], [0, 0, 0, 255]],
            [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]],
            [[255, 255, 255, 255], [255, 128, 128, 255], [255, 0, 0, 255]],
            [[0, 0, 0, 255], [128, 128, 128, 255], [255, 255, 255, 255]],
            [[0, 0, 0, 255], [128, 128, 128, 255], [255, 255, 255, 255]],
        ];
        for (component, row) in rows.iter().enumerate() {
            let source = row.concat();
            let base = fill_component(&row[1], component);
            for comparison in 0..5 {
                let mode = component * 5 + comparison;
                let threshold = if comparison == 4 { 0.0 } else { base };
                let (image, bounds) = fill_area(&source, 3, 1, [1, 0], mode, threshold)?.unwrap();
                let expected = match comparison {
                    0 | 2 => [false, true, true],
                    1 | 3 => [true, true, false],
                    _ => [false, true, false],
                };
                for (pixel, selected) in image.pixels.chunks_exact(4).zip(expected) {
                    assert_eq!(
                        pixel,
                        if selected { &[255; 4] } else { &[0; 4] },
                        "mode {mode}"
                    );
                }
                assert_eq!(
                    bounds,
                    match comparison {
                        0 | 2 => vec![1, 0, 2, 1],
                        1 | 3 => vec![0, 0, 2, 1],
                        _ => vec![1, 0, 1, 1],
                    }
                );
            }
            assert!(fill_area(&source, 3, 1, [0, 0], component * 5, base)?.is_none());
            assert!(fill_area(&source, 3, 1, [2, 0], component * 5 + 1, base)?.is_none());
        }
        let source = [[0, 0, 0, 100], [0, 0, 0, 110], [0, 0, 0, 120]].concat();
        let (_, bounds) = fill_area(&source, 3, 1, [0, 0], 4, 10.0)?.unwrap();
        assert_eq!(bounds, vec![0, 0, 2, 1]);
        assert_eq!(fill_component(&[255, 0, 255, 0], 1), 300.0);
        assert_eq!(fill_component(&[0, 0, 0, 0], 2), 0.0);
        assert_eq!(fill_component(&[255, 0, 0, 0], 4), 0.299 * 255.0);
        Ok(())
    }

    #[test]
    fn fill_connectivity_bounds_and_failures() -> anyhow::Result<()> {
        let source: Vec<u8> = [
            255, 255, 255, 0, 0, 255, 0, 255, 0, 0, 255, 255, 255, 0, 0, 0, 0, 0, 255, 0,
        ]
        .into_iter()
        .flat_map(|alpha| [23, 45, 67, alpha])
        .collect();
        let original = source.clone();
        let (mask, bounds) = fill_area(&source, 5, 4, [2, 2], 0, 255.0)?.unwrap();
        assert_eq!(bounds, vec![0, 0, 3, 3]);
        assert_eq!(
            mask.pixels.chunks_exact(4).filter(|p| p[3] == 255).count(),
            8
        );
        assert_eq!(&mask.pixels[24..28], &[0; 4]); // 穴
        assert_eq!(&mask.pixels[72..76], &[0; 4]); // 斜めに接する独立領域
        assert!(fill_area(&source, 5, 4, [5, 0], 0, 0.0)?.is_none());
        assert!(fill_area(&source, 5, 4, [0, 4], 0, 0.0)?.is_none());
        assert!(fill_area(&[], 0, 0, [0, 0], 0, 0.0)?.is_none());
        for (mode, threshold) in [(25, 0.0), (0, -1.0), (5, 361.0), (10, 101.0), (0, f64::NAN)] {
            assert!(fill_area(&source, 5, 4, [0, 0], mode, threshold).is_err());
        }
        assert_eq!(source, original);
        let (_, bounds) = fill_area(&[1, 2, 3, 0], 1, 1, [0, 0], 1, 0.0)?.unwrap();
        assert_eq!(bounds, vec![0, 0, 1, 1]);
        let (mask, bounds) =
            fill_area(&vec![255; 512 * 512 * 4], 512, 512, [511, 511], 0, 255.0)?.unwrap();
        assert_eq!(bounds, vec![0, 0, 512, 512]);
        assert!(mask.pixels.iter().all(|&byte| byte == 255));
        Ok(())
    }

    #[test]
    fn exports_images_with_compatible_paths_colors_and_quality() -> anyhow::Result<()> {
        let executable = std::env::current_exe()?;
        let directory = executable.parent().unwrap();
        for (file, format, expected) in [
            ("画像", "png", "画像.png"),
            ("画像.PNG", "png", "画像.PNG"),
            ("画像.bmp", "png", "画像.bmp.png"),
            ("画像.JPEG", "jpg", "画像.JPEG"),
            ("画像.JPG", "jpg", "画像.JPG"),
            ("画像.BMP", "bmp", "画像.BMP"),
        ] {
            assert_eq!(export_path(file, format)?, directory.join(expected));
        }
        let folder = directory.join(format!("__gi_image_export_{}", std::process::id()));
        std::fs::create_dir(&folder)?;
        let file = folder.join("画像");
        let file = file.to_str().unwrap();
        // 幅3でBGRの行パディングも検証する。上段は不透明・半透明・透明。
        let pixels = [
            255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 13, 31, 73, 255, 254, 127, 63, 254, 99,
            199, 255, 1,
        ];
        unsafe {
            for format in ["png", "bmp", "jpg"] {
                save_file(file, format, pixels.as_ptr(), 3, 2, 100)?;
                let path = export_path(file, format)?;
                let bytes = std::fs::read(&path)?;
                let signature: &[u8] = match format {
                    "png" => b"\x89PNG\r\n\x1a\n",
                    "bmp" => b"BM",
                    _ => b"\xff\xd8\xff",
                };
                assert!(bytes.starts_with(signature));
                if format == "bmp" {
                    assert_eq!(u16::from_le_bytes(bytes[28..30].try_into()?), 24);
                }
                let mut token = 0;
                let input = gdip::GdiplusStartupInput {
                    GdiplusVersion: 1,
                    ..Default::default()
                };
                assert_eq!(
                    gdip::GdiplusStartup(&mut token, &input, std::ptr::null_mut()),
                    gdip::Ok
                );
                let mut image = ExportBitmap {
                    token,
                    bitmap: std::ptr::null_mut(),
                };
                let filename = windows::core::HSTRING::from(path.as_os_str());
                assert_eq!(
                    gdip::GdipCreateBitmapFromFile(&filename, &mut image.bitmap),
                    gdip::Ok
                );
                let (mut width, mut height) = (0, 0);
                assert_eq!(
                    gdip::GdipGetImageWidth(image.bitmap.cast(), &mut width),
                    gdip::Ok
                );
                assert_eq!(
                    gdip::GdipGetImageHeight(image.bitmap.cast(), &mut height),
                    gdip::Ok
                );
                assert_eq!((width, height), (3, 2));
                if format != "jpg" {
                    let expected = if format == "png" {
                        [
                            0xffff0000, 0x8000ff00, 0x000000ff, 0xff0d1f49, 0xfefe7f3f, 0x0163c7ff,
                        ]
                    } else {
                        [
                            0xffff0000, 0xff008000, 0xff000000, 0xff0d1f49, 0xfffd7e3e, 0xff000001,
                        ]
                    };
                    for (i, expected) in expected.into_iter().enumerate() {
                        let mut color = 0;
                        assert_eq!(
                            gdip::GdipBitmapGetPixel(
                                image.bitmap,
                                (i % 3) as i32,
                                (i / 3) as i32,
                                &mut color
                            ),
                            gdip::Ok
                        );
                        assert_eq!(color, expected, "{format} pixel {i}");
                    }
                }
            }
            let best = std::fs::read(export_path(file, "jpg")?)?;
            save_file(file, "jpg", pixels.as_ptr(), 3, 2, 1)?;
            assert_ne!(std::fs::read(export_path(file, "jpg")?)?, best);
            for (file, format, width, height, quality) in [
                ("", "png", 3, 2, 100),
                ("invalid\0name", "png", 3, 2, 100),
                (file, "gif", 3, 2, 100),
                (file, "png", 0, 2, 100),
                (file, "png", i32::MAX as usize, 2, 100),
                (file, "jpg", 3, 2, 101),
            ] {
                assert!(save_file(file, format, pixels.as_ptr(), width, height, quality).is_err());
            }
            assert!(save_file(file, "png", std::ptr::null(), 3, 2, 100).is_err());
            let missing = folder.join("missing").join("画像.png");
            assert!(
                save_file(missing.to_str().unwrap(), "png", pixels.as_ptr(), 3, 2, 100).is_err()
            );
            // 失敗後も保存でき、既存ファイルも上書きできる。
            save_file(file, "png", pixels.as_ptr(), 3, 2, 100)?;
        }
        std::fs::remove_dir_all(folder)?;
        Ok(())
    }
}
