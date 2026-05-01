# AudioTools Pipeline Architecture

## 1. 概要 (Overview)

本プロジェクトは元々、「独立したコマンドを集めたオーディオ処理コレクション」としてスタートしました。しかし、各ツールが独立しているがゆえに、データの読み込み・保存処理の重複、レイヤーの混在、および複数処理を連結する際のオーバーヘッドが生じていました。

これらの課題を解決し、モジュール式の「柔軟に処理プロセスを組み合わせられる」というメリットを最大限に引き出すため、**「アセットストア（Asset Store）とノード（Node）に基づくパイプライン・アーキテクチャ」**へとリファクタリングを行いました。

これにより、各モジュールは純粋な「処理ブロック（Node）」として機能し、オーケストレーター（`pipeline-cli`）を介して、インメモリで効率的にデータを引き渡しながら連続処理を行うことが可能になっています。将来的なGUI（ノードエディタ）との連携も視野に入れた設計です。

---

## 2. コア・コンポーネント (Core Components)

新しいアーキテクチャは `audiotools-core` クレート内の `pipeline` モジュールで定義されています。

### 2.1 Asset (データの型)
パイプライン内を流れるデータはすべて `Asset` 列挙型としてカプセル化されます。これにより、音声データ、特徴量、ファイルパスなど、異なる型のデータを単一のストアで柔軟に管理できます。

```rust
pub enum Asset {
    Audio(Vec<f32>, u32),              // 音声波形データとサンプリングレート
    AudioList(Vec<(Vec<f32>, u32)>),   // 分割された複数音声データ
    Features(HashMap<String, Vec<f32>>), // 特徴量データ
    Path(String),                      // ファイルパス（画像やCSVなど）
    String(String),                    // テキスト情報
}
```

### 2.2 AssetStore (共有コンテキスト)
各ノードの入出力を繋ぐ「データ置き場」です。実行時に一意のキー（String）と `Asset` をマッピングして保持します。ノード間を直接繋ぐのではなく、このストアを介してデータを読み書きすることで、複雑なDAG（有向非巡回グラフ）の構築やノードエディタとの連携が容易になります。

### 2.3 Node (処理ブロック)と NodeContext
すべての処理モジュールは `Node` トレイトを実装します。`process` メソッドは `NodeContext`（AssetStoreへの参照と、入出力キーのマッピング情報を持つ構造体）を受け取ります。

**Nodeの実装例（NormalizeNode）**:
```rust
use anyhow::{anyhow, Result};
use audiotools_core::pipeline::{Asset, Node, NodeContext};

pub struct NormalizeNode {
    pub level_dbfs: f32, // ノード固有の設定値
}

impl Node for NormalizeNode {
    fn name(&self) -> &str {
        "NormalizeNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        // 1. Contextから入力キー("audio")に対応するデータを取得
        let asset = context.get_input("audio")?;

        let (samples, sample_rate) = match asset {
            Asset::Audio(s, r) => (s, *r),
            _ => return Err(anyhow!("Expected Audio asset")),
        };

        // 2. 実際の処理（ゲインの計算と適用）
        let current_peak = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);
        let current_peak_dbfs = if current_peak > 0.0 { 20.0 * current_peak.log10() } else { -100.0 };
        let gain_db = self.level_dbfs - current_peak_dbfs;
        let gain_multiplier = 10.0_f32.powf(gain_db / 20.0);

        let normalized: Vec<f32> = samples.iter().map(|&s| (s * gain_multiplier).clamp(-1.0, 1.0)).collect();

        // 3. 処理結果をContextの出力キー("audio")にセット
        context.set_output("audio", Asset::Audio(normalized, sample_rate))?;
        Ok(())
    }
}
```

---

## 3. オーケストレーター (pipeline-cli) と設定ファイル

新しく追加された `pipeline-cli` は、YAML形式の設定ファイルを読み込み、定義された順序に従ってNodeを生成・実行するオーケストレーターです。

### pipeline.yaml の仕様と例

設定ファイルでは、各ノードの `id`、`type`（ノードの種類）、`config`（パラメータ）、および `inputs` と `outputs`（AssetStoreのキーマッピング）を定義します。

**例：音声を読み込み、ノーマライズし、特徴量抽出と画像出力を同時に行うパイプライン**

```yaml
nodes:
  - id: step1_load
    type: AudioInputNode
    config:
      path: "input.wav"
    outputs:
      audio: "raw_audio"  # AssetStoreに "raw_audio" として保存

  - id: step2_normalize
    type: NormalizeNode
    config:
      level: -1.0
    inputs:
      audio: "raw_audio"  # "raw_audio" を入力として受け取る
    outputs:
      audio: "norm_audio" # 結果を "norm_audio" として保存

  - id: step3_features
    type: FeaturesNode
    inputs:
      audio: "norm_audio" # 分岐1: 正規化された音声から特徴量抽出
    outputs:
      features: "features_result"

  - id: step4_spectrum
    type: SpectrumNode
    config:
      width: 1024
      height: 512
    inputs:
      audio: "norm_audio" # 分岐2: 正規化された音声からスペクトログラム生成
    outputs:
      image: "spectrum_image"
```

**実行コマンド**:
```bash
cargo run --bin pipeline-cli -- --config pipeline.yaml
```

---

## 4. 今後の方針・課題 (Next Steps)

ここまでの作業で基礎的なパイプライン基盤とCLIのNode化は完了しました。今後は以下の拡張が想定されます。

1. **GUI / ノードエディタとの連携基盤の実装**
   - 現在の `pipeline.yaml` は、ノードエディタから出力されるJSON/YAMLと1対1でマッピングできる構造になっています。次はフロントエンド（Web UI等）を構築し、視覚的にノードを繋いで実行設定ファイルを生成する仕組みの開発が目標となります。
2. **既存CLIのルーティング改善**
   - 現在、各機能のスタンドアロンCLI（`normalize-cli`、`spectrum-cli`など）は、内部で3ステップのパイプライン（Input -> Node -> Output）を構築して実行するようにリファクタリングされています。
   - より高度な引数のマッピングや、バッチ処理（複数ファイルのフォルダ一括処理）の柔軟性をNodeアーキテクチャ上でさらに高めるための設計チューニングが課題です。
3. **入出力ノードの拡充**
   - 現在のファイルI/Oは `AudioInputNode` と `AudioOutputNode` が中心ですが、CSVの入出力ノード（PCA用）や、複数音声の書き出しノード（`segment-cli` の `AudioList` 出力用）などの汎用I/Oノードを `core` に追加していく必要があります。
4. **複雑な処理の復元**
   - 今回のリファクタリングではシステムの軽量化のため、複雑なリサンプリング機能などは一部パススルー（モック）にしています。必要に応じて `rubato` 等のクレートを導入し、完全な信号処理レイヤーを復活させます。
