# SweepLSD

- Upstream: https://github.com/yosh-shimizu/sweeplsd
- Commit: `a7685084419e2aeb8846da4f084eb4ae8ba80ec0` (4.1.0)
- License: MIT; see [LICENSE](LICENSE).

`detectOnePass` に必要なヘッダーとソースのみを、変更せず同梱しています。
OpenCV、画像入出力、ベンチマーク用の他の検出器は使用しません。
`sweeplsd::detect(&pixels, width, height)` で8bitグレースケール画像から
`[x0, y0, x1, y1]` の配列を取得できます。C++17コンパイラーが必要です。
`build.rs` がC++のビルドと静的リンクを行い、`src/bridge.cpp` がFFIを提供します。
RGBAの合成・リサイズとrikky_moduleの互換処理は親crate側で行います。
