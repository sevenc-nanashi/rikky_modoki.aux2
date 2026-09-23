use super::{BakedAudio, Format, frame_samples};
use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::{
    f64::consts::TAU,
    sync::{Arc, LazyLock},
};

static FFT: LazyLock<[Arc<dyn Fft<f64>>; 4]> = LazyLock::new(|| {
    let mut planner = FftPlanner::new();
    std::array::from_fn(|resolution| planner.plan_fft_forward(1024 << resolution))
});

// 指定フレームの先頭から連続するPCMを読む。隣接するバッファも利用する。
pub(super) fn read_window(
    snapshots: &[Arc<BakedAudio>],
    format: Format,
    mut frame: i64,
    last_frame: i64,
    count: usize,
) -> anyhow::Result<[Vec<f64>; 2]> {
    let mut output = [vec![0.0; count], vec![0.0; count]];
    let mut offset = 0;
    while offset < count {
        if frame > last_frame {
            break;
        }
        let samples = if frame < 0 {
            None
        } else {
            frame_samples(snapshots, format, frame)?
        };
        let frame_count = match samples {
            Some(samples) => samples[0].len(),
            None => format.sample_count(frame),
        };
        let take = frame_count.min(count - offset);
        if let Some(samples) = samples {
            for channel in 0..2 {
                for i in 0..take {
                    output[channel][offset + i] = f64::from(samples[channel][i]);
                }
            }
        }
        offset += take;
        frame += 1;
    }
    Ok(output)
}

pub(super) fn fourier(
    mut samples: [Vec<f64>; 2],
    resolution: usize,
    monaural: bool,
) -> (Vec<f64>, Vec<f64>) {
    if monaural {
        let [left, right] = &mut samples;
        for (left, right) in left.iter_mut().zip(right) {
            *left = (*left + *right) / 2.0;
        }
        (amplitudes(&samples[0], resolution), Vec::new())
    } else {
        (
            amplitudes(&samples[0], resolution),
            amplitudes(&samples[1], resolution),
        )
    }
}

fn amplitudes(samples: &[f64], resolution: usize) -> Vec<f64> {
    let count = samples.len();
    // 周期Hann窓の振幅損失を補正する。DCを含み、Nyquistのビンは含めない。
    let mut buffer: Vec<_> = samples
        .iter()
        .enumerate()
        .map(|(i, sample)| {
            Complex::new(
                sample * (0.5 - 0.5 * (TAU * i as f64 / count as f64).cos()),
                0.0,
            )
        })
        .collect();
    FFT[resolution].process(&mut buffer);
    buffer[..count / 2]
        .iter()
        .enumerate()
        .map(|(i, value)| value.norm() * if i == 0 { 2.0 } else { 4.0 } / count as f64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aviutl2::common::Rational32;

    #[test]
    fn resolves_tones_and_normalizes_amplitudes_at_every_resolution() {
        for resolution in 0..=3 {
            let count = 1024 << resolution;
            let tone = |bin: usize, gain: f64| -> Vec<f64> {
                (0..count)
                    .map(|i| gain * (TAU * bin as f64 * i as f64 / count as f64).sin())
                    .collect()
            };
            let (left, right) = fourier([tone(37, 0.5), tone(91, 0.25)], resolution, false);
            assert_eq!((left.len(), right.len()), (count / 2, count / 2));
            for (values, bin, gain) in [(&left, 37, 0.5), (&right, 91, 0.25)] {
                let peak = values
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .unwrap();
                assert_eq!(peak.0, bin);
                assert!((peak.1 - gain).abs() < 1e-12);
                assert!(values[bin + 10] < 1e-12);
            }
            let (dc, _) = fourier([vec![0.25; count], vec![0.0; count]], resolution, false);
            assert!((dc[0] - 0.25).abs() < 1e-12);
            let (silence, right) = fourier([vec![0.0; count], vec![0.0; count]], resolution, false);
            assert!(silence.iter().chain(&right).all(|&value| value == 0.0));
            let (mono, right) = fourier([tone(37, 0.5), tone(37, -0.5)], resolution, true);
            assert!(mono.iter().all(|&value| value == 0.0));
            assert!(right.is_empty());
        }
        // 公開APIの不正引数はホストへのアクセス前に拒否する。
        assert!(super::super::fourier(0, 4, false).is_err());
        assert!(super::super::fourier(i64::MAX, 0, false).is_err());
    }

    #[test]
    fn reads_contiguous_audio_across_overlaps_gaps_and_scene_edges() {
        let format = Format {
            fps: Rational32::new(30, 1),
            sample_rate: 120,
        };
        let buffer = |generation, start, frames| {
            Arc::new(BakedAudio {
                scene: 1,
                revision: 0,
                generation,
                format,
                start,
                frames,
            })
        };
        let snapshots = [
            buffer(1, 1, vec![[vec![0.25; 4], vec![-0.25; 4]]; 2]),
            buffer(2, 2, vec![[vec![0.5; 3], vec![-0.5; 3]]]),
            buffer(3, 4, vec![[vec![1.0; 4], vec![-1.0; 4]]]),
        ];
        let result = read_window(&snapshots, format, 0, 4, 24).unwrap();
        let expected = [
            vec![0.0; 4],
            vec![0.25; 4],
            vec![0.5; 3],
            vec![0.0; 4],
            vec![1.0; 4],
            vec![0.0; 5],
        ]
        .concat();
        assert_eq!(result[0], expected);
        assert_eq!(
            result[1],
            expected.iter().map(|value| -value).collect::<Vec<_>>()
        );
        assert_eq!(
            read_window(&snapshots, format, -1, 4, 8).unwrap(),
            [vec![0.0; 8], vec![0.0; 8]]
        );
        assert_eq!(
            read_window(&snapshots, format, 5, 4, 8).unwrap(),
            [vec![0.0; 8], vec![0.0; 8]]
        );
        assert_eq!(
            read_window(&snapshots, format, 2, 4, 2).unwrap(),
            [vec![0.5; 2], vec![-0.5; 2]]
        );
        assert!(
            read_window(
                &snapshots,
                Format {
                    sample_rate: 240,
                    ..format
                },
                0,
                4,
                16
            )
            .is_err()
        );
    }
}
