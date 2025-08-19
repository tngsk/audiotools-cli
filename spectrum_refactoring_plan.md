# Spectrum Command Refactoring Plan

## 🔍 現在の問題分析

### 1. **複雑な依存関係と高い結合度**
- `spectrum/mod.rs`が650行超の巨大ファイル
- 設定、FFT処理、レンダリング、ファイルI/Oが一つのモジュールに混在
- 外部ライブラリ（plotters、hound、rustfft等）への直接依存
- `main.rs`で大量のパラメータ処理とコマンドライン引数の解析

### 2. **単一責任原則の違反**
- `create_spectrogram`関数が200行近くで複数責任を持つ：
  - オーディオファイルの読み込み
  - 時間範囲の処理
  - FFT設定の決定
  - スペクトログラムの生成
  - レンダリング処理
- `spectrum/mod.rs`内に描画ロジック、FFT処理、設定管理が混在

### 3. **複雑な設定システム**
- `SpectrogramConfig`が過度に複雑（400行超）
- レガシーパラメータとの互換性維持で複雑化
- 多数の`calculate_*`メソッドで設定ロジックが散らばっている
- 設定の検証ロジックが分散

### 4. **エラーハンドリングの不統一**
- 複数のエラータイプが混在（Config、FFT、IO、Audio）
- エラー変換が複数箇所に散らばっている
- 一貫性のないエラーメッセージ

### 5. **テスト性の問題**
- 統合テストが不足
- モジュール間の境界でのテストが困難
- 外部依存のモックが困難

## 🎯 リファクタリング計画

### フェーズ1: アーキテクチャの分離

#### 1.1 コア層の抽出
```
audiotools/src/command/spectrum/core/
├── audio/          # オーディオ処理層
│   ├── loader.rs   # ファイル読み込み
│   ├── processor.rs # サンプル処理・時間範囲
│   └── mod.rs
├── analysis/       # スペクトラム解析層
│   ├── fft.rs      # FFT処理（既存から移動・簡素化）
│   ├── windowing.rs # ウィンドウ関数
│   └── mod.rs
├── config/         # 設定管理層
│   ├── builder.rs  # 設定ビルダーパターン
│   ├── presets.rs  # プリセット管理
│   ├── validator.rs # 設定検証
│   └── mod.rs
└── mod.rs
```

#### 1.2 表現層の分離
```
audiotools/src/command/spectrum/render/
├── canvas.rs       # 描画キャンバス抽象化
├── colormap.rs     # カラーマッピング
├── annotations.rs  # アノテーション処理
├── layout.rs       # レイアウト計算
└── mod.rs
```

#### 1.3 ドメイン層の作成
```
audiotools/src/command/spectrum/domain/
├── spectrogram.rs  # スペクトログラムエンティティ
├── frequency.rs    # 周波数ドメインロジック
├── time_range.rs   # 時間範囲処理
├── audio_data.rs   # オーディオデータ構造
└── mod.rs
```

### フェーズ2: 依存性注入とインターフェース

#### 2.1 トレイトベースの設計

```rust
// core/audio/mod.rs
pub trait AudioLoader {
    type Error;
    fn load(&self, path: &Path) -> Result<AudioData, Self::Error>;
}

// core/analysis/mod.rs  
pub trait SpectralAnalyzer {
    type Error;
    fn analyze(&self, samples: &[f32]) -> Result<Spectrogram, Self::Error>;
}

// render/mod.rs
pub trait SpectrogramRenderer {
    type Error;
    fn render(&self, spectrogram: &Spectrogram, output: &Path) -> Result<(), Self::Error>;
}
```

#### 2.2 設定システムの簡素化

```rust
// core/config/builder.rs
pub struct ConfigBuilder {
    sample_rate: Option<f32>,
    frequency_range: Option<(f32, f32)>,
    window_config: Option<WindowConfig>,
    quality_level: QualityLevel,
}

impl ConfigBuilder {
    pub fn new() -> Self;
    pub fn auto_configure(duration_ms: f32) -> Self;
    pub fn with_frequency_range(mut self, min: f32, max: f32) -> Self;
    pub fn with_window_size(mut self, size: usize) -> Self;
    pub fn build(self) -> Result<SpectrumConfig, ConfigError>;
}
```

#### 2.3 ドメインエンティティの定義

```rust
// domain/spectrogram.rs
#[derive(Debug, Clone)]
pub struct Spectrogram {
    pub data: Vec<Vec<f32>>,
    pub time_axis: Vec<f32>,
    pub freq_axis: Vec<f32>,
    pub metadata: SpectrogramMetadata,
}

// domain/audio_data.rs
#[derive(Debug, Clone)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: f32,
    pub duration: f32,
    pub channels: u32,
}
```

### フェーズ3: コマンドパターンの導入

#### 3.1 SpectrumCommand構造体

```rust
// command.rs
pub struct SpectrumCommand {
    audio_loader: Box<dyn AudioLoader>,
    analyzer: Box<dyn SpectralAnalyzer>,
    renderer: Box<dyn SpectrogramRenderer>,
}

impl SpectrumCommand {
    pub fn new(
        loader: Box<dyn AudioLoader>,
        analyzer: Box<dyn SpectralAnalyzer>, 
        renderer: Box<dyn SpectrogramRenderer>,
    ) -> Self;
    
    pub async fn execute(&self, request: SpectrumRequest) -> Result<SpectrumResponse>;
    
    // バッチ処理用
    pub async fn execute_batch(&self, requests: Vec<SpectrumRequest>) -> Vec<Result<SpectrumResponse>>;
}
```

#### 3.2 リクエスト/レスポンス構造

```rust
// domain/request.rs
#[derive(Debug, Clone)]
pub struct SpectrumRequest {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub config: SpectrumConfig,
    pub time_range: Option<TimeRange>,
    pub annotations: Vec<FrequencyAnnotation>,
    pub options: SpectrumOptions,
}

#[derive(Debug)]
pub struct SpectrumResponse {
    pub output_path: PathBuf,
    pub metadata: SpectrumMetadata,
    pub processing_time: Duration,
    pub config_used: SpectrumConfig,
}
```

### フェーズ4: エラーハンドリングの統一

#### 4.1 統一エラー型

```rust
// error.rs
#[derive(Debug, thiserror::Error)]
pub enum SpectrumError {
    #[error("Audio loading failed: {0}")]
    AudioLoad(#[from] AudioLoadError),
    
    #[error("Analysis failed: {0}")]
    Analysis(#[from] AnalysisError),
    
    #[error("Rendering failed: {0}")]
    Render(#[from] RenderError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("Time range processing failed: {0}")]
    TimeRange(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

#### 4.2 エラーコンテキスト

```rust
// error.rs
pub trait ErrorContext<T> {
    fn with_context<F>(self, f: F) -> Result<T, SpectrumError>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<SpectrumError>,
{
    fn with_context<F>(self, f: F) -> Result<T, SpectrumError>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let mut error = e.into();
            // エラーコンテキストを追加
            error
        })
    }
}
```

### フェーズ5: テスト改善

#### 5.1 モックとテストユーティリティ

```
audiotools/src/command/spectrum/testing/
├── mocks/
│   ├── audio_loader.rs     # AudioLoaderのモック
│   ├── analyzer.rs         # SpectralAnalyzerのモック
│   └── renderer.rs         # Rendererのモック
├── fixtures/
│   ├── test_audio.rs       # テスト用オーディオデータ
│   └── sample_configs.rs   # テスト用設定
├── integration/
│   ├── end_to_end.rs      # エンドツーエンドテスト
│   └── performance.rs      # パフォーマンステスト
└── mod.rs
```

#### 5.2 プロパティベーステスト

```rust
// testing/property_tests.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn config_builder_produces_valid_config(
        sample_rate in 8000.0f32..192000.0,
        min_freq in 20.0f32..1000.0,
        max_freq in 1000.0f32..20000.0,
    ) {
        let config = ConfigBuilder::new()
            .with_frequency_range(min_freq, max_freq)
            .build();
        
        prop_assert!(config.is_ok());
        let config = config.unwrap();
        prop_assert!(config.min_freq >= 20.0);
        prop_assert!(config.max_freq <= sample_rate / 2.0);
    }
}
```

## 📋 実装ロードマップ

### Week 1-2: 基盤構築
- [x] 現在のコード分析完了
- [ ] 新しいディレクトリ構造作成
- [ ] コアトレイト定義
- [ ] 基本的なエラー型統一
- [ ] ドメインエンティティ定義

**成果物:**
- 新しいモジュール構造
- トレイト定義
- 基本的なエラー型
- ドメインモデル

### Week 3-4: コア機能移行
- [ ] AudioLoader実装
- [ ] SpectralAnalyzer実装  
- [ ] ConfigBuilder実装
- [ ] 基本的なテスト作成
- [ ] 既存FFTロジックの移行

**成果物:**
- コア機能の実装
- 単体テストスイート
- 設定ビルダーパターン

### Week 5-6: レンダリング層
- [ ] SpectrogramRenderer実装
- [ ] 描画ロジックの分離
- [ ] アノテーション機能の改善
- [ ] カラーマッピングの抽象化

**成果物:**
- レンダリング層の完全分離
- 拡張可能な描画システム
- 改善されたアノテーション

### Week 7-8: 統合とテスト
- [ ] SpectrumCommand統合
- [ ] 既存APIとの互換性維持
- [ ] 包括的テストスイート
- [ ] パフォーマンステスト
- [ ] ドキュメント更新

**成果物:**
- 完全に統合されたシステム
- 後方互換性の保証
- 包括的テストカバレッジ
- 詳細なドキュメント

## 🎁 期待される改善効果

### 1. **メンテナンス性の向上**
- 各コンポーネントの責任が明確
- 依存関係の分離により変更の影響範囲を限定
- コードの可読性向上

### 2. **拡張性の向上**
- 新しいレンダラーや解析手法を容易に追加
- プラグインアーキテクチャの基盤
- 設定システムの柔軟性向上

### 3. **テスタビリティの向上**
- モックによる単体テスト可能
- 統合テストの自動化
- プロパティベーステストの導入

### 4. **再利用性の向上**
- 他のコマンドでもコア機能を活用
- ライブラリとしての利用可能性
- コンポーネントの独立性

### 5. **パフォーマンスの向上**
- 非同期処理の導入
- ストリーミング処理対応
- メモリ使用量の最適化

## 📊 成功指標

### コード品質指標
- [ ] 平均関数長: < 20行
- [ ] 最大ファイルサイズ: < 300行
- [ ] 循環的複雑度: < 10
- [ ] テストカバレッジ: > 90%

### パフォーマンス指標
- [ ] メモリ使用量: 20%削減
- [ ] 処理時間: 現状と同等以上
- [ ] 起動時間: 30%短縮

### 開発効率指標
- [ ] 新機能追加時間: 50%短縮
- [ ] バグ修正時間: 60%短縮
- [ ] テスト実行時間: < 5秒

## 🚀 移行戦略

### 段階的移行アプローチ
1. **新システムの並行開発**: 既存コードを変更せずに新システムを構築
2. **機能フラグによる切り替え**: 段階的に新システムに移行
3. **後方互換性の維持**: 既存のAPIを維持しながら内部実装を置換
4. **段階的廃止**: 旧システムの段階的な削除

### リスク軽減策
- 包括的なテストスイートによる回帰防止
- A/Bテストによる品質確認
- ロールバック計画の準備
- ドキュメント更新の並行実施

このリファクタリングにより、Spectrumコマンドの品質と開発効率が大幅に向上し、将来の機能拡張が容易になることが期待されます。