use aviutl2::{common::Rational32, filter::FilterConfigItems};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex, Weak},
};

const NAME: &str = "オーディオバッファ@rikky_modoki.aux2";

// 両方の生成ボタンの開始判定を直列化する。完了通知を待つ間は保持しない。
pub static RENDER_START: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Format {
    fps: Rational32,
    sample_rate: u32,
}

impl Format {
    fn sample_at(self, frame: i64) -> i64 {
        let numerator =
            i128::from(frame) * i128::from(self.sample_rate) * i128::from(*self.fps.denom());
        let denominator = i128::from(*self.fps.numer());
        ((numerator + denominator / 2).div_euclid(denominator)) as i64
    }

    fn sample_count(self, frame: i64) -> usize {
        (self.sample_at(frame + 1) - self.sample_at(frame)) as usize
    }
}

struct BakedAudio {
    scene: i32,
    revision: u64,
    generation: u64,
    format: Format,
    start: u32,
    // SDKの返す実際のサンプル数を保持する（フレームごとに異なる）。
    frames: Vec<[Vec<f32>; 2]>,
}

#[derive(Default)]
struct SavedAudio {
    discard_on_update: bool,
    audio: Option<Arc<BakedAudio>>,
}

type Destination = Arc<Mutex<SavedAudio>>;
type SceneBuffers = HashMap<i64, Weak<Mutex<SavedAudio>>>;

struct Capture {
    id: u64,
    scene: i32,
    effect: i64,
    end: u32,
    frame: u32,
    audio: BakedAudio,
    destination: Option<Weak<Mutex<SavedAudio>>>,
    progress: crate::progress::Progress,
}

#[derive(Default)]
struct Store {
    revision: u64,
    reset_revision: u64,
    next_id: u64,
    scenes: HashMap<i32, SceneBuffers>,
    capture: Option<Capture>,
}

static STORE: LazyLock<Mutex<Store>> = LazyLock::new(Default::default);

impl Store {
    fn invalidate(&mut self) {
        self.revision += 1;
        if let Some(capture) = &self.capture {
            capture.progress.state.cancel();
        }
    }

    fn snapshots(&mut self, scene: i32) -> Vec<Arc<BakedAudio>> {
        let mut snapshots = Vec::new();
        if let Some(buffers) = self.scenes.get_mut(&scene) {
            buffers.retain(|_, weak| {
                let Some(destination) = weak.upgrade() else {
                    return false;
                };
                let mut saved = destination.lock().unwrap();
                if saved.audio.as_ref().is_some_and(|audio| {
                    audio.revision < self.reset_revision
                        || (saved.discard_on_update && audio.revision != self.revision)
                }) {
                    saved.audio = None;
                }
                if let Some(audio) = &saved.audio
                    && audio.scene == scene
                {
                    snapshots.push(Arc::clone(audio));
                }
                true
            });
        }
        snapshots
    }
}

pub fn busy() -> bool {
    STORE.lock().unwrap().capture.is_some()
}

pub fn invalidate() {
    STORE.lock().unwrap().invalidate();
}

pub fn reset() {
    let mut store = STORE.lock().unwrap();
    store.invalidate();
    store.reset_revision = store.revision;
    for buffers in store.scenes.values() {
        for weak in buffers.values() {
            if let Some(destination) = weak.upgrade() {
                destination.lock().unwrap().audio = None;
            }
        }
    }
    store.scenes.clear();
}

pub fn shutdown() {
    let capture = STORE.lock().unwrap().capture.take();
    drop(capture);
}

pub fn info() -> (u32, u32) {
    (2, crate::EDIT_HANDLE.get_edit_info().sample_rate as u32)
}

pub fn pcm(frame: i64, size: Option<usize>) -> anyhow::Result<(Vec<f64>, Vec<f64>)> {
    anyhow::ensure!(
        (i64::from(i32::MIN)..i64::from(i32::MAX)).contains(&frame),
        "Audio frame is out of range"
    );
    let info = crate::EDIT_HANDLE.get_edit_info();
    let format = Format {
        fps: info.fps,
        sample_rate: info.sample_rate as u32,
    };
    let snapshots = if frame < 0 || frame > info.frame_max as i64 {
        Vec::new()
    } else {
        STORE.lock().unwrap().snapshots(info.scene_id)
    };
    read_pcm(&snapshots, format, frame, size)
}

fn read_pcm(
    snapshots: &[Arc<BakedAudio>],
    format: Format,
    frame: i64,
    size: Option<usize>,
) -> anyhow::Result<(Vec<f64>, Vec<f64>)> {
    let selected = snapshots
        .iter()
        .filter(|audio| {
            frame >= i64::from(audio.start)
                && frame - i64::from(audio.start) < audio.frames.len() as i64
        })
        .max_by_key(|audio| audio.generation);
    let samples = if let Some(audio) = selected {
        anyhow::ensure!(
            audio.format == format,
            "シーンの音声設定が変更されています。「範囲内の音声をレンダリング」を押してください"
        );
        Some(&audio.frames[(frame - i64::from(audio.start)) as usize])
    } else {
        None
    };
    let count = match samples {
        Some(samples) => samples[0].len(),
        None => format.sample_count(frame),
    };
    let size = match size {
        Some(size) => {
            anyhow::ensure!(
                size > 0 && size <= count,
                "PCM size must be between 1 and {count}"
            );
            size
        }
        None => count,
    };
    let mut result = (vec![0.0; size], vec![0.0; size]);
    if let Some(samples) = samples {
        for i in 0..size {
            let index = i * count / size;
            result.0[i] = f64::from(samples[0][index]);
            result.1[i] = f64::from(samples[1][index]);
        }
    }
    Ok(result)
}

#[aviutl2::plugin(FilterPlugin)]
pub struct AudioBufferAuf2;

#[aviutl2::filter::filter_config_items]
struct AudioBufferConfig {
    #[check(name = "オブジェクトの更新後に音声を破棄", default = false)]
    discard_on_update: bool,
    #[button(name = "範囲内の音声をレンダリング")]
    render_audio: fn(),
}

fn render_audio(
    edit: &mut aviutl2::generic::EditSection,
    _object: aviutl2::generic::ObjectHandle,
    _effect_name: String,
    _effect_index: usize,
    _item: String,
) -> aviutl2::common::AnyResult<()> {
    let _start = RENDER_START.lock().unwrap();
    anyhow::ensure!(
        !busy() && !crate::objectsound::busy(),
        "音声を生成中です（中止後は完了通知を待ってください）"
    );
    anyhow::ensure!(
        crate::EDIT_HANDLE.get_edit_state()? == aviutl2::generic::EditState::Edit,
        "再生・出力を停止してから音声をレンダリングしてください"
    );
    let object = edit
        .get_focused_object()?
        .ok_or_else(|| anyhow::anyhow!("No focused object"))?;
    let effect = edit.get_first_effect(object)?;
    anyhow::ensure!(
        edit.get_effect_name(effect)? == NAME,
        "オーディオバッファを選択してください"
    );
    let effect = edit.get_effect_id(effect)?;
    let range = edit.get_object_layer_frame(object)?;
    let start = u32::try_from(range.start)?;
    let end = u32::try_from(range.end)?;
    let mut frames = Vec::new();
    frames.try_reserve_exact((end - start + 1) as usize)?;
    let owner = crate::EDIT_HANDLE
        .get_host_app_window_raw()
        .ok_or_else(|| anyhow::anyhow!("AviUtl2のウィンドウを取得できません"))?;
    let progress =
        crate::progress::Progress::new("範囲内の音声をレンダリング".into(), 0x0078d4, owner.hwnd)?;
    let id = {
        let mut store = STORE.lock().unwrap();
        store.next_id += 1;
        let id = store.next_id;
        store.capture = Some(Capture {
            id,
            scene: edit.info.scene_id,
            effect,
            end,
            frame: start,
            audio: BakedAudio {
                scene: edit.info.scene_id,
                revision: store.revision,
                generation: id,
                format: Format {
                    fps: edit.info.fps,
                    sample_rate: edit.info.sample_rate as u32,
                },
                start,
                frames,
            },
            destination: None,
            progress,
        });
        id
    };
    // 編集ロック中には待機しない。最初の音声処理でUserdataへ結び付ける。
    request_frame(id, start)?;
    Ok(())
}

fn cancel(id: u64) {
    let capture = {
        let mut store = STORE.lock().unwrap();
        if store
            .capture
            .as_ref()
            .is_some_and(|capture| capture.id == id)
        {
            store.capture.take()
        } else {
            None
        }
    };
    // ProgressのUIスレッドのjoinはSTOREのロック外で行う。
    drop(capture);
}

fn request_frame(id: u64, frame: u32) -> anyhow::Result<()> {
    let current_scene = crate::EDIT_HANDLE.get_edit_info().scene_id;
    let scene = {
        let store = STORE.lock().unwrap();
        let Some(capture) = store.capture.as_ref().filter(|capture| capture.id == id) else {
            return Ok(());
        };
        if !capture.progress.state.running() || current_scene != capture.scene {
            drop(store);
            cancel(id);
            return Ok(());
        }
        capture.scene
    };
    let result = crate::EDIT_HANDLE.rendering_scene_audio(frame, move |audio| {
        if crate::EDIT_HANDLE.get_edit_info().scene_id != scene {
            cancel(id);
            return;
        }
        let result =
            finish_frame(id, audio.frame, audio.buffer0, audio.buffer1).and_then(
                |next| match next {
                    Some(frame) => request_frame(id, frame),
                    None => Ok(()),
                },
            );
        if let Err(error) = result {
            cancel(id);
            tracing::error!("オーディオバッファの生成に失敗しました: {error:#}");
        }
    });
    if result.is_err() {
        cancel(id);
    }
    Ok(result?)
}

fn finish_frame(id: u64, frame: u32, left: &[f32], right: &[f32]) -> anyhow::Result<Option<u32>> {
    let mut store = STORE.lock().unwrap();
    let Some(capture) = store.capture.as_mut().filter(|capture| capture.id == id) else {
        return Ok(None);
    };
    if !capture.progress.state.running() {
        drop(store);
        cancel(id);
        return Ok(None);
    }
    anyhow::ensure!(!left.is_empty(), "音声のレンダリングに失敗しました");
    let destination = capture
        .destination
        .as_ref()
        .and_then(Weak::upgrade)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "対象の音声が処理されませんでした。レイヤーの表示状態を確認してください"
            )
        })?;
    capture.audio.frames.push([left.to_vec(), right.to_vec()]);
    if frame == capture.end {
        if !capture.progress.state.update(100.0) {
            drop(store);
            cancel(id);
            return Ok(None);
        }
        let capture = store.capture.take().unwrap();
        destination.lock().unwrap().audio = Some(Arc::new(capture.audio));
        drop(store);
        drop(capture.progress);
        tracing::info!("オーディオバッファの生成が完了しました");
        Ok(None)
    } else {
        capture.progress.state.update(
            f64::from(frame - capture.audio.start + 1)
                / f64::from(capture.end - capture.audio.start + 1)
                * 100.0,
        );
        capture.frame += 1;
        let next = capture.frame;
        drop(store);
        Ok(Some(next))
    }
}

#[derive(Default)]
pub struct AudioBufferData {
    saved: Destination,
}

impl aviutl2::filter::FilterUserdata for AudioBufferData {
    fn new(_effect_id: i64) -> Self {
        Self::default()
    }
}

impl aviutl2::filter::FilterPlugin for AudioBufferAuf2 {
    type Userdata = AudioBufferData;

    fn new(_info: aviutl2::common::AviUtl2Info) -> aviutl2::common::AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::filter::FilterPluginTable {
        aviutl2::filter::FilterPluginTable {
            name: NAME.into(),
            information: "audio_buffer.auf2 @ rikky_modoki.aux2".into(),
            label: Some("@rikky_modoki.aux2".into()),
            flags: aviutl2::bitflag!(aviutl2::filter::FilterPluginFlags {
                audio: true,
                input: true,
            }),
            config_items: AudioBufferConfig::to_config_items(),
        }
    }

    fn proc_audio(
        &self,
        configs: &[aviutl2::filter::FilterConfigItem],
        audio: &mut aviutl2::filter::FilterProcAudio<Self::Userdata>,
    ) -> aviutl2::common::AnyResult<()> {
        let config = AudioBufferConfig::from_config_items(configs);
        let data = audio.userdata.read();
        let mut store = STORE.lock().unwrap();
        data.saved.lock().unwrap().discard_on_update = config.discard_on_update;
        if let Some(capture) = store.capture.as_mut()
            && capture.effect == audio.object.effect_id
            && capture.progress.state.running()
        {
            let weak = Arc::downgrade(&data.saved);
            capture.destination = Some(weak.clone());
            let scene = capture.scene;
            store
                .scenes
                .entry(scene)
                .or_default()
                .insert(audio.object.effect_id, weak);
        }
        drop(store);
        drop(data);
        let silence = vec![0.0; audio.audio_object.sample_num as usize];
        audio.set_sample_data(aviutl2::filter::AudioChannel::Left, &silence);
        audio.set_sample_data(aviutl2::filter::AudioChannel::Right, &silence);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format() -> Format {
        Format {
            fps: Rational32::new(30, 1),
            sample_rate: 120,
        }
    }

    fn baked(generation: u64, start: u32, frames: Vec<[Vec<f32>; 2]>) -> Arc<BakedAudio> {
        Arc::new(BakedAudio {
            scene: 1,
            revision: 0,
            generation,
            format: format(),
            start,
            frames,
        })
    }

    #[test]
    fn combines_ranges_without_mixing_and_uses_actual_sample_counts() {
        let old = baked(1, 10, vec![[vec![0.25; 4], vec![0.5; 4]]; 2]);
        let recent = baked(2, 11, vec![[vec![0.0, 0.25, 0.5, 0.75, 1.0], vec![1.0; 5]]]);
        let separate = baked(3, 13, vec![[vec![-0.5; 4], vec![0.5; 4]]]);
        let snapshots = [recent, old, separate]; // 登録順には依存しない。
        assert_eq!(
            read_pcm(&snapshots, format(), 10, None).unwrap(),
            (vec![0.25; 4], vec![0.5; 4])
        );
        assert_eq!(
            read_pcm(&snapshots, format(), 11, Some(2)).unwrap(),
            (vec![0.0, 0.5], vec![1.0; 2])
        );
        assert_eq!(read_pcm(&snapshots, format(), 11, None).unwrap().0.len(), 5);
        assert_eq!(
            read_pcm(&snapshots, format(), 13, None).unwrap().0,
            vec![-0.5; 4]
        );
        for frame in [-1, 0, 9, 12, 14] {
            assert_eq!(
                read_pcm(&snapshots, format(), frame, None).unwrap(),
                (vec![0.0; 4], vec![0.0; 4])
            );
        }
        assert!(read_pcm(&snapshots, format(), 11, Some(0)).is_err());
        assert!(read_pcm(&snapshots, format(), 11, Some(6)).is_err());
        let changed = Format {
            sample_rate: 240,
            ..format()
        };
        assert!(read_pcm(&snapshots, changed, 11, None).is_err());
        let fractional = Format {
            fps: Rational32::new(30_000, 1_001),
            sample_rate: 48_000,
        };
        assert_eq!(
            (fractional.sample_count(0), fractional.sample_count(1)),
            (1602, 1601)
        );
        assert_eq!(
            (101..201)
                .map(|frame| fractional.sample_count(frame))
                .sum::<usize>(),
            (fractional.sample_at(201) - fractional.sample_at(101)) as usize
        );
        assert_eq!(read_pcm(&[], fractional, -1, None).unwrap().0.len(), 1602);
    }

    #[test]
    fn isolates_scenes_and_tracks_userdata_lifetime_and_discard_settings() {
        let mut store = Store::default();
        let data = AudioBufferData::default();
        let sound = baked(1, 0, vec![[vec![0.25; 4], vec![0.5; 4]]]);
        data.saved.lock().unwrap().audio = Some(Arc::clone(&sound));
        store
            .scenes
            .entry(1)
            .or_default()
            .insert(7, Arc::downgrade(&data.saved));
        assert!(store.snapshots(2).is_empty());
        // 同じUserdataを別シーンへ再登録しても以前の索引から音声を取り違えない。
        store
            .scenes
            .entry(2)
            .or_default()
            .insert(7, Arc::downgrade(&data.saved));
        assert!(store.snapshots(2).is_empty());
        assert_eq!(store.snapshots(1).len(), 1);
        store.invalidate();
        assert_eq!(store.snapshots(1).len(), 1); // 既定では編集しても保持する。
        data.saved.lock().unwrap().discard_on_update = true;
        assert!(store.snapshots(1).is_empty());
        {
            let mut saved = data.saved.lock().unwrap();
            saved.discard_on_update = false;
            saved.audio = Some(sound);
        }
        store.reset_revision = store.revision;
        assert!(store.snapshots(1).is_empty());
        drop(data);
        assert!(store.snapshots(1).is_empty());
        assert!(store.scenes[&1].is_empty());
    }

    fn capture(id: u64, destination: &Destination) -> Capture {
        Capture {
            id,
            scene: 1,
            effect: 7,
            frame: 10,
            end: 11,
            audio: BakedAudio {
                scene: 1,
                revision: 0,
                generation: id,
                format: format(),
                start: 10,
                frames: Vec::new(),
            },
            destination: Some(Arc::downgrade(destination)),
            progress: crate::progress::Progress::headless(),
        }
    }

    #[test]
    fn commits_only_complete_jobs_and_ignores_late_callbacks() {
        let destination = Arc::new(Mutex::new(SavedAudio::default()));
        let previous = baked(1, 10, vec![[vec![0.25; 4], vec![0.5; 4]]; 2]);
        destination.lock().unwrap().audio = Some(Arc::clone(&previous));
        STORE.lock().unwrap().capture = Some(capture(2, &destination));
        assert!(busy());
        assert_eq!(
            finish_frame(2, 10, &[0.5; 4], &[0.75; 4]).unwrap(),
            Some(11)
        );
        assert!(Arc::ptr_eq(
            destination.lock().unwrap().audio.as_ref().unwrap(),
            &previous
        ));
        assert_eq!(finish_frame(1, 11, &[1.0; 4], &[1.0; 4]).unwrap(), None);
        cancel(1);
        assert!(busy());
        STORE
            .lock()
            .unwrap()
            .capture
            .as_ref()
            .unwrap()
            .progress
            .state
            .cancel();
        assert!(busy()); // キャンセルしても通知までジョブを保持する。
        finish_frame(2, 11, &[], &[]).unwrap();
        assert!(!busy());
        assert!(Arc::ptr_eq(
            destination.lock().unwrap().audio.as_ref().unwrap(),
            &previous
        ));

        STORE.lock().unwrap().capture = Some(capture(3, &destination));
        assert!(finish_frame(3, 10, &[], &[]).is_err());
        cancel(3);
        assert!(Arc::ptr_eq(
            destination.lock().unwrap().audio.as_ref().unwrap(),
            &previous
        ));

        STORE.lock().unwrap().capture = Some(capture(4, &destination));
        finish_frame(4, 10, &[0.5; 4], &[0.75; 4]).unwrap();
        assert_eq!(finish_frame(4, 11, &[1.0; 5], &[-1.0; 5]).unwrap(), None);
        assert!(!busy());
        let saved = destination.lock().unwrap().audio.clone().unwrap();
        assert_eq!(saved.generation, 4);
        assert_eq!(
            saved.frames,
            vec![[vec![0.5; 4], vec![0.75; 4]], [vec![1.0; 5], vec![-1.0; 5]]]
        );

        STORE.lock().unwrap().capture = Some(capture(5, &destination));
        invalidate();
        finish_frame(5, 10, &[1.0; 4], &[1.0; 4]).unwrap();
        assert!(!busy());
        assert!(Arc::ptr_eq(
            destination.lock().unwrap().audio.as_ref().unwrap(),
            &saved
        ));

        STORE
            .lock()
            .unwrap()
            .scenes
            .entry(1)
            .or_default()
            .insert(7, Arc::downgrade(&destination));
        STORE.lock().unwrap().capture = Some(capture(6, &destination));
        reset();
        assert!(destination.lock().unwrap().audio.is_none());
        assert!(STORE.lock().unwrap().scenes.is_empty());
        finish_frame(6, 10, &[1.0; 4], &[1.0; 4]).unwrap();
        assert!(!busy());

        STORE.lock().unwrap().capture = Some(capture(7, &destination));
        drop(destination);
        assert!(finish_frame(7, 10, &[1.0; 4], &[1.0; 4]).is_err());
        cancel(7);
        assert!(!busy());
    }
}
