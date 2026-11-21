# SBS Player

> Stage Back Sound Player

SBS Player は、QLab、MusicBee、Multiplayといったソフトウェアにインスパイアされたイベント向け音源再生ソフトウェアです。

## 機能

- 音源の登録、再生
- 主要な音源ファイルへの対応（caf, mp4, mka, webm, ogg, wav, aac, alac, flac, mp3, opus）
- フェードイン・フェードアウトの設定
- 個別のリピート、音量、再生範囲等の設定
- 複数音源のリピート
- ファイルへの保存/読み込み
- 音源も含めたフォルダへのエクスポート
- 時計の表示
- スリープ防止

など

## ドキュメント

使用方法のについては、この[ドキュメント](https://keinsleif.github.io/sbsp/docs/)を参照

## ビルド

ツールの管理には [mise](https://mise.jdx.dev/) を使います。

### 前準備

1. [mise](https://mise.jdx.dev/getting-started.html#installing-mise-cli) をインストール

2. リポジトリをクローン

    ```bash
    git clone https://github.com/Keinsleif/sbsp.git
    cd sbsp
    ```

3. 依存ツール，パッケージをインストール（Rust、trunk、Python など）

    ```bash
    mise bootstrap
    ```

### ビルド手順

1. ビルドの実行

    ```bash
    # SBS Player をビルド（Web 版も含めてビルドされます）
    mise run build_host

    # SBS Player Remote をビルド
    mise run build_remote
    ```

    `target/release/` に `sbsp_host` と `sbsp_remote` が出力されます。

2. インストーラーとアップデート用のパッケージを作成（任意）

    ```bash
    mise run pack_host
    mise run pack_remote
    ```

    `target/dist/` に出力されます。

> [!NOTE]
> Web 版だけを開発するときは `mise run dev_web` でブラウザ向けの開発サーバーが起動します。

## プライバシーポリシー

SBS Player（以下「本ソフトウェア」）は、ユーザーのプライバシーを尊重し、個人情報や利用データを**一切収集・送信・保存しません**。

- **データの収集・送信なし**  
    氏名、メールアドレス、位置情報、端末識別子、利用状況、クラッシュログなどのいかなるデータも外部サーバーへ送信・収集しません。

- **完全ローカル処理**  
    入力または生成されたすべてのデータはユーザーの端末内、LAN内でのみ処理・保存され、開発者がアクセスすることはありません。

- **第三者ツールの不使用**  
    アクセス解析（Google Analytics等）や広告配信などの外部SDK・トラッキングツールは一切使用していません。

プライバシーに関するお問い合わせは、本リポジトリの [Issues](https://github.com/Keinsleif/sbsp/issues) または開発者までご連絡ください。

## ライセンス

本プロジェクトはGNU General Public License v3.0の下でライセンスされています。詳細は[LICENSE](LICENSE)ファイルを参照してください。  

### 使用、研究、共有、改良の自由

このソフトウェアは、あなたの自由を尊重します。あなたは以下のことができます。

- どのような目的であれ、ソフトウェアを使用すること
- 仕組みを研究し、自身のニーズに合わせて改変すること
- 他の人々とコピーを共有すること
- ソフトウェアを改良し、その改良を他の人々と共有すること

派生するすべての著作物も、GPL v3.0ライセンスの下で提供されなければなりません。
