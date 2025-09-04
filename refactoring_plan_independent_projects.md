# リファクタリング計画: 各コマンドの完全独立プロジェクト化 (共通モジュールなし)

## 目的
`audiotools2` プロジェクト内の各コマンド (`convert`, `info`, `loudness`, `normalize`, `spectrum`, `waveform`) を、それぞれ完全に独立したRustプロジェクト（独自の`Cargo.toml`とソースコードを持つ）として切り出す。共通モジュールは一切持たず、各コマンドの機能に特化させる。

## 前提
*   現在の `audiotools2` リポジトリをモノレポとして再構築し、各コマンドが独自の `cargo` パッケージとなる構造を採用する。
*   コードの重複が大幅に発生し、将来的なメンテナンスコストが増大する可能性があることを理解している。

## 計画詳細

### フェーズ1: プロジェクト構造の準備

1.  **ルート `Cargo.toml` の修正:**
    *   現在の `audiotools` クレートの `[[bin]]` セクションを削除する。
    *   `[workspace]` セクションを追加し、各コマンドの新しいパッケージディレクトリを `members` として指定する。

2.  **各コマンド用の新しい `cargo` パッケージの作成:**
    *   リポジトリのルートに `crates/` ディレクトリを作成する。
    *   各コマンドについて、`crates/<command_name>-cli/` ディレクトリを作成し、その中で `cargo new --bin <command_name>-cli` を実行する。
        *   例: `crates/spectrum-cli`, `crates/convert-cli`, `crates/info-cli`, `crates/loudness-cli`, `crates/normalize-cli`, `crates/waveform-cli`

### フェーズ2: コードの複製と調整

各コマンド (`spectrum`, `convert`, `info`, `loudness`, `normalize`, `waveform`) ごとに以下の手順を実行する。

1.  **コアロジックの複製:**
    *   `src/command/<command_name>/` ディレクトリの内容全体を `crates/<command_name>-cli/src/` にコピーする。
    *   例: `src/command/spectrum/*` を `crates/spectrum-cli/src/` にコピー。

2.  **共通ユーティリティの複製:**
    *   各コマンドが必要とする `src/utils/` 内のモジュール（`detection.rs`, `time.rs`, `ffprobe.rs`, `wave_header.rs` など）を、それぞれの `crates/<command_name>-cli/src/utils/` にコピーする。
    *   例: `src/utils/detection.rs`, `src/utils/time.rs` を `crates/spectrum-cli/src/utils/` にコピー。

3.  **`src/main.rs` からのロジックの抽出と複製:**
    *   現在の `src/main.rs` にある、該当コマンドのCLI引数定義 (`clap` の `#[derive(Subcommand)]` 内の定義)、`FrequencyPresetArg` とその `From` 実装、`SpectrogramConfig` の構築ロジック、`SpectrumRequest` の `annotations` 変換ロジックなど、コマンド固有の初期設定と実行ロジックを、`crates/<command_name>-cli/src/main.rs` にコピーする。
    *   各 `main.rs` は、自身のCLI引数を解析し、複製されたコマンドロジックをインスタンス化して実行する。

4.  **`Cargo.toml` の依存関係の複製:**
    *   現在のルート `Cargo.toml` から、該当コマンドが必要とするすべての依存関係（`clap`, `tokio`, `hound`, `rustfft`, `plotters`, `image`, `thiserror`, `rand`, `tokio-test`, `proptest`, `criterion` など）を、それぞれの `crates/<command_name>-cli/Cargo.toml` にコピーする。
    *   `[dev-dependencies]` や `[[bench]]` セクションも、必要に応じて複製する。

5.  **パスの調整:**
    *   複製された各プロジェクト内で、すべての `use crate::...` パスを、新しいプロジェクト構造を反映するように調整する。
        *   例: `use crate::command::spectrum::core::audio::processor::process_time_range;` は `use <command_name>-cli::command::spectrum::core::audio::processor::process_time_range;` のようになるか、あるいは `use crate::core::audio::processor::process_time_range;` のように、`crates/<command_name>-cli/src/` をルートとするパスに調整する。
        *   `utils` モジュールへのパスも同様に調整する。

### フェーズ3: クリーンアップ

1.  **元の `audiotools2` プロジェクトのクリーンアップ:**
    *   `src/main.rs` を削除する。
    *   `src/command/` ディレクトリの内容を削除する。
    *   `src/utils/` ディレクトリの内容を削除する。
    *   `src/lib.rs` は、もし汎用的な（どのコマンドにも属さない）共有ライブラリコードが残っている場合にのみ保持するが、今回の計画では「共通モジュールなし」のため、実質的に空になるか削除される。

## 完了基準
*   各コマンドが `crates/<command_name>-cli/` ディレクトリ内に完全に独立したRustプロジェクトとして存在し、それぞれが `cargo run --bin <command_name>-cli` で実行可能であること。
*   各プロジェクトが独自の `Cargo.toml` を持ち、必要な依存関係をすべて含んでいること。
*   各プロジェクトのソースコード内で、`use` パスが正しく解決されていること。
*   元の `audiotools2` プロジェクトの `src/main.rs`, `src/command/`, `src/utils/` が削除されていること。
*   `cargo check` がワークスペース全体で成功すること。

## 懸念事項 (再確認)
*   **コードの重複**: 大量のコードが複製されるため、バグ修正や機能追加の際に複数の場所を更新する必要がある。
*   **メンテナンスコスト**: 長期的には、このアプローチはメンテナンスコストを大幅に増加させる。
*   **依存関係の不整合**: 各プロジェクトが独自の依存関係を持つため、異なるバージョンが使用される可能性があり、予期せぬ問題を引き起こす可能性がある。

---
