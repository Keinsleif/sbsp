# SBS Player

創価大学放送部（SBS）での使用を想定した音源再生ソフトです。
Mac用の舞台オペレーティングソフトウェアである [QLab](https://qlab.app/) にインスパイアされています。

## 機能

### Backend (sbsp_backend)

- 基本
  - [x] apiserverの起動
  - [x] キュー、キューリストの一括管理
  - [x] キュー、キューリストの編集
  - [x] キューの実行
  - [x] preWaitの実装
  - [ ] postWait、sequence(auto-follow)の実装
  - [ ] auto-followターゲット指定
  - [ ] キュー名のデフォルト機能（内容から自動生成）
  - [x] preWait中のロード
  - [ ] cursor設定後のロード
  - [ ] cursor解除後のアンロード
- 音声
  - [x] 再生、一時停止、再開、停止
  - [x] ロード
  - [ ] アンロード
  - [x] 開始位置、終了位置
  - [x] フェードイン、フェードアウト
  - [ ] 長さ解析
  - [ ] 波形解析
  - [ ] 回数指定ループ
- その他
  - [x] 待機キュー
  - [ ] 再生、一時停止、停止キュー
  - [ ] MIDIキュー
  - [ ] MIDIトリガー
  - [ ] OSCキュー
  - [ ] OSCトリガー
  - [ ] 動画キュー (未定)

### GUI (sbsp_tauri)

- 基本
  - [ ] apiserverクライアント
- キーボードショートカット
  - [ ] 開く、保存
  - [ ] キューリスト移動（↑↓）
  - [ ] キューリスト全選択 (Ctrl+A)
  - [ ] キュー削除 (Ctrl+Delete)
- メニュー項目
  - [ ] キューリスト全選択
  - [ ] キュー削除
  - [ ] 各キューの追加

## ビルド (sbsp_tauri)

### 前準備

- Tauri Prerequisites

[https://v2.tauri.app/ja/start/prerequisites/](https://v2.tauri.app/ja/start/prerequisites/)

上記のページを元に Rust, Node.js, その他の依存パッケージをインストール

- [pnpm](https://pnpm.io/ja/installation)

上記のページを元に pnpm をインストール

### ビルド手順

1. リポジトリをクローン

    ```bash
    git clone https://github.com/Keinsleif/sbsp.git
    cd sbsp/sbsp_tauri
    ```

2. Node.js 依存関係をインストール

    ```bash
    pnpm i
    ```

3. ビルドを実行

    ```bash
    pnpm tauri build --release
    ```

    > [!NOTE]
    > 開発のためにホットリロードやDevtoolsを利用するときは以下のコマンドで実行まで行えます。
    >
    > ```bash
    > pnpm tauri dev
    > ```

4. `sbsp/target/release/` に実行ファイルが出力されます。
