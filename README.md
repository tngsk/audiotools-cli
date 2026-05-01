# AudioTools Pipeline (Rust)

AudioTools は、音声ファイルの処理、分析、可視化を行うためのRust製ユーティリティ集です。
現在はノードベースのパイプラインアーキテクチャを採用しており、`pipeline.yaml`の設定を通じて複雑な音声処理ワークフローを構築可能です。

## 主な特徴

- **パイプライン・オーケストレーション**: `pipeline-cli`を使用し、複数の処理ノードを連結・実行。
- **柔軟なデータフロー**: `AssetStore` と `Node` トレイトにより、ノード間でシームレスにデータを引き継ぎます。
- **高速＆ネイティブ実装**: 外部依存を極力減らしたRustネイティブ環境（WAVなどの基本的なフォーマットに特化）。

## 組み込みノード

パイプラインで利用可能な主なノード機能は以下の通りです。

- **入出力**: `AudioInputNode`, `AudioOutputNode`
- **分析・分割**: `SegmentNode` (分割), `FeaturesNode` (特徴量抽出), `PcaNode` (主成分分析)
- **可視化**: `SpectrumNode` (スペクトログラム), `WaveformNode` (波形画像)
- **処理**: `ConvertNode` (フォーマット・チャンネル変換), `NormalizeNode` (ノーマライズ)
- **情報取得**: `InfoNode` (メタデータ), `LoudnessNode` (ラウドネス測定)

## セットアップとビルド

### 必要要件 (Debian/Ubuntu)
ビルドにはALSA開発ヘッダーが必要です。
```bash
sudo apt-get update && sudo apt-get install -y libasound2-dev
```

### ビルド
```bash
cargo build --workspace --release
```

## 実行方法

パイプライン定義ファイル（例: `pipeline.yaml`）を作成し、オーケストレーターを実行します。

```bash
target/release/pipeline-cli --config pipeline.yaml
```

## ライセンス

MIT License
