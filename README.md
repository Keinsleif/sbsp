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
  - [x] postWait、sequence(auto-follow)の実装
  - [x] auto-followターゲット指定
  - [x] キュー名のデフォルト機能（内容から自動生成）
  - [x] preWait中のロード
  - [ ] cursor設定後のロード
  - [ ] cursor解除後のアンロード
  - [x] シーク
- 音声
  - [x] 再生、一時停止、再開、停止
  - [x] ロード
  - [ ] アンロード
  - [x] 開始位置、終了位置
  - [x] フェードイン、フェードアウト
  - [x] 長さ解析
  - [x] 波形解析
  - [x] リピート&リピート停止(CueActionで対応)
  - [ ] 音量調整(AudioEngine全体)
  - [x] streaming sound 対応(手動設定)
  - [x] モノラル出力
- その他
  - [x] 待機キュー
  - [ ] グループキュー
  - [ ] 再生、一時停止、停止キュー
  - [ ] MIDIキュー
  - [ ] MIDIトリガー
  - [ ] OSCキュー
  - [ ] OSCトリガー
- 内部実装
  - [x] controller_txからcontroller_handleへ変更
  - [ ] stopping, pausing状態を追加
  - [x] 実行中のキュー削除時の対応
  - [x] HardStopの実装(EngineにおいてStop中のStopコマンド処理で対応)
- リモート処理
  - [x] apiclientの実装
  - [x] apiclient、切断処理、切断時処理
  - [x] mDNSによるLANディスカバリー
  - [ ] 閲覧/編集 権限管理
  - [ ] ping

### GUI (sbsp_tauri)

- 基本
  - [x] スリープ防止
  - [x] キューパラメータ編集
  - [x] フォーカスなしインジケータ
  - [x] キューアクションのUI実装
  - [ ] 閲覧/編集モード
- キーボードショートカット
  - [x] 開く、保存
  - [x] キューリスト移動（↑↓）
  - [x] キューリスト全選択 (Ctrl+A)
  - [x] キュー削除 (Ctrl+Delete)
- メニュー項目
  - [x] キューリスト全選択
  - [x] キュー削除
  - [ ] 各キューの追加
- 設定画面
  - [ ] キューテンプレート編集機能
- その他
  - [x] vueuse使用に書き換え
  - [x] i18n対応(vue-i18n)
- リモート機能
  - [x] サーバー管理パネル

### Remote GUI (sbsp_remote)

- 基本
  - [x] 接続

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
