# timenow

ターミナルに巨大なデジタル時計(HH:MM)をフルスクリーン表示するTUIアプリ。
コロンが秒単位で点滅。`q` または `Enter` で終了。端末リサイズに追従して最大サイズで描画。
実行時キー操作でパターン(フォント)・シンボル(ON/OFF文字)・前景色・背景色を切り替え可能。

## キー操作

| キー | 機能 |
|------|------|
| `p` | パターン(フォント)切替: Block / Outline(中抜き輪郭) |
| `s` | シンボル切替: `#` / `█` / `▓░` / `*.` / `+-`(すべて半角) |
| `c` | 前景色切替(ANSI 8色 + Default) |
| `b` | 背景色切替(ANSI 8色 + Default) |
| `h` / `H` / `Esc` | ヘルプダイアログ表示/非表示 |
| `o` / `O` | 設定ページ(modal)表示/非表示 |
| `q` / `Q` / `Enter` | 終了 |

### 設定ページ(`o` で開く、modal表示)

- 全選択肢(Pattern / Symbol / Foreground / Background)がボックス描画文字(┌─┐│├┤└┘)の枠線付き modal 内に縦に並ぶ
- セクション(Pattern/Symbol/Foreground/Background)ごとに太字ヘッダ + `├──┤` 区切り線
- 各選択肢行: `▶`(カーソル) / `●`(選択中) / `○`(未選択) のラジオドット風マーカ + ラベル + プレビュー
- カーソル行は太字 `\x1b[1m` で強調。背景反転・下線は使わない(プレビュー内の `\x1b[0m` 後に再適用してハイライトを保持)
- `↑` `↓` でカーソル移動(移動するだけでは反映しない)
- `Space` または **マウス左クリック** で決定 — カーソル/クリック位置の選択肢を style/theme に反映
- `Enter` / `o` / `Esc` / `q` で modal を閉じる
- ビューポートより長い場合はスクロール(カーソルが中央付近に来るよう追従)
- マウスクリックのヒットテストは `layout_item_at(layout, top, left, row, col)` で行う(選択肢行のみ反応、枠/ヘッダ/区切り/フッタは無視)

## ビルド / 実行

```sh
cargo run
```

## テスト

```sh
cargo test
```

## 公開前品質チェック

```sh
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo build --release
```

## 設計

- `src/digits.rs` — 5x5 ビットマップデジタルフォント。`Font` enum で複数パターン(Block/Outline/Segment/Dot/Slant)を切り替え(純粋)
- `src/clock.rs` — 時刻フォーマット・コロン点滅・スケール計算・巨大描画・`Style`/`CharSet`/`Color`/`Theme`(純粋、単体テストあり)
- `src/config.rs` — 設定ファイルの parse/format とデフォルト設定(純粋、単体テストあり)
- `src/main.rs` — crossterm によるフルスクリーンTUI(入力/リサイズ/再描画ループ・キー操作でスタイル/色をcycling)

### レスポンシブ描画の仕組み

- 1ピクセル = ターミナル2列 x 1行(セル縦横比≈2:1 を補正してほぼ正方形に)
- "HH:MM" のピクセル幅 = 5グリフ*5 + 4ギャップ*1 = 29px => 58列(scale=1)
- `compute_scale(cols, rows)` が端末サイズに収まる最大 scale を算出
- 変化(分更新/コロン点滅/リサイズ/スタイル・色変更)があったときだけ再描画
- 文字の表示幅は `unicode-width` クレートで判定。半角文字(幅1)なら2個、全角文字(幅2)なら1個で1ピクセル(2列)を構成し、レイアウト崩れを防ぐ
- 中央揃えの計算も `visible_width` (ANSIエスケープ除外 + unicode-width) を使用

### カスタマイズの仕組み

- `Font`(Block/Outline) と `CharSet`(on/off文字ペア) は `next()` でcycling。文字幅は unicode-width で判定されるため全角・半角混在でもレイアウトが崩れない
- `Color`(ANSI 8色+Default) は前景・背景それぞれ独立にcycling
- `theme_prefix(Theme)` が前景+背景の ANSI SGI シーケンスを生成(描画前に出力、描画後に `\x1b[0m` でリセット)
- 背景色は画面クリア時に現在の背景色で埋める端末挙動を利用して全面塗りつぶし
- `help_dialog(width)` が枠線付きヘルプダイアログの行を生成(純粋関数、単体テストあり)。`h` キーでオーバーレイ表示/非表示
- `setting_items(style, theme)` が全選択肢リストを生成(純粋関数)。`settings_layout(items, cursor, vp_h)` がボックス文字・セクション区切り・太字ヘッダ・カーソル反転ハイライト付きの modal レイアウト(`SettingsLayout`: 行種別+テキスト+寸法)を生成(純粋関数、単体テストあり)。`settings_modal(...)` はレイアウトのテキストのみを返す薄いラッパ。`layout_item_at(layout, top, left, row, col)` がマウスクリックのヒットテスト(純粋関数、単体テストあり)。`o` キーで modal 表示、↑↓で移動、`Space`/マウスクリックで決定(反映)
