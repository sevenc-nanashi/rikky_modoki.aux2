use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};

use aviutl2::module::{AsScriptModuleUserData, ScriptModuleUserData};

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
