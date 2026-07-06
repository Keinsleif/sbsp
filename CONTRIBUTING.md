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
* frontend: sbsp_frontendパッケージに関する変更
* license: sbsp_licenseクレートに関する変更
* app: sbsp_appクレートにのみ関する変更
* remote_app: sbsp_remote_appクレートにのみ関する変更
* docs: sbsp_docsに関する変更

#### 破壊的変更について

既存の機能やファイル形式、APIの仕様に互換性が無くなる変更を含める場合は`<type>`のすぐ後ろに`!`を付与します。

記入例：`feat!: remove deprecated audio cue`

---

## 3. 開発環境のセットアップ

当プロジェクトでは、ツールチェインの管理に `mise` を使用しています。

### ステップ 1: `mise` のインストール

お使いの環境に応じて `mise` をインストールしてください。

* [mise 公式ドキュメント](https://mise.jdx.dev/getting-started.html#installing-mise-cli)

### ステップ 2: 依存パッケージのインストール

Tauri およびオーディオ制御（cpal）に必要なシステムパッケージをインストールします。

#### ① Tauri v2 の前提パッケージ

各OSに応じた必須パッケージをインストールしてください。

* 詳細は [Tauri 公式ドキュメント: 前提条件](https://v2.tauri.app/ja/start/prerequisites/) を参照。

#### ② Linux環境のみ：cpal のビルド依存関係

Linux環境で開発する場合は、ALSAやASOUND関連の開発用パッケージが必要です。

* 詳細は [RustAudio/cpal (Linux build dependencies)](https://github.com/RustAudio/cpal#linux-build-dependencies) を参照。

### ステップ 3: ツールと依存関係のセットアップ

環境が整ったら、リポジトリのルートで以下のコマンドを順番に実行します。

```bash
# 1. (任意) cargo-binstall の追加を行う場合
mise use -g cargo-binstall

# 2. miseに定義されたツールチェイン（Rust, Node.js等）のインストール
mise install

# 3. フロントエンド依存パッケージのインストール
pnpm install
```

### ステップ 4: アプリケーションのビルド・起動

開発対象のアプリケーションディレクトリに移動し、Tauriのビルドを行います。

sbsp_app の場合:

```bash
cd sbsp_app
cargo tauri build
```

sbsp_remote_app の場合:

```bash
cd sbsp_remote_app
cargo tauri build
```

---

ご不明な点がある場合や、環境構築でエラーが発生した場合は、Issueまたはメンテナーまでお気軽にご連絡ください！
