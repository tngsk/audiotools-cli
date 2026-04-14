# AudioTools CLI (Rust)

AudioTools CLI は、音声ファイルの処理、分析、可視化を行うためのRust製コマンドラインユーティリティ集です。
Python版 `audiotools` プロジェクトの機能をRustで再実装・拡張し、高効率かつモジュール化された構成になっています。

## 特徴

-   **モジュラー構成**: 各機能が独立したクレート（`crates/`）として実装されていますが、共通ロジック（`audiotools-core`）を介して連携します。
-   **統合設定管理**: デフォルト設定を単一の `config.yaml` で管理可能。CLI引数によるオーバーライドもサポート。
-   **高速処理**: Rustによる実装で、大量の音声ファイル処理や信号処理を高速に実行します。

## モジュール一覧

本プロジェクトは以下のCLIツールを含みます：

### 新規実装・強化モジュール

1.  **`segment-cli`** (`crates/segment-cli`)
    -   音声分割ツール。無音区間やオンセット（音の立ち上がり）を検出してファイルを分割します。
    -   主な機能: 無音トリミング、オンセット検出分割、フェード処理。
2.  **`features-cli`** (`crates/features-cli`)
    -   特徴量抽出ツール。音声ファイルから様々な音響特徴量を計算し、JSON/CSVで出力します。
    -   主な特徴量: RMS, ZCR, Spectral Centroid/Rolloff/Flatness/Flux。
3.  **`pca-cli`** (`crates/pca-cli`)
    -   PCA（主成分分析）可視化ツール。抽出された特徴量データを次元圧縮し、散布図としてプロットします。

### 既存・リファクタリング済みモジュール

4.  **`convert-cli`** (`crates/convert-cli`)
    -   フォーマット変換（WAV出力）、ビット深度変換、チャンネル変換。※Rustネイティブ環境への移行に伴い、FLACやMP3の出力、およびサンプリングレートの変換機能はドロップされました。
5.  **`normalize-cli`** (`crates/normalize-cli`)
    -   ピークノーマライズ。指定したdBFSレベルに音量を正規化します。
6.  **`spectrum-cli`** (`crates/spectrum-cli`)
    -   スペクトログラム画像の生成。STFTパラメータや周波数範囲を詳細に設定可能。
7.  **`waveform-cli`** (`crates/waveform-cli`)
    -   波形画像の生成。RMSエンベロープ表示やアノテーション機能に対応。
8.  **`info-cli`** (`crates/info-cli`)
    -   音声ファイルのメタデータ（サンプリングレート、ビット深度、長さ等）の表示。
9.  **`loudness-cli`** (`crates/loudness-cli`)
    -   ラウドネス測定（EBU R128準拠）。

## セットアップ

### 必要要件
-   **Rust**: 最新の安定版 (`stable`)

### ビルド
プロジェクトのルートディレクトリで以下のコマンドを実行し、全ツールをビルドします。

```bash
cargo build --workspace --release
```

生成されたバイナリは `target/release/` に配置されます。

## 設定 (config.yaml)

カレントディレクトリに `config.yaml` が存在する場合、自動的に読み込まれてデフォルト値として使用されます。
CLI引数で値を指定した場合は、設定ファイルの値よりもCLI引数が優先されます。

**config.yaml の例**:

```yaml
global:
  overwrite: false
  recursive: true

segment:
  segment_len: 1.0
  top_db: 30

spectrogram:
  width: 1200
  height: 600
  fmax: 20000
  n_mels: 128
  
normalize:
  level: -1.0
```

## 使用例

### 1. 音声の分割 (segment-cli)

```bash
# default: config.yaml の設定を使用
target/release/segment-cli --input raw_audio.wav --output-dir segments/

# override: パラメータをCLIで指定
target/release/segment-cli -i raw_audio.wav -o segments/ --segment-len 0.5 --top-db 40
```

### 2. 特徴量の抽出 (features-cli)

```bash
# フォルダ内の全WAVファイルから特徴量を抽出
target/release/features-cli -i segments/ -o features.csv --format csv --recursive
```

### 3. PCA分析と可視化 (pca-cli)

```bash
# 特徴量CSVからPCAを実行し、プロット画像を生成
target/release/pca-cli -i features.csv -o pca_plot.png --components 2
```

### 4. スペクトログラム生成 (spectrum-cli)

```bash
# スペクトログラムを生成（設定はconfig.yamlまたはデフォルト）
target/release/spectrum-cli -i audio.wav -o spec.png
```

### 5. フォーマット変換 (convert-cli)

```bash
# 音声ファイルをWAV(16bit)に変換
target/release/convert-cli -i input_dir/ -O wav --bit-depth 16 --recursive
```

## 開発ルール

本プロジェクトは以下のポリシーに従って開発されています：
-   **モジュールの独立性**: 各CLIツールは独立して動作可能。
-   **設定の統一**: `audiotools-core` を通じた共通設定管理。
-   **Rust Way**: `thiserror` によるエラーハンドリング、`clap` による引数解析、`cargo` ワークスペース機能の活用。

## ライセンス

MIT License
