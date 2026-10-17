# UI fonts

`sbsp_ui` embeds these files (see `src/theme/fonts.rs`). They are not
committed by the patches; add them by hand:

| File | Use |
|---|---|
| `BIZUDPGothic-Regular.ttf` | UI text (family "BIZ UDPGothic") |
| `BIZUDGothic-Regular.ttf` | fixed width: clock, elapsed / remaining time (family "BIZ UDGothic") |
| `OFL.txt` | license text of the two fonts |

Source: https://github.com/googlefonts/morisawa-biz-ud-gothic
(SIL Open Font License 1.1). Take the files from its `fonts/ttf/` folder
and the license from `OFL.txt` in the repository root.

Check the family names after adding (`fc-scan --format '%{family}\n' <file>`):
they must read `BIZ UDPGothic` and `BIZ UDGothic`, or the constants in
`fonts.rs` have to follow.
