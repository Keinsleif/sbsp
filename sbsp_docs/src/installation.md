---
title: インストール方法
description: SBS Playerの入手方法
---

# インストール方法

以下の配布元からインストーラまたはアプリ本体を入手できます。

<DownloadButton />

もしくは、以下のページよりインストーラーのZIPファイルを入手できます。

## BOOTH

[BOOTH 商品ページ](https://keinsleif.booth.pm/items/7619260)

Windows向けには for Windows
macOS向けには for macOS

を選択します。

## GitHub Releases

[GitHub Releases](https://github.com/Keinsleif/sbsp/releases)

上記のページの`Assets`より、OSに応じたファイルをダウンロードします。

Windows向けには `SBSP-Host-Windows-Setup.exe`（インストールせずに使う場合は `SBSP-Host-Windows-Portable.zip`）

macOS向けには `SBSP-Host-macOS-Setup.pkg`（または `SBSP-Host-macOS-Portable.zip`）

Linux向けには `SBSP-Host-Linux.AppImage`

をダウンロードします。

## macOSで開けない場合

現在のmacOS向けの配布物は、Appleの署名・公証を受けていません。そのため、ダウンロードしたファイルをそのまま開くと「開発元を検証できません」などと表示され、起動が止められることがあります。次のいずれかの方法で開きます。

### `.pkg` のインストーラを開く

1. `SBSP-Host-macOS-Setup.pkg` をダブルクリックし、表示された警告は「完了」で閉じます。
2. 「システム設定」→「プライバシーとセキュリティ」を開き、下の方にある「"SBSP-Host-macOS-Setup.pkg" は開くことをブロックされました」の横の「このまま開く」を押します。
3. 認証のあと、インストーラが起動するので、案内に従ってインストールします。

macOS 14 以前では、`.pkg` を右クリック（Controlキーを押しながらクリック）して「開く」を選び、確認画面で「開く」を押す方法でも開けます。

### ZIPで配布されたアプリを使う

1. `SBSP-Host-macOS-Portable.zip` を展開し、できたアプリを「アプリケーション」フォルダへ移動します。
2. 移動したアプリををダブルクリックし、表示された警告は「完了」で閉じます。
3. 「システム設定」→「プライバシーとセキュリティ」を開き、下の方にある「"<アプリ名>" は開くことをブロックされました」の横の「このまま開く」を押します。
4. 認証のあと、アプリが起動します．

この手順は初回インストール時のみ必要です．

また，署名と公証を受けた配布物が用意されたときは、この手順は不要になります。

インストール後は、メニューの「ヘルプ」→「アップデートを確認」から、新しいバージョンの確認とインストールができます。

<script setup>
    import DownloadButton from './components/DownloadButton.vue';
</script>
