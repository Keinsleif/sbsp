# SBS Player

創価大学放送部（SBS）での使用を想定した音源再生ソフトです。  
Mac用の舞台オペレーティングソフトウェアである [QLab](https://qlab.app/) にインスパイアされています。  

## ドキュメント

使用方法のについては、この[ドキュメント](https://keinsleif.github.io/sbsp/docs/)を参照

## ビルド

ツールの管理には [mise](https://mise.jdx.dev/) を使います。

### 前準備

1. [mise](https://mise.jdx.dev/getting-started.html#installing-mise-cli) をインストール

2. ツールをインストール（Rust、trunk、Python など）

    ```bash
    mise install
    ```

3. Linux のみ: 音声出力（ALSA）などのシステムパッケージをインストール

    ```bash
    mise bootstrap packages apply
    ```

### ビルド手順

1. リポジトリをクローン

    ```bash
    git clone https://github.com/Keinsleif/sbsp.git
    cd sbsp
    ```

2. ビルドを実行

    ```bash
    # SBS Player をビルド（Web 版も含めてビルドされます）
    mise run build_host

    # SBS Player Remote をビルド
    mise run build_remote
    ```

    `target/release/` に `sbsp_host` と `sbsp_remote` が出力されます。

3. インストーラーとアップデート用のパッケージを作成（任意）

    [.NET 8 SDK](https://dotnet.microsoft.com/) が必要です（`mise install` で入ります）。

    ```bash
    mise run pack_host
    mise run pack_remote
    ```

    `target/dist/` に出力されます。

> [!NOTE]
> Web 版だけを開発するときは `mise run dev_web` でブラウザ向けの開発サーバーが起動します。

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

### Freedom to use, study, share and improve

This software respects your freedom. You can:

- Use it for any purpose
- Study how it works and adapt it to your needs
- Share copies with others
- Improve the software and share your improvements with others

All derivative works must also be licensed under GPL v3.0.
