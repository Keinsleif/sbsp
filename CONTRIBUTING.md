# CONTRIBUTING.md (コントリビューションガイド)

## 1. ワークフローとブランチ戦略

当プロジェクトでは、GitHubを利用したPR（Pull Request）ベースの開発を行っています。

### 各ブランチの役割と保護規則

| ブランチ名 | 役割 | 保護ルール | マージ方法 |
| --- | --- | --- | --- |
| `master` | リリース済みのソースコードを管理。 | **直接のPush禁止** | `dev` ブランチからのみ（Mergeコミット） |
| `dev` | 開発のベースとなるメインブランチ。 | **直接のPush禁止** | 派生ブランチからのみ（Squash & Merge） |
| `feat/*`, `fix/*`, `ci/*` | 機能追加やバグ修正などの作業用ブランチ。 | 特になし | `dev` ブランチへPRを作成 |

### ブランチの命名規則

作業ブランチを作成する際は、以下のプレフィックスを使用してください。

* 機能追加: `feat/具体的な機能名`
* バグ修正: `fix/問題の概要`
* CI/CD関連: `ci/変更内容`
* その他: `docs/` (ドキュメント), `refactor/` (リファクタリング) など

---

## 2. プルリクエスト（PR）のルール

### `feat/*` などの作業ブランチ ➔ `dev` へのPR

1. PRタイトル（必須）:
    * **英語**、かつ [**Conventional Commits 形式**](#conventional-commits-形式について)で記述してください。
    * 例: `feat: add audio playback support`, `fix: resolve crash on startup`
    * *※ CIによる自動チェックが実行され、形式に沿っていない場合はマージできません。*

2. THIRD_PARTY_NOTICESのチェック:
    * パッケージやライブラリの依存関係（`Cargo.toml` や `package.json`）に変更がある場合、`THIRD_PARTY_NOTICES` の更新チェックが自動で行われます。必要に応じて`mise generate_notices`で更新してください。

### `dev` ➔ `master` へのPR（リリース時）

* バージョン更新時に作成されます。
* マージ後、GitHub Actionsにより [cocogitto](https://docs.cocogitto.io/) が自動で呼び出され、以下の処理が実行されます。
* バージョンの自動バンプ（繰り上げ）
* 変更履歴（`CHANGELOG.md`）の自動生成

### Conventional Commits 形式について

当プロジェクトでは、自動でのバージョンバンプ（cocogitto による管理）と変更履歴（CHANGELOG.md）の生成を行うため、PRタイトルに [Conventional Commits 1.0.0 形式](https://www.conventionalcommits.org/ja/v1.0.0/)を採用しています。

#### 基本フォーマット

```text
<type>(<scope>): <subject>
```

`(<scope>)`は任意です。

コミットが以前のコミットを元に戻す場合は、`revert:` で始まり、その後に元に戻すコミットのヘッダーが続きます。

#### Type

`<type>`は以下のうちの一つとなります。

* feat: 新しい機能
* fix: バグ修正
* build: ビルドシステムまたは外部依存関係に影響を与える変更
* ci: CI設定ファイルとスクリプトの変更
* docs: ドキュメントの変更のみ
* perf: パフォーマンス改善に関連する変更
* refactor: バグ修正も機能追加も行わないコード変更
* style: コードの意味に影響を与えない変更（空白、書式設定、セミコロンの欠落など）
* test: 不足しているテストを追加したり、既存のテストを修正したりする

#### Scope

`<scope>`は指定されないか、以下のうちの一つから選択されます。

* backend: sbsp_backendクレートに関する変更
* ui: sbsp_uiクレート（ホスト・リモート・Web）に関する変更
* docs: sbsp_docsに関する変更
* deps: 依存関係の更新

##### backend などの変更と ui の変更

`sbsp_backend` の変更は `sbsp_ui` のバージョンにも反映されます（backend の `feat` は ui の minor 更新になります）。変更履歴では、これらの scope のコミットは「Core changes」節に分けて表示されます。

* 変更は crate ごとにコミットを分け、それぞれの scope（`backend` と `ui` など）を付けてください。
* 分けられない場合は、利用者から見える側（通常は `ui`、または scope なし）を付け、backend 側の内容は本文に記述してください。
* 破壊的変更は、どの scope でも `!` を付けてください。ui の major 更新になります。

#### 破壊的変更について

既存の機能やファイル形式、APIの仕様に互換性が無くなる変更を含める場合は`<type>`のすぐ後ろに`!`を付与します。

記入例：`feat!: remove deprecated audio cue`

---

## 3. 開発環境のセットアップ

当プロジェクトでは、ツールチェインの管理に `mise` を使用しています。

### ステップ 1: `mise` のインストール

お使いの環境に応じて `mise` をインストールしてください。

* [mise 公式ドキュメント](https://mise.jdx.dev/getting-started.html#installing-mise-cli)

### ステップ 2: ツールと依存パッケージのセットアップ

リポジトリのルートで以下を実行します。

```bash
# 1. (任意) cargo-binstall の追加を行う場合
mise use -g cargo-binstall

# 2. miseに定義されたツールチェイン（Rust, trunk, Python, .NET等）のインストール
mise install

# 3. Linux のみ：オーディオ（cpal）のビルドに必要なシステムパッケージのインストール
mise bootstrap packages apply
```

オーディオ制御（cpal）のビルド依存関係の詳細は [RustAudio/cpal (Linux build dependencies)](https://github.com/RustAudio/cpal#linux-build-dependencies) を参照してください。

### ステップ 3: アプリケーションのビルド・起動

```bash
# SBS Player（Web 版を含む）
mise run build_host

# SBS Player Remote
mise run build_remote
```

開発中は、通常の `cargo run` で起動できます。

```bash
cargo run -p sbsp_ui --no-default-features --features host --bin sbsp_host
cargo run -p sbsp_ui --no-default-features --features remote --bin sbsp_remote
```

ドキュメント（`sbsp_docs`）の開発には Node.js と pnpm を使用します（`mise install` で入ります）。

---

ご不明な点がある場合や、環境構築でエラーが発生した場合は、Issueまたはメンテナーまでお気軽にご連絡ください！
