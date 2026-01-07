# SBS Player

創価大学放送部（SBS）での使用を想定した音源再生ソフトです。  
Mac用の舞台オペレーティングソフトウェアである [QLab](https://qlab.app/) にインスパイアされています。  

## ドキュメント

使用方法のについては、この[ドキュメント](https://keinsleif.github.io/sbsp/docs/)を参照

## ビルド

### 前準備

- Tauri Prerequisites

[https://v2.tauri.app/ja/start/prerequisites/](https://v2.tauri.app/ja/start/prerequisites/)

上記のページを元に Rust, Node.js, その他の依存パッケージをインストール

> [!NOTE]
> Linuxにおいては、追加で以下の依存関係が必要です。
>
> > Debian/Ubuntu/Mint では `libasound2-dev`、Fedora/Centos では`alsa-lib-devel` パッケージ

- pnpm

[https://pnpm.io/ja/installation](https://pnpm.io/ja/installation)

上記のページを元に pnpm をインストール

- Tauri CLI

以下のコマンドでTauri CLIをインストール

```bash
cargo install tauri-cli --version "^2.0.0" --locked
```

### ビルド手順

1. リポジトリをクローン

    ```bash
    git clone https://github.com/Keinsleif/sbsp.git
    ```

2. Node.js 依存関係をインストール

    ```bash
    cd sbsp/sbsp_frontend
    pnpm i
    ```

3. ビルドを実行

    ```bash
    # SBS Player をビルド
    cd ../sbsp_tauri

    cargo tauri build

    # SBS Player Remote をビルド
    cd ../sbsp_remote

    cargo tauri build
    ```

> [!NOTE]
> 開発のためにホットリロードやDevtoolsを利用するときは以下のコマンドで実行まで行えます。
>
> ```bash
> cargo tauri dev
> ```

  `sbsp/target/release/` に実行ファイルが出力されます。
