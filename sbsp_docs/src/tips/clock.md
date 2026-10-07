# 高精度な時計の表示

メイン画面の時計ではコンピューターの時刻を表示するが、macOSやLinuxと比べてWindows環境では1秒～30秒ほどの誤差が生じることがある。

これを改善するために時刻同期に関する設定を変更する必要がある。

このページでは

- W32TIME (Windowsデフォルト) の設定変更
- Meinberg NTPソフトウェア の導入

の2つの手段での高精度時刻同期を行う方法を紹介する。

## W32TIME (Windowsデフォルト) の設定変更での対応

コマンドプロンプトを管理者として起動し、以下のコマンドを実行する

```text
sc triggerinfo w32time start/networkon stop/networkoff
```

レジストリエディタで以下のキーを選択

```text
HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Services\W32Time\Parameters
```

表示されたキーの内、`NtpServer`を以下の値に設定

```text
time.cloudflare.com,0x8 time.windows.com,0x8
```

レジストリエディタで以下のキーを選択

```text
HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Services\W32Time\Config
```

`UpdateInterval`を`0x64` (`100`)に、
`PhaseCorrectRate`を`7`に設定

> 上記の設定内容に関する解説は以下のページに存在する。
>
> [Windows タイム サービスのツールと設定](https://learn.microsoft.com/ja-jp/windows-server/networking/windows-time-service/windows-time-service-tools-and-settings?tabs=parameters)

## Meinberg NTP ソフトウェアによる対応

下記のページからインストーラをダウンロード

<https://www.meinbergglobal.com/english/sw/ntp.htm> (English)

インストール中の設定に関する詳細な解説は以下のサイトに存在する。

<https://www.meinbergglobal.com/english/sw/readme-ntpinstaller.htm> (English)

これ以降は上記の手順から推奨する手順でのインストール方法を紹介する。
