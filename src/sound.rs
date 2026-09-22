use std::sync::{Arc, LazyLock, Mutex};

use aviutl2::{common::Rational32, filter::FilterConfigItems};

const NAME: &str = "オブジェクトサウンド@rikky_modoki.aux2";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Target {
    scene: i32,
    layer: u32,
    start: u32,
    end: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Format {
    fps: Rational32,
    sample_rate: u32,
}

impl Format {
    fn sample_at(self, frame: u32) -> usize {
        let numerator =
            u128::from(frame) * u128::from(self.sample_rate) * *self.fps.denom() as u128;
        let denominator = *self.fps.numer() as u128;
        ((numerator + denominator / 2) / denominator) as usize
    }

    fn fps(self) -> f64 {
        f64::from(*self.fps.numer()) / f64::from(*self.fps.denom())
    }
}

pub struct Sound {
    pub file: String,
    pub frame: f64,
    pub volume: f64,
    pub speed: f64,
    pub pan: f64,
    pub reverse: bool,
}

struct BakedSound {
    revision: u64,
    format: Format,
    samples: [Vec<f32>; 2],
}

type SavedSound = Arc<Mutex<Option<Arc<BakedSound>>>>;

struct Capture {
    id: u64,
    target: Target,
    frame: u32,
    accepting: bool,
    requests: Vec<Sound>,
    sound: BakedSound,
    destination: Option<SavedSound>,
    progress: crate::progress::Progress,
}

#[derive(Default)]
struct SoundStore {
    revision: u64,
    reset_revision: u64,
    next_capture_id: u64,
    capture: Option<Capture>,
}

// ponytail: SDKに描画要求IDがないため収集は一度に1件。同フレームの別の描画とは区別できない。
static STORE: LazyLock<Mutex<SoundStore>> = LazyLock::new(Default::default);

impl SoundStore {
    fn invalidate(&mut self) {
        self.revision += 1;
        if let Some(capture) = &self.capture {
            capture.progress.state.cancel();
        }
    }

    fn receiving(&self, frame: u32) -> bool {
        self.capture.as_ref().is_some_and(|capture| {
            capture.accepting && capture.frame == frame && capture.progress.state.running()
        })
    }
}

pub fn invalidate() {
    STORE.lock().unwrap().invalidate();
}

pub fn reset() {
    let mut store = STORE.lock().unwrap();
    store.invalidate();
    store.reset_revision = store.revision;
}

pub fn shutdown() {
    let capture = STORE.lock().unwrap().capture.take();
    // UIスレッドのjoinはSTOREのロック外で行う。
    drop(capture);
}

pub fn receiving(frame: u32) -> bool {
    STORE.lock().unwrap().receiving(frame)
}

pub fn register(frame: u32, sound: Sound) -> anyhow::Result<bool> {
    anyhow::ensure!(
        [sound.frame, sound.volume, sound.speed, sound.pan]
            .iter()
            .all(|value| value.is_finite()),
        "Expected finite sound parameters"
    );
    let mut store = STORE.lock().unwrap();
    if !store.receiving(frame) || sound.volume <= 0.0 || sound.speed <= 0.0 {
        return Ok(false);
    }
    store.capture.as_mut().unwrap().requests.push(sound);
    Ok(true)
}

pub fn length(file: &str) -> anyhow::Result<Option<f64>> {
    Ok(aviutl2::cache::get_audio_file_info(file)?.map(|info| info.total_time))
}

#[aviutl2::plugin(FilterPlugin)]
pub struct ObjectSoundAuf2 {}

#[aviutl2::filter::filter_config_items]
struct ObjectSoundAuf2Config {
    #[check(name = "オブジェクトの更新後に音声を破棄", default = false)]
    discard_on_update: bool,
    #[button(name = "音声を更新")]
    update_sound: fn(),
}

fn update_sound(
    edit: &mut aviutl2::generic::EditSection,
    _object: aviutl2::generic::ObjectHandle,
    _effect_name: String,
    _effect_index: usize,
    _item: String,
) -> aviutl2::common::AnyResult<()> {
    let object = edit
        .get_focused_object()?
        .ok_or_else(|| anyhow::anyhow!("No focused object"))?;
    let range = edit.get_object_layer_frame(object)?;
    anyhow::ensure!(
        edit.get_effect_name(edit.get_first_effect(object)?)? == NAME,
        "オブジェクトサウンドを選択してください"
    );
    anyhow::ensure!(
        crate::EDIT_HANDLE.get_edit_state()? == aviutl2::generic::EditState::Edit,
        "再生・出力を停止してから音声を更新してください"
    );
    let target = Target {
        scene: edit.info.scene_id,
        layer: range.layer.try_into()?,
        start: range.start.try_into()?,
        end: range.end.try_into()?,
    };
    let format = Format {
        fps: edit.info.fps,
        sample_rate: edit.info.sample_rate.try_into()?,
    };
    let len = format.sample_at(target.end.checked_add(1).unwrap()) - format.sample_at(target.start);
    // ponytail: 更新範囲のPCMをメモリに保持。長時間の音声ではファイル/チャンク保存に変更する。
    let mut samples = [Vec::new(), Vec::new()];
    for channel in &mut samples {
        channel.try_reserve_exact(len)?;
    }
    anyhow::ensure!(
        STORE.lock().unwrap().capture.is_none(),
        "音声を更新中です（中止後は描画の完了を待ってください）"
    );
    let owner = crate::EDIT_HANDLE
        .get_host_app_window_raw()
        .ok_or_else(|| anyhow::anyhow!("AviUtl2のウィンドウを取得できません"))?;
    let progress = crate::progress::Progress::new("音声を更新".into(), 0x0078d4, owner.hwnd)?;
    let id = {
        let mut store = STORE.lock().unwrap();
        anyhow::ensure!(store.capture.is_none(), "音声を更新中です");
        let revision = store.revision;
        store.next_capture_id += 1;
        let id = store.next_capture_id;
        store.capture = Some(Capture {
            id,
            target,
            frame: target.start,
            accepting: false,
            requests: Vec::new(),
            sound: BakedSound {
                revision,
                format,
                samples,
            },
            destination: None,
            progress,
        });
        id
    };
    // ボタンは編集ロック中に呼ばれる。ここでは要求だけを行い、完了を待たない。
    // 最初に音声処理を呼び、生成結果の保存先を対象エフェクトのuserdataへ結び付ける。
    let result = crate::EDIT_HANDLE.rendering_scene_audio(target.start, move |_| {
        let result = begin_capture(id);
        if let Err(error) = result {
            cancel(id);
            tracing::error!("音声の更新に失敗しました: {error:#}");
        }
    });
    if result.is_err() {
        cancel(id);
    }
    result?;
    Ok(())
}

fn begin_capture(id: u64) -> anyhow::Result<()> {
    let frame = {
        let mut store = STORE.lock().unwrap();
        let Some(capture) = store.capture.as_mut().filter(|capture| capture.id == id) else {
            return Ok(());
        };
        if !capture.progress.state.running() {
            drop(store);
            cancel(id);
            return Ok(());
        }
        anyhow::ensure!(
            capture.destination.is_some(),
            "対象の音声が処理されませんでした。レイヤーの表示状態を確認してください"
        );
        capture.accepting = true;
        capture.frame
    };
    request_frame(id, frame)
}

fn request_frame(id: u64, frame: u32) -> anyhow::Result<()> {
    {
        let store = STORE.lock().unwrap();
        let Some(capture) = store.capture.as_ref().filter(|capture| capture.id == id) else {
            return Ok(());
        };
        if !capture.progress.state.running() {
            drop(store);
            cancel(id);
            return Ok(());
        }
    }
    let result = crate::EDIT_HANDLE.rendering_scene_video(frame, move |video| {
        let result = if video.buffer.is_empty() {
            Err(anyhow::anyhow!("映像の評価に失敗しました"))
        } else {
            finish_frame(id, frame)
        };
        if let Err(error) = result {
            cancel(id);
            tracing::error!("音声の更新に失敗しました: {error:#}");
        }
    });
    if result.is_err() {
        cancel(id);
    }
    Ok(result?)
}

fn cancel(id: u64) {
    // SDKの完了通知後、または描画要求が失敗した時だけ呼ぶ。
    // ×操作・編集時はフラグだけ変え、描画中のジョブをここまで保持する。
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
    drop(capture);
}

fn finish_frame(id: u64, frame: u32) -> anyhow::Result<()> {
    let (format, requests, progress) = {
        let mut store = STORE.lock().unwrap();
        let Some(capture) = store.capture.as_mut().filter(|capture| capture.id == id) else {
            return Ok(());
        };
        assert_eq!(capture.frame, frame);
        capture.accepting = false;
        (
            capture.sound.format,
            std::mem::take(&mut capture.requests),
            Arc::clone(&capture.progress.state),
        )
    };
    let count = format.sample_at(frame + 1) - format.sample_at(frame);
    let mut samples = [vec![0.0; count], vec![0.0; count]];
    for sound in requests {
        if !progress.running() {
            cancel(id);
            return Ok(());
        }
        mix_file(&sound, format, &mut samples)?;
    }
    let next = {
        let mut store = STORE.lock().unwrap();
        let Some(capture) = store.capture.as_mut().filter(|capture| capture.id == id) else {
            return Ok(());
        };
        if !progress.running() {
            drop(store);
            cancel(id);
            return Ok(());
        }
        for (channel, chunk) in capture.sound.samples.iter_mut().zip(samples) {
            channel.extend(chunk);
        }
        if frame == capture.target.end {
            if !progress.update(100.0) {
                drop(store);
                cancel(id);
                return Ok(());
            }
            let capture = store.capture.take().unwrap();
            *capture.destination.unwrap().lock().unwrap() = Some(Arc::new(capture.sound));
            drop(store);
            drop(capture.progress);
            None
        } else {
            progress.update(
                f64::from(frame - capture.target.start + 1)
                    / f64::from(capture.target.end - capture.target.start + 1)
                    * 100.0,
            );
            capture.frame += 1;
            capture.accepting = true;
            Some(capture.frame)
        }
    };
    if let Some(next) = next {
        request_frame(id, next)?;
    } else {
        tracing::info!("音声の更新が完了しました");
    }
    Ok(())
}

fn mix_file(sound: &Sound, format: Format, output: &mut [Vec<f32>; 2]) -> anyhow::Result<()> {
    let info = aviutl2::cache::get_audio_file_info(&sound.file)?
        .ok_or_else(|| anyhow::anyhow!("音声を開けません: {}", sound.file))?;
    let speed = sound.speed / 100.0;
    let start = (sound.frame * speed * f64::from(format.sample_rate) / format.fps() + 0.5).floor();
    let end =
        ((sound.frame + 1.0) * speed * f64::from(format.sample_rate) / format.fps() + 0.5).floor();
    let ratio = f64::from(info.rate) / f64::from(format.sample_rate);
    let start = start * ratio;
    let end = end * ratio;
    anyhow::ensure!(
        start.is_finite() && end.is_finite(),
        "Sound position overflow"
    );
    let read_start = start.floor().clamp(0.0, info.sample_num as f64) as usize;
    let read_end = (end.ceil() + 1.0).clamp(0.0, info.sample_num as f64) as usize;
    if read_end <= read_start || output[0].is_empty() {
        return Ok(());
    }
    let mut source = [Vec::new(), Vec::new()];
    for channel in &mut source {
        channel.try_reserve_exact(read_end - read_start)?;
        channel.resize(read_end - read_start, 0.0);
    }
    let [left, right] = &mut source;
    let read = aviutl2::cache::get_audio_file_data(&sound.file, 0, read_start, left, right)?;
    anyhow::ensure!(read > 0, "音声を読み込めません: {}", sound.file);
    left.truncate(read);
    right.truncate(read);
    if info.channel == 1 {
        right.copy_from_slice(left);
    }
    mix_samples(
        sound,
        &source,
        start - read_start as f64,
        end - start,
        output,
    );
    Ok(())
}

fn mix_samples(
    sound: &Sound,
    source: &[Vec<f32>; 2],
    start: f64,
    length: f64,
    output: &mut [Vec<f32>; 2],
) {
    let count = output[0].len();
    let pan = sound.pan.clamp(-100.0, 100.0) / 100.0;
    for i in 0..count {
        let position = start + length * i as f64 / count as f64;
        let left = interpolate(&source[0], position);
        let right = interpolate(&source[1], position);
        // 旧版のステレオパンは片側を絞りつつ、反対側へ半分を混ぜる。
        let (left, right) = if pan < 0.0 {
            (
                (1.0 + pan / 2.0) * left - pan / 2.0 * right,
                (1.0 + pan) * right,
            )
        } else {
            (
                (1.0 - pan) * left,
                pan / 2.0 * left + (1.0 - pan / 2.0) * right,
            )
        };
        let index = if sound.reverse { count - 1 - i } else { i };
        output[0][index] += (left * sound.volume / 100.0) as f32;
        output[1][index] += (right * sound.volume / 100.0) as f32;
    }
}

fn interpolate(samples: &[f32], position: f64) -> f64 {
    // ファイル範囲外は無音として補間する。
    let sample = |index: f64| {
        if index < 0.0 || index >= samples.len() as f64 {
            0.0
        } else {
            f64::from(samples[index as usize])
        }
    };
    let index = position.floor();
    let left = sample(index);
    left + (sample(index + 1.0) - left) * (position - index)
}

#[derive(Default)]
pub struct ObjectSoundData {
    sound: SavedSound,
}

impl ObjectSoundData {
    fn snapshot(&self, store: &SoundStore, discard_on_update: bool) -> Option<Arc<BakedSound>> {
        let mut saved = self.sound.lock().unwrap();
        if saved.as_ref().is_some_and(|sound| {
            sound.revision < store.reset_revision
                || (discard_on_update && sound.revision != store.revision)
        }) {
            *saved = None;
        }
        saved.clone()
    }
}

impl aviutl2::filter::FilterUserdata for ObjectSoundData {
    fn new(_effect_id: i64) -> Self {
        Self::default()
    }
}

impl aviutl2::filter::FilterPlugin for ObjectSoundAuf2 {
    type Userdata = ObjectSoundData;

    fn new(_info: aviutl2::common::AviUtl2Info) -> aviutl2::common::AnyResult<Self> {
        Ok(Self {})
    }

    fn plugin_info(&self) -> aviutl2::filter::FilterPluginTable {
        aviutl2::filter::FilterPluginTable {
            name: NAME.to_string(),
            information: "object_sound.auf2 @ rikky_modoki.aux2".to_string(),
            label: Some("@rikky_modoki.aux2".to_string()),
            flags: aviutl2::bitflag!(aviutl2::filter::FilterPluginFlags {
                audio: true,
                input: true,
            }),
            config_items: ObjectSoundAuf2Config::to_config_items(),
        }
    }

    fn proc_audio(
        &self,
        configs: &[aviutl2::filter::FilterConfigItem],
        audio: &mut aviutl2::filter::FilterProcAudio<Self::Userdata>,
    ) -> aviutl2::common::AnyResult<()> {
        let config = ObjectSoundAuf2Config::from_config_items(configs);
        let target = Target {
            scene: crate::EDIT_HANDLE.get_edit_info().scene_id,
            layer: audio.object.layer,
            start: audio.object.frame_s,
            end: audio.object.frame_e,
        };
        let data = audio.userdata.read();
        let mut store = STORE.lock().unwrap();
        if let Some(capture) = store.capture.as_mut()
            && capture.target == target
            && capture.destination.is_none()
            && capture.progress.state.running()
        {
            capture.destination = Some(Arc::clone(&data.sound));
        }
        let saved = data.snapshot(&store, config.discard_on_update);
        // 収集中は自身の音を返さない。obj.getaudio("audiobuffer") の循環を防ぐ。
        let capturing = store
            .capture
            .as_ref()
            .is_some_and(|capture| capture.progress.state.running());
        drop(store);
        drop(data);
        let mut output = [
            vec![0.0; audio.audio_object.sample_num as usize],
            vec![0.0; audio.audio_object.sample_num as usize],
        ];
        // 未生成・破棄済みのオブジェクトは無音。
        if !capturing && let Some(sound) = saved {
            anyhow::ensure!(
                sound.format
                    == Format {
                        fps: audio.scene.frame_rate,
                        sample_rate: audio.scene.sample_rate
                    },
                "シーンの音声設定が変更されています。「音声を更新」を押してください"
            );
            let start = usize::try_from(audio.audio_object.sample_index)?;
            for (destination, source) in output.iter_mut().zip(&sound.samples) {
                if start < source.len() {
                    let count = destination.len().min(source.len() - start);
                    destination[..count].copy_from_slice(&source[start..start + count]);
                }
            }
        }
        audio.set_sample_data(aviutl2::filter::AudioChannel::Left, &output[0]);
        audio.set_sample_data(aviutl2::filter::AudioChannel::Right, &output[1]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixes_frames_and_discards_only_when_requested() {
        let format = Format {
            fps: Rational32::new(30_000, 1_001),
            sample_rate: 48_000,
        };
        assert_eq!(
            (
                format.sample_at(1),
                format.sample_at(2),
                format.sample_at(5)
            ),
            (1602, 3203, 8008)
        );
        let frames: usize = (101..201)
            .map(|frame| format.sample_at(frame + 1) - format.sample_at(frame))
            .sum();
        assert_eq!(frames, format.sample_at(201) - format.sample_at(101));

        let mut sound = Sound {
            file: String::new(),
            frame: 0.0,
            volume: 50.0,
            speed: 100.0,
            pan: 0.0,
            reverse: false,
        };
        let source = [vec![0.0, 0.25, 0.5, 0.75], vec![1.0, 0.75, 0.5, 0.25]];
        let mut output = [vec![0.0; 4], vec![0.0; 4]];
        mix_samples(&sound, &source, 0.0, 2.0, &mut output);
        assert_eq!(
            output,
            [
                vec![0.0, 0.0625, 0.125, 0.1875],
                vec![0.5, 0.4375, 0.375, 0.3125]
            ]
        );
        sound.pan = 100.0;
        sound.reverse = true;
        mix_samples(&sound, &source, 0.0, 4.0, &mut output);
        assert_eq!(
            output,
            [
                vec![0.0, 0.0625, 0.125, 0.1875],
                vec![0.75, 0.6875, 0.625, 0.5625]
            ]
        );
        sound.pan = -100.0;
        output = [vec![0.0; 4], vec![0.0; 4]];
        mix_samples(&sound, &source, -2.0, 8.0, &mut output);
        assert_eq!(output, [vec![0.0, 0.25, 0.25, 0.0], vec![0.0; 4]]);
        assert_eq!(interpolate(&[1.0], -0.5), 0.5);
        assert_eq!(interpolate(&[1.0], 0.5), 0.5);

        let mut store = SoundStore::default();
        let baked = Arc::new(BakedSound {
            revision: 0,
            format,
            samples: output,
        });
        let data = ObjectSoundData {
            sound: Arc::new(Mutex::new(Some(Arc::clone(&baked)))),
        };
        assert!(data.snapshot(&store, true).is_some());
        store.capture = Some(Capture {
            id: 1,
            target: Target {
                scene: 0,
                layer: 0,
                start: 10,
                end: 11,
            },
            frame: 10,
            accepting: true,
            requests: Vec::new(),
            sound: BakedSound {
                revision: 0,
                format,
                samples: [Vec::new(), Vec::new()],
            },
            destination: Some(Arc::clone(&data.sound)),
            progress: crate::progress::Progress::headless(),
        });
        assert!(store.receiving(10));
        assert!(!store.receiving(11));
        store.invalidate(); // 登録元オブジェクトの削除もこの経路を通る。
        assert!(!store.receiving(10));
        assert!(data.snapshot(&store, false).is_some());
        assert!(data.snapshot(&store, true).is_none());
        assert!(ObjectSoundData::default().snapshot(&store, false).is_none());
        *data.sound.lock().unwrap() = Some(baked);
        store.reset_revision = store.revision;
        assert!(data.snapshot(&store, false).is_none()); // プロジェクトを跨いで持ち越さない。

        // 取り消したジョブの遅延コールバックで、新しい収集を確定・取り消ししない。
        STORE.lock().unwrap().capture = Some(Capture {
            id: 2,
            target: Target {
                scene: 0,
                layer: 0,
                start: 10,
                end: 10,
            },
            frame: 10,
            accepting: true,
            requests: Vec::new(),
            sound: BakedSound {
                revision: 0,
                format,
                samples: [Vec::new(), Vec::new()],
            },
            destination: Some(Arc::clone(&data.sound)),
            progress: crate::progress::Progress::headless(),
        });
        finish_frame(1, 10).unwrap();
        cancel(1);
        assert!(receiving(10));
        finish_frame(2, 10).unwrap();
        assert!(!receiving(10));
        let saved = data.sound.lock().unwrap();
        assert_eq!(
            saved.as_ref().unwrap().samples[0],
            vec![0.0; format.sample_at(11) - format.sample_at(10)]
        );
        let previous = Arc::clone(saved.as_ref().unwrap());
        drop(saved);

        let cancelled = crate::progress::Progress::headless();
        let state = Arc::clone(&cancelled.state);
        STORE.lock().unwrap().capture = Some(Capture {
            id: 3,
            target: Target {
                scene: 0,
                layer: 0,
                start: 10,
                end: 11,
            },
            frame: 11,
            accepting: true,
            requests: vec![Sound {
                file: "読み込んではいけない.wav".into(),
                frame: 1.0,
                volume: 100.0,
                speed: 100.0,
                pan: 0.0,
                reverse: false,
            }],
            sound: BakedSound {
                revision: 0,
                format,
                samples: [vec![0.5; 100], vec![0.5; 100]],
            },
            destination: Some(Arc::clone(&data.sound)),
            progress: cancelled,
        });
        state.cancel(); // ×を押した後も描画の完了通知まではジョブを保持する。
        assert!(!receiving(11));
        assert!(STORE.lock().unwrap().capture.is_some());
        finish_frame(2, 11).unwrap(); // 古い通知では新しいジョブを解放しない。
        assert!(STORE.lock().unwrap().capture.is_some());
        finish_frame(3, 11).unwrap(); // 最終フレームでも確定せず、途中結果を破棄する。
        assert!(STORE.lock().unwrap().capture.is_none());
        assert!(Arc::ptr_eq(
            data.sound.lock().unwrap().as_ref().unwrap(),
            &previous
        ));
    }
}
