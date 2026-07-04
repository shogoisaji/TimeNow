//! 時計のコアロジック: 時刻フォーマット、コロン点滅、レスポンシブ描画、スタイル/カラー。
//! すべて純粋関数で単体テスト可能。

use crate::digits::{self, Font, GAP, GLYPH_H, GLYPH_W};
use unicode_width::UnicodeWidthChar;

/// 1ピクセルあたりのターミナル列数(セルの縦横比≈2:1 を補正するため2列で1ピクセル)。
pub const PIXEL_COLS: usize = 2;

/// ON/OFF ピクセル文字のペア(シンボルセット)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharSet {
    pub on: char,
    pub off: char,
}

impl CharSet {
    /// 従来のデフォルト: `#` と空白。
    pub const DEFAULT: CharSet = CharSet { on: '#', off: ' ' };

    /// 定義済みシンボルセットを順番に返す(cycling用)。
    /// すべて半角文字(セル幅1)で統一 — 全角文字が混じると列幅が崩れて左寄りになるため。
    pub fn all() -> &'static [CharSet] {
        &[
            CharSet { on: '#', off: ' ' },
            CharSet {
                on: '█', off: ' '
            },
            CharSet {
                on: '▓', off: '░'
            },
            CharSet { on: '*', off: '.' },
            CharSet { on: '+', off: '-' },
        ]
    }

    /// 次のシンボルセットへ(cycling)。
    pub fn next(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|&s| s == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }
}

impl Default for CharSet {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// 描画スタイル: フォント + シンボルセット。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub font: Font,
    pub chars: CharSet,
}

impl Style {
    /// デフォルトスタイル(太いブロック体 + ブロック文字)。
    pub const DEFAULT: Style = Style {
        font: Font::Neo,
        chars: CharSet {
            on: '█', off: ' '
        },
    };
}

/// ANSI 8色 + デフォルト。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Color {
    /// cycling用の色リスト。
    pub fn all() -> &'static [Color] {
        &[
            Color::Default,
            Color::Black,
            Color::Red,
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Magenta,
            Color::Cyan,
            Color::White,
        ]
    }

    /// 次の色へ(cycling)。
    pub fn next(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|&c| c == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    /// 前景色の ANSI SGR コード番号(30-37, Default=39)。
    fn fg_code(self) -> u8 {
        match self {
            Color::Default => 39,
            Color::Black => 30,
            Color::Red => 31,
            Color::Green => 32,
            Color::Yellow => 33,
            Color::Blue => 34,
            Color::Magenta => 35,
            Color::Cyan => 36,
            Color::White => 37,
        }
    }

    /// 背景色の ANSI SGR コード番号(40-47, Default=49)。
    fn bg_code(self) -> u8 {
        match self {
            Color::Default => 49,
            Color::Black => 40,
            Color::Red => 41,
            Color::Green => 42,
            Color::Yellow => 43,
            Color::Blue => 44,
            Color::Magenta => 45,
            Color::Cyan => 46,
            Color::White => 47,
        }
    }
}

/// 前景・背景色をまとめたテーマ。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub fg: Color,
    pub bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            fg: Color::Default,
            bg: Color::Default,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DateDisplay {
    None,
    #[default]
    Numeric,
}

impl DateDisplay {
    pub fn all() -> &'static [DateDisplay] {
        &[DateDisplay::Numeric, DateDisplay::None]
    }

    pub fn next(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|&d| d == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }
}

pub fn date_display_label(display: DateDisplay) -> &'static str {
    match display {
        DateDisplay::None => "None",
        DateDisplay::Numeric => "4/1",
    }
}

pub fn format_date(month: u32, day: u32, display: DateDisplay) -> Option<String> {
    match display {
        DateDisplay::None => None,
        DateDisplay::Numeric => Some(format!("{month}/{day}")),
    }
}

/// テーマの ANSI エスケープシーケンス(前景+背景の設定)を返す。
/// 描画開始時に一度出力し、描画後にリセット(`\x1b[0m`)すること。
pub fn theme_prefix(theme: Theme) -> String {
    format!("\x1b[{};{}m", theme.fg.fg_code(), theme.bg.bg_code())
}

/// ヘルプダイアログの本文行。枠線は呼び出し側が付ける。
pub fn help_lines() -> Vec<&'static str> {
    vec![
        "timenow controls",
        "",
        " p / s / c / b   cycle style",
        " o               open settings",
        " h / Esc         toggle help",
        " q / Enter       quit",
        "",
        "Settings: ←→ section  ↑↓ change  Click select",
    ]
}

/// 指定幅 `width` でヘルプダイアログを枠線付きで生成する。
/// 戻り値はダイアログの各行(枠線含む)。中央揃えは呼び出し側が行う。
/// `width` は本文の最大幅。実際のダイアログ幅 = width + 4(左右枠+余白)。
pub fn help_dialog(width: usize) -> Vec<String> {
    let lines = help_lines();
    let inner_w = lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
        .max(width);
    let total_w = inner_w + 4; // 左枠 + 余白1 + 本文 + 余白1 + 右枠

    let mut out = Vec::with_capacity(lines.len() + 2);
    // 上辺
    out.push(format!("┌{}┐", "─".repeat(total_w - 2)));
    for l in &lines {
        let pad = inner_w - l.chars().count();
        let left_pad = pad / 2;
        let right_pad = pad - left_pad;
        out.push(format!(
            "│ {}{}{} │",
            " ".repeat(left_pad),
            l,
            " ".repeat(right_pad)
        ));
    }
    // 下辺
    out.push(format!("└{}┘", "─".repeat(total_w - 2)));
    out
}

// --- 設定ページ ---

/// 設定項目の種類。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingKind {
    Date,
    Pattern,
    Symbol,
    Foreground,
    Background,
}

impl SettingKind {
    /// ヘッダ表示用ラベル。
    pub fn header(self) -> &'static str {
        match self {
            SettingKind::Date => "Date",
            SettingKind::Pattern => "Pattern",
            SettingKind::Symbol => "Symbol",
            SettingKind::Foreground => "Foreground",
            SettingKind::Background => "Background",
        }
    }

    pub fn all() -> &'static [SettingKind] {
        &[
            SettingKind::Date,
            SettingKind::Pattern,
            SettingKind::Symbol,
            SettingKind::Foreground,
            SettingKind::Background,
        ]
    }
}

/// 設定ページの1選択肢。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingItem {
    pub kind: SettingKind,
    /// 選択肢の表示名。
    pub label: String,
    /// プレビュー文字列(ANSIエスケープ含む場合あり)。
    pub preview: String,
    /// 現在の設定値と一致するか。
    pub selected: bool,
    /// この選択肢を適用したときの新しいスタイル。
    pub style: Style,
    /// この選択肢を適用したときの新しいテーマ。
    pub theme: Theme,
    /// この選択肢を適用したときの日付表示。
    pub date_display: DateDisplay,
}

/// フォントの表示名。
pub fn font_label(font: Font) -> &'static str {
    match font {
        Font::Neo => "Neo",
        Font::Block => "Block",
        Font::Outline => "Outline",
        Font::Segment => "Segment",
        Font::Dot => "Dot",
        Font::Slant => "Slant",
    }
}

/// フォントのプレビュー: "2" の中段を描画(Block は全幅、Outline は中抜き)。
fn font_preview(font: Font) -> String {
    let g = crate::digits::render_glyph(font, '2');
    let row = g[g.len() / 2];
    let mut s = String::new();
    for ch in row.chars() {
        let c = if ch == '#' { '█' } else { ' ' };
        for _ in 0..PIXEL_COLS {
            s.push(c);
        }
    }
    s
}

/// CharSet の表示名。
pub fn charset_label(cs: CharSet) -> String {
    let off_name = if cs.off == ' ' {
        "space".to_string()
    } else {
        cs.off.to_string()
    };
    format!("{} / {}", cs.on, off_name)
}

/// CharSet のプレビュー: on文字5個 + off文字2個。
fn charset_preview(cs: CharSet) -> String {
    let mut s = String::new();
    for _ in 0..5 {
        s.push(cs.on);
    }
    for _ in 0..2 {
        s.push(cs.off);
    }
    s
}

/// Color の表示名。
pub fn color_label(color: Color) -> &'static str {
    match color {
        Color::Default => "Default",
        Color::Black => "Black",
        Color::Red => "Red",
        Color::Green => "Green",
        Color::Yellow => "Yellow",
        Color::Blue => "Blue",
        Color::Magenta => "Magenta",
        Color::Cyan => "Cyan",
        Color::White => "White",
    }
}

/// 前景色のプレビュー: その色で "Sample" を描画。
fn color_fg_preview(color: Color) -> String {
    format!("\x1b[{}mSample\x1b[0m", color.fg_code())
}

/// 背景色のプレビュー: その色の背景で5個の空白(色块)。
fn color_bg_preview(color: Color) -> String {
    format!(
        "\x1b[{}m     \x1b[0m {}",
        color.bg_code(),
        color_label(color)
    )
}

pub fn selected_index_for_kind(items: &[SettingItem], kind: SettingKind) -> Option<usize> {
    items
        .iter()
        .enumerate()
        .find(|(_, item)| item.kind == kind && item.selected)
        .map(|(idx, _)| idx)
}

pub fn first_index_for_kind(items: &[SettingItem], kind: SettingKind) -> Option<usize> {
    items
        .iter()
        .enumerate()
        .find(|(_, item)| item.kind == kind)
        .map(|(idx, _)| idx)
}

pub fn active_setting_kind(items: &[SettingItem], cursor: usize) -> SettingKind {
    items
        .get(cursor)
        .map(|item| item.kind)
        .unwrap_or(SettingKind::Pattern)
}

/// 設定ページの全選択肢リストを生成する。
/// 現在の `style` と `theme` に基づいて `selected` を設定する。
pub fn setting_items(style: Style, theme: Theme, date_display: DateDisplay) -> Vec<SettingItem> {
    let mut items = Vec::new();

    for &display in DateDisplay::all() {
        items.push(SettingItem {
            kind: SettingKind::Date,
            label: date_display_label(display).to_string(),
            preview: format_date(4, 1, display).unwrap_or_else(|| "hidden".to_string()),
            selected: date_display == display,
            style,
            theme,
            date_display: display,
        });
    }

    // Pattern (Font)
    for &font in Font::all() {
        items.push(SettingItem {
            kind: SettingKind::Pattern,
            label: font_label(font).to_string(),
            preview: font_preview(font),
            selected: style.font == font,
            style: Style { font, ..style },
            theme,
            date_display,
        });
    }

    // Symbol (CharSet)
    for &cs in CharSet::all() {
        items.push(SettingItem {
            kind: SettingKind::Symbol,
            label: charset_label(cs),
            preview: charset_preview(cs),
            selected: style.chars == cs,
            style: Style { chars: cs, ..style },
            theme,
            date_display,
        });
    }

    // Foreground (Color)
    for &color in Color::all() {
        items.push(SettingItem {
            kind: SettingKind::Foreground,
            label: color_label(color).to_string(),
            preview: color_fg_preview(color),
            selected: theme.fg == color,
            style,
            theme: Theme { fg: color, ..theme },
            date_display,
        });
    }

    // Background (Color)
    for &color in Color::all() {
        items.push(SettingItem {
            kind: SettingKind::Background,
            label: color_label(color).to_string(),
            preview: color_bg_preview(color),
            selected: theme.bg == color,
            style,
            theme: Theme { bg: color, ..theme },
            date_display,
        });
    }

    items
}

/// 設定 modal の行種別。マウスのヒットテストで使用する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsRowKind {
    TopBorder,
    Title,
    Blank,
    Header,
    Divider,
    /// 選択肢行。`index` は `setting_items` のインデックス。
    Item {
        index: usize,
    },
    Footer,
    BottomBorder,
}

/// 設定 modal の1行(種別 + 描画テキスト)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsRow {
    pub kind: SettingsRowKind,
    pub text: String,
}

/// 設定 modal のレイアウト(全行 + 寸法)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsLayout {
    pub rows: Vec<SettingsRow>,
    pub width: usize,
    pub height: usize,
}

/// 選択肢行の本文(枠線なし)を生成する。
/// `▶` = カーソル、`●` = 選択中、`○` = 未選択(ラジオドット風)。
fn item_inner(item: &SettingItem, is_cursor: bool, label_w: usize) -> String {
    let cm = if is_cursor { "▶" } else { " " };
    let check = if item.selected { "●" } else { "○" };
    format!(
        "{} {} {:<w$}  {}",
        cm,
        check,
        item.label,
        item.preview,
        w = label_w
    )
}

/// 設定 modal のレイアウトを生成する。
/// タブでカテゴリを切り替え、選択中カテゴリの項目だけを表示する。
/// `viewport_h` は modal 全体の最大表示行数。小さい端末では項目だけをスクロールする。
pub fn settings_layout(items: &[SettingItem], cursor: usize, viewport_h: usize) -> SettingsLayout {
    let cursor = cursor.min(items.len().saturating_sub(1));
    let active = active_setting_kind(items, cursor);
    let active_indices: Vec<usize> = items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| (item.kind == active).then_some(idx))
        .collect();

    let label_w = items
        .iter()
        .map(|i| i.label.chars().count())
        .max()
        .unwrap_or(0);
    let title = " Settings ";
    let footer = " ←→ section  ↑↓ change  Space/Click apply  Enter/o/Esc close ";
    let tabs = setting_tabs(active);
    let summary = settings_summary(items);
    let header = format!("  {} options", active.header());

    let fixed_rows = 9usize; // top, summary, tabs, divider, header, blank, divider, footer, bottom
    let max_items = viewport_h
        .saturating_sub(fixed_rows)
        .max(1)
        .min(active_indices.len().max(1));
    let active_pos = active_indices
        .iter()
        .position(|&idx| idx == cursor)
        .unwrap_or(0);
    let half = max_items / 2;
    let offset = if active_indices.len() <= max_items || active_pos <= half {
        0
    } else if active_pos + half >= active_indices.len() {
        active_indices.len().saturating_sub(max_items)
    } else {
        active_pos - half
    };

    let visible_indices = active_indices
        .iter()
        .skip(offset)
        .take(max_items)
        .copied()
        .collect::<Vec<_>>();

    let mut body: Vec<SettingsRow> = Vec::new();
    for idx in visible_indices {
        let item = &items[idx];
        body.push(SettingsRow {
            kind: SettingsRowKind::Item { index: idx },
            text: item_inner(item, idx == cursor, label_w),
        });
    }

    let body_w = body
        .iter()
        .map(|r| visible_width(&r.text))
        .max()
        .unwrap_or(0);
    let inner_w = body_w
        .max(title.chars().count())
        .max(footer.chars().count());
    let inner_w = inner_w
        .max(visible_width(&tabs))
        .max(visible_width(&summary))
        .max(visible_width(&header));
    let total_w = inner_w + 4; // │ + space + inner + space + │

    // 本文を inner_w に揃えて左右枠を付ける
    let pad_row = |content: &str| -> String {
        let vw = visible_width(content);
        let pad = inner_w.saturating_sub(vw);
        format!("│ {}{} │", content, " ".repeat(pad))
    };

    let mut rows: Vec<SettingsRow> = Vec::new();

    // 上辺(タイトル入り)
    let title_w = title.chars().count();
    let dash_total = total_w - 2;
    let dash_l = (dash_total.saturating_sub(title_w)) / 2;
    let dash_r = dash_total - title_w - dash_l;
    rows.push(SettingsRow {
        kind: SettingsRowKind::TopBorder,
        text: format!("┌{}{}{}┐", "─".repeat(dash_l), title, "─".repeat(dash_r)),
    });
    rows.push(SettingsRow {
        kind: SettingsRowKind::Blank,
        text: pad_row(&summary),
    });
    rows.push(SettingsRow {
        kind: SettingsRowKind::Title,
        text: pad_row(&tabs),
    });
    rows.push(SettingsRow {
        kind: SettingsRowKind::Divider,
        text: format!("├{}┤", "─".repeat(total_w - 2)),
    });
    rows.push(SettingsRow {
        kind: SettingsRowKind::Header,
        text: pad_row(&format!("\x1b[1m{}\x1b[22m", header)),
    });

    // 本文
    for r in &body {
        if let SettingsRowKind::Item { index } = r.kind {
            let is_cursor = index == cursor;
            let padded = pad_row(&r.text);
            if is_cursor {
                let with_attr = padded.replace("\x1b[0m", "\x1b[0m\x1b[1m");
                rows.push(SettingsRow {
                    kind: SettingsRowKind::Item { index },
                    text: format!("\x1b[1m{}\x1b[22m", with_attr),
                });
            } else {
                rows.push(SettingsRow {
                    kind: SettingsRowKind::Item { index },
                    text: padded,
                });
            }
        }
    }

    // 空行 + フッタ区切り + フッタ + 下辺
    rows.push(SettingsRow {
        kind: SettingsRowKind::Blank,
        text: format!("│{}│", " ".repeat(total_w - 2)),
    });
    rows.push(SettingsRow {
        kind: SettingsRowKind::Divider,
        text: format!("├{}┤", "─".repeat(total_w - 2)),
    });
    let footer_pad = inner_w.saturating_sub(footer.chars().count());
    let footer_pad_l = footer_pad / 2;
    let footer_pad_r = footer_pad - footer_pad_l;
    rows.push(SettingsRow {
        kind: SettingsRowKind::Footer,
        text: format!(
            "│ {}{}{} │",
            " ".repeat(footer_pad_l),
            footer,
            " ".repeat(footer_pad_r)
        ),
    });
    rows.push(SettingsRow {
        kind: SettingsRowKind::BottomBorder,
        text: format!("└{}┘", "─".repeat(total_w - 2)),
    });

    let height = rows.len();
    SettingsLayout {
        rows,
        width: total_w,
        height,
    }
}

fn setting_tabs(active: SettingKind) -> String {
    let mut tabs = Vec::new();
    for &kind in SettingKind::all() {
        if kind == active {
            tabs.push(format!("\x1b[1m[{}]\x1b[22m", kind.header()));
        } else {
            tabs.push(format!(" {} ", kind.header()));
        }
    }
    tabs.join("  ")
}

fn settings_summary(items: &[SettingItem]) -> String {
    let mut parts = Vec::new();
    for &kind in SettingKind::all() {
        let label = items
            .iter()
            .find(|item| item.kind == kind && item.selected)
            .map(|item| item.label.as_str())
            .unwrap_or("-");
        parts.push(format!("{}: {}", kind.header(), label));
    }
    parts.join("  ")
}

/// 設定 modal を枠線付きで生成する(行テキストのみ)。
/// `settings_layout` の結果からテキストを取り出したもの。中央揃えは呼び出し側が行う。
pub fn settings_modal(items: &[SettingItem], cursor: usize, viewport_h: usize) -> Vec<String> {
    settings_layout(items, cursor, viewport_h)
        .rows
        .into_iter()
        .map(|r| r.text)
        .collect()
}

/// クリック位置から選択肢インデックスを解決する(マウスヒットテスト)。
/// `top`/`left` は modal の左上のターミナル座標(0-based)。
/// `row`/`col` はクリックされたターミナル座標(0-based)。
/// 選択肢行以外(枠/ヘッダ/区切り/フッタ)のクリックは `None`。
pub fn layout_item_at(
    layout: &SettingsLayout,
    top: usize,
    left: usize,
    row: usize,
    col: usize,
) -> Option<usize> {
    if col < left || col >= left + layout.width {
        return None;
    }
    for (i, r) in layout.rows.iter().enumerate() {
        if let SettingsRowKind::Item { index } = r.kind {
            if top + i == row {
                return Some(index);
            }
        }
    }
    None
}

/// ANSIエスケープシーケンスを除外した表示幅(unicode-width準拠)を返す。
pub fn visible_width_pub(s: &str) -> usize {
    visible_width(s)
}

/// ANSIエスケープシーケンスを除外した表示幅を返す。
fn visible_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    // ANSIエスケープを取り除いてから幅を計算
    let stripped = strip_ansi(s);
    stripped.width()
}

/// ANSIエスケープシーケンス(\x1b[...m)を取り除いた文字列を返す。
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for sc in chars.by_ref() {
                    if sc == 'm' {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// 24時間制の "HH:MM" 文字列を返す。
pub fn format_hhmm(hour: u32, minute: u32) -> String {
    format!("{:02}:{:02}", hour, minute)
}

/// 秒が偶数のときコロン点灯、奇数のとき消灯(1秒ごとに点滅)。
pub fn colon_visible(second: u32) -> bool {
    second.is_multiple_of(2)
}

/// "HH:MM" を描画するのに必要なターミナル列数を scale 倍で返す。
/// レイアウト: 5グリフ + 4ギャップ。ピクセル幅 = 5*5 + 4*1 = 29。
pub fn rendered_cols(scale: usize) -> usize {
    let px_w = 5 * GLYPH_W + 4 * GAP;
    px_w * PIXEL_COLS * scale
}

/// 描画するのに必要なターミナル行数を scale 倍で返す。
pub fn rendered_rows(scale: usize) -> usize {
    GLYPH_H * scale
}

pub fn rendered_cols_for_style(scale: usize, style: &Style) -> usize {
    let px_w = 5 * digits::glyph_width(style.font) + 4 * GAP;
    px_w * PIXEL_COLS * scale
}

pub fn rendered_rows_for_style(scale: usize, style: &Style) -> usize {
    digits::glyph_height(style.font) * scale
}

/// ターミナルサイズ (cols, rows) に収まる最大の scale を返す(最小1)。
pub fn compute_scale(cols: usize, rows: usize) -> usize {
    if cols == 0 || rows == 0 {
        return 1;
    }
    let by_w = cols / rendered_cols(1);
    let by_h = rows / rendered_rows(1);
    let s = by_w.min(by_h);
    if s == 0 {
        1
    } else {
        s
    }
}

pub fn compute_scale_for_style(cols: usize, rows: usize, style: &Style) -> usize {
    if cols == 0 || rows == 0 {
        return 1;
    }
    let by_w = cols / rendered_cols_for_style(1, style);
    let by_h = rows / rendered_rows_for_style(1, style);
    let s = by_w.min(by_h);
    if s == 0 {
        1
    } else {
        s
    }
}

/// 1ピクセル分の文字を `target_cols` 列幅になるように繰り返し追加する。
/// 半角文字(幅1)なら2個、全角文字(幅2)なら1個追加する。
fn push_pixel(line: &mut String, ch: char, target_cols: usize) {
    let w = ch.width().unwrap_or(1).max(1);
    let count = target_cols / w;
    for _ in 0..count {
        line.push(ch);
    }
}

/// `text`(例: "12:34") を `scale` 倍で巨大描画した行の Vec を返す。
/// `colon_on` が false のとき ':' のピクセルを空白にする(点滅)。
/// `style` でフォントと ON/OFF 文字を切り替え。
/// 文字の表示幅(unicode-width)を考慮し、1ピクセル = PIXEL_COLS列 になるよう調整。
pub fn render(text: &str, colon_on: bool, scale: usize, style: &Style) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let glyphs: Vec<&'static [&'static str]> = chars
        .iter()
        .map(|&c| digits::render_glyph(style.font, c))
        .collect();
    // コロン位置を事前計算(ピクセル内ループでの chars().nth() を回避)
    let is_colon: Vec<bool> = chars.iter().map(|&c| c == ':').collect();
    let glyph_h = digits::glyph_height(style.font);
    let glyph_w = digits::glyph_width(style.font);
    let mut lines = Vec::with_capacity(rendered_rows_for_style(scale, style));
    let px_cols = PIXEL_COLS * scale;
    let gap_cols = GAP * PIXEL_COLS * scale;
    for py in 0..glyph_h {
        for _ in 0..scale {
            let mut line = String::new();
            for (gi, g) in glyphs.iter().enumerate() {
                let row = g[py];
                for px in 0..glyph_w {
                    let on = row.as_bytes()[px] == b'#';
                    let ch = if on { style.chars.on } else { style.chars.off };
                    // ':' のとき colon_on=false なら空白にする
                    let draw = if is_colon[gi] && !colon_on {
                        style.chars.off
                    } else {
                        ch
                    };
                    push_pixel(&mut line, draw, px_cols);
                }
                // ギャップ(最後のグリフの後には入れない)
                if gi + 1 < glyphs.len() {
                    push_pixel(&mut line, style.chars.off, gap_cols);
                }
            }
            lines.push(line);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_zero_padded() {
        assert_eq!(format_hhmm(0, 0), "00:00");
        assert_eq!(format_hhmm(9, 5), "09:05");
        assert_eq!(format_hhmm(23, 59), "23:59");
    }

    #[test]
    fn colon_blinks_each_second() {
        assert!(colon_visible(0));
        assert!(!colon_visible(1));
        assert!(colon_visible(2));
        assert!(!colon_visible(59));
    }

    #[test]
    fn rendered_dims_scale_linearly() {
        assert_eq!(rendered_cols(1), 29 * PIXEL_COLS);
        assert_eq!(rendered_rows(1), GLYPH_H);
        assert_eq!(rendered_cols(3), rendered_cols(1) * 3);
        assert_eq!(rendered_rows(3), rendered_rows(1) * 3);
    }

    #[test]
    fn compute_scale_fits_terminal() {
        // 丁度 scale=1 に収まる
        assert_eq!(compute_scale(rendered_cols(1), rendered_rows(1)), 1);
        // scale=2 に必要なサイズ
        assert_eq!(compute_scale(rendered_cols(2), rendered_rows(2)), 2);
        // 幅が足りないときは幅で制限
        assert_eq!(compute_scale(rendered_cols(2) - 1, rendered_rows(10)), 1);
        // 高さが足りないときは高さで制限
        assert_eq!(compute_scale(rendered_cols(10), rendered_rows(2) - 1), 1);
    }

    #[test]
    fn compute_scale_minimum_one() {
        assert_eq!(compute_scale(1, 1), 1);
        assert_eq!(compute_scale(0, 0), 1);
    }

    #[test]
    fn render_height_matches_scale() {
        let lines = render("12:34", true, 2, &Style::DEFAULT);
        assert_eq!(lines.len(), rendered_rows_for_style(2, &Style::DEFAULT));
    }

    #[test]
    fn render_visible_width_matches_scale() {
        let lines = render("12:34", true, 1, &Style::DEFAULT);
        assert_eq!(
            visible_width(&lines[0]),
            rendered_cols_for_style(1, &Style::DEFAULT)
        );
    }

    #[test]
    fn render_visible_width_matches_cols_for_halfwidth() {
        // 半角文字(#, 空白)のとき表示幅 = rendered_cols
        let lines = render("12:34", true, 1, &Style::DEFAULT);
        assert_eq!(
            visible_width(&lines[0]),
            rendered_cols_for_style(1, &Style::DEFAULT)
        );
    }

    #[test]
    fn render_visible_width_matches_cols_for_block_char() {
        // ブロック要素(█)が全角扱いの端末でも、表示幅は rendered_cols に一致すべき
        let style = Style {
            font: Font::Block,
            chars: CharSet {
                on: '█', off: ' '
            },
        };
        let lines = render("12:34", true, 1, &style);
        // unicode-width では █ は幅1(半角)扱い。表示幅 = rendered_cols(1)
        // 端末フォント依存を除くため、ここでは unicode-width の判定に従う
        assert_eq!(
            visible_width(&lines[0]),
            rendered_cols(1),
            "block char visible width should match rendered_cols"
        );
    }

    #[test]
    fn render_fullwidth_on_char_produces_correct_width() {
        // 全角文字(●, 幅2)を on にした場合、1ピクセル = 1個 で PIXEL_COLS(2)列を満たす
        let style = Style {
            font: Font::Block,
            chars: CharSet {
                on: '●', off: ' '
            },
        };
        let lines = render("1", true, 1, &style);
        // "1" の行0は "  #  " -> 中央1ピクセル = ●1個(幅2) + 両端4ピクセル=空白4個(幅4) = 6列
        // グリフ1つ = GLYPH_W * PIXEL_COLS = 5*2 = 10列
        assert_eq!(visible_width(&lines[0]), GLYPH_W * PIXEL_COLS);
    }

    #[test]
    fn render_colon_on_has_pixels() {
        let lines = render("00:00", true, 1, &Style::DEFAULT);
        // コロン行(行1)の中央付近に # がある
        let colon_row = &lines[1];
        assert!(
            colon_row.contains(Style::DEFAULT.chars.on),
            "colon on should have pixels: {colon_row}"
        );
    }

    #[test]
    fn render_colon_off_has_no_pixels_in_colon() {
        // "00:00" のコロンは3つ目のグリフ(インデックス2)。
        // グリフスロット幅 = GLYPH_W*PIXEL_COLS + GAP*PIXEL_COLS = 5*2 + 1*2 = 12
        // コロン領域の開始列 = 2 * 12 = 24、幅 = GLYPH_W*PIXEL_COLS = 10
        let glyph_w = digits::glyph_width(Style::DEFAULT.font);
        let slot = (glyph_w + GAP) * PIXEL_COLS;
        let start = 2 * slot;
        let end = start + glyph_w * PIXEL_COLS;

        let off = render("00:00", false, 1, &Style::DEFAULT);
        let on = render("00:00", true, 1, &Style::DEFAULT);

        let colon_rows: Vec<usize> = digits::render_glyph(Style::DEFAULT.font, ':')
            .iter()
            .enumerate()
            .filter_map(|(row, pattern)| pattern.contains('#').then_some(row))
            .collect();
        assert!(
            !colon_rows.is_empty(),
            "default font colon should have visible rows"
        );

        for r in colon_rows {
            let colon_off: String = off[r].chars().skip(start).take(end - start).collect();
            let colon_on: String = on[r].chars().skip(start).take(end - start).collect();
            assert!(
                colon_on.contains(Style::DEFAULT.chars.on),
                "colon on row {r} should have pixels: {colon_on}"
            );
            assert!(
                !colon_off.contains(Style::DEFAULT.chars.on),
                "colon off row {r} should be blank: {colon_off}"
            );
        }

        // 数字領域は on/off で変わらない
        assert_eq!(off[0], on[0], "digit rows must not change with colon state");
    }

    #[test]
    fn render_scale_2_doubles_pixel_block() {
        let lines = render("1", true, 2, &Style::DEFAULT);
        // グリフ1の行0は "  #  " -> 中央ピクセル1つ = PIXEL_COLS*2=4 個の #
        // ギャップなし(グリフ1つだけ)なので行0の # は4つ連続
        let row0 = &lines[0];
        assert!(
            row0.contains(&Style::DEFAULT.chars.on.to_string().repeat(4)),
            "scale2 should produce 4-wide pixel: {row0}"
        );
    }

    // --- スタイル/カラーのテスト ---

    #[test]
    fn render_with_custom_on_char_uses_it() {
        let style = Style {
            font: Font::Block,
            chars: CharSet {
                on: '█', off: ' '
            },
        };
        let lines = render("8", true, 1, &style);
        // 行0は "#####" -> on_char 5つ
        assert!(lines[0].contains('█'), "on_char should appear: {lines:?}");
        assert!(
            !lines[0].contains('#'),
            "default # should not appear with custom on_char: {lines:?}"
        );
    }

    #[test]
    fn render_with_custom_off_char_uses_it() {
        let style = Style {
            font: Font::Block,
            chars: CharSet { on: '#', off: '·' },
        };
        let lines = render("0", true, 1, &style);
        // 行0は "#####" なので off_char は出ないが、行1は "#   #" なので中央に off_char
        let row1 = &lines[1];
        assert!(row1.contains('·'), "off_char should appear in gaps: {row1}");
    }

    #[test]
    fn render_outline_font_differs_from_block() {
        let block_style = Style {
            font: Font::Block,
            chars: CharSet::DEFAULT,
        };
        let block = render("0", true, 1, &block_style);
        let outline = render(
            "0",
            true,
            1,
            &Style {
                font: Font::Outline,
                chars: CharSet::DEFAULT,
            },
        );
        // Block の 2 の中段は "#####"(全幅)、Outline は " ### "(中抜き)
        let block2 = render("2", true, 1, &block_style);
        let outline2 = render(
            "2",
            true,
            1,
            &Style {
                font: Font::Outline,
                chars: CharSet::DEFAULT,
            },
        );
        assert!(block2[2].contains("##########"));
        // Outline の 2 中段は " ### " -> 3px * PIXEL_COLS(2) = 6 個の #
        assert_eq!(outline2[2], "  ######  ");
        // 0 は両フォントで同じ(輪郭だけなので)
        assert_eq!(block, outline);
    }

    #[test]
    fn charset_next_cycles() {
        let all = CharSet::all();
        let mut s = all[0];
        for _ in 0..all.len() {
            s = s.next();
        }
        assert_eq!(s, all[0]);
    }

    #[test]
    fn charset_next_visits_each() {
        let all = CharSet::all();
        let mut s = all[0];
        let mut visited = vec![s];
        for _ in 0..(all.len() - 1) {
            s = s.next();
            visited.push(s);
        }
        assert_eq!(visited.len(), all.len());
        for &a in all {
            assert!(visited.contains(&a), "charset {a:?} should be visited");
        }
    }

    #[test]
    fn color_next_cycles() {
        let all = Color::all();
        let mut c = all[0];
        for _ in 0..all.len() {
            c = c.next();
        }
        assert_eq!(c, all[0]);
    }

    #[test]
    fn theme_prefix_default_is_reset_codes() {
        // Default fg=39, Default bg=49
        let p = theme_prefix(Theme::default());
        assert_eq!(p, "\x1b[39;49m");
    }

    #[test]
    fn theme_prefix_red_on_blue() {
        let p = theme_prefix(Theme {
            fg: Color::Red,
            bg: Color::Blue,
        });
        assert_eq!(p, "\x1b[31;44m");
    }

    #[test]
    fn theme_prefix_fg_and_bg_codes_correct() {
        // いくつかの組み合わせを検証
        assert_eq!(
            theme_prefix(Theme {
                fg: Color::Green,
                bg: Color::Black,
            }),
            "\x1b[32;40m"
        );
        assert_eq!(
            theme_prefix(Theme {
                fg: Color::White,
                bg: Color::Cyan,
            }),
            "\x1b[37;46m"
        );
    }

    #[test]
    fn default_style_uses_neo_and_block_char() {
        assert_eq!(Style::DEFAULT.font, Font::Neo);
        assert_eq!(Style::DEFAULT.chars.on, '█');
        assert_eq!(Style::DEFAULT.chars.off, ' ');
    }

    // --- ヘルプダイアログのテスト ---

    #[test]
    fn help_lines_is_nonempty() {
        let lines = help_lines();
        assert!(!lines.is_empty());
        // タイトル行は空でない
        assert!(!lines[0].is_empty());
    }

    #[test]
    fn help_dialog_has_top_and_bottom_border() {
        let d = help_dialog(0);
        assert!(d.len() >= 3, "dialog should have top, body, bottom");
        assert!(d.first().unwrap().starts_with('┌'));
        assert!(d.first().unwrap().ends_with('┐'));
        assert!(d.last().unwrap().starts_with('└'));
        assert!(d.last().unwrap().ends_with('┘'));
    }

    #[test]
    fn help_dialog_all_rows_same_width() {
        let d = help_dialog(0);
        let w = d[0].chars().count();
        for (i, row) in d.iter().enumerate() {
            assert_eq!(row.chars().count(), w, "row {i} width mismatch: {row:?}");
        }
    }

    #[test]
    fn help_dialog_body_rows_have_side_borders() {
        let d = help_dialog(0);
        // 最初(上辺)と最後(下辺)以外は '|' で始まり '|' で終わる
        for row in &d[1..d.len() - 1] {
            assert!(row.starts_with('│'), "body row should start with │: {row}");
            assert!(row.ends_with('│'), "body row should end with │: {row}");
        }
    }

    #[test]
    fn help_dialog_width_grows_with_argument() {
        let small = help_dialog(0);
        let large = help_dialog(90);
        let small_w = small[0].chars().count();
        let large_w = large[0].chars().count();
        assert!(
            large_w > small_w,
            "larger width arg should produce wider dialog"
        );
        // すべての行が同じ幅
        for row in &large {
            assert_eq!(row.chars().count(), large_w);
        }
    }

    // --- 設定ページのテスト ---

    #[test]
    fn setting_items_count_is_total_options() {
        // Date(2) + Font(4) + CharSet(5) + Color(9) + Color(9) = 29
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        assert_eq!(items.len(), 2 + 4 + 5 + 9 + 9);
    }

    #[test]
    fn setting_items_kinds_in_order() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        // Date x2, Pattern x4, Symbol x5, Foreground x9, Background x9
        assert_eq!(items[0].kind, SettingKind::Date);
        assert_eq!(items[1].kind, SettingKind::Date);
        assert_eq!(items[2].kind, SettingKind::Pattern);
        assert_eq!(items[5].kind, SettingKind::Pattern);
        assert_eq!(items[6].kind, SettingKind::Symbol);
        assert_eq!(items[10].kind, SettingKind::Symbol);
        assert_eq!(items[11].kind, SettingKind::Foreground);
        assert_eq!(items[19].kind, SettingKind::Foreground);
        assert_eq!(items[20].kind, SettingKind::Background);
        assert_eq!(items[28].kind, SettingKind::Background);
    }

    #[test]
    fn setting_items_selected_matches_current() {
        let style = Style {
            font: Font::Outline,
            chars: CharSet {
                on: '█', off: ' '
            },
        };
        let theme = Theme {
            fg: Color::Red,
            bg: Color::Blue,
        };
        let items = setting_items(style, theme, DateDisplay::default());
        // 各種類で1つだけ selected=true
        let date_selected: Vec<_> = items
            .iter()
            .filter(|i| i.kind == SettingKind::Date && i.selected)
            .collect();
        assert_eq!(date_selected.len(), 1);
        let pattern_selected: Vec<_> = items
            .iter()
            .filter(|i| i.kind == SettingKind::Pattern && i.selected)
            .collect();
        assert_eq!(pattern_selected.len(), 1);
        assert_eq!(pattern_selected[0].label, "Outline");
        let symbol_selected: Vec<_> = items
            .iter()
            .filter(|i| i.kind == SettingKind::Symbol && i.selected)
            .collect();
        assert_eq!(symbol_selected.len(), 1);
        assert_eq!(symbol_selected[0].label, "█ / space");
        let fg_selected: Vec<_> = items
            .iter()
            .filter(|i| i.kind == SettingKind::Foreground && i.selected)
            .collect();
        assert_eq!(fg_selected.len(), 1);
        assert_eq!(fg_selected[0].label, "Red");
        let bg_selected: Vec<_> = items
            .iter()
            .filter(|i| i.kind == SettingKind::Background && i.selected)
            .collect();
        assert_eq!(bg_selected.len(), 1);
        assert_eq!(bg_selected[0].label, "Blue");
    }

    #[test]
    fn setting_items_apply_style_and_theme() {
        let style = Style::DEFAULT;
        let theme = Theme::default();
        let items = setting_items(style, theme, DateDisplay::default());
        // Pattern の Outline を適用すると font=Outline になる
        let outline = items
            .iter()
            .find(|i| i.kind == SettingKind::Pattern && i.label == "Outline")
            .unwrap();
        assert_eq!(outline.style.font, Font::Outline);
        assert_eq!(outline.style.chars, style.chars);
        // Foreground の Red を適用すると fg=Red になる
        let red = items
            .iter()
            .find(|i| i.kind == SettingKind::Foreground && i.label == "Red")
            .unwrap();
        assert_eq!(red.theme.fg, Color::Red);
        assert_eq!(red.theme.bg, theme.bg);
    }

    #[test]
    fn font_preview_differs_between_block_and_outline() {
        let block = font_preview(Font::Block);
        let outline = font_preview(Font::Outline);
        // Block の "2" 中段は全幅(10文字)、Outline は中抜き
        assert_eq!(block.chars().count(), 10);
        assert!(block.starts_with("█████"));
        // Outline は両端に空白
        assert!(outline.starts_with("  "));
        assert_ne!(block, outline);
    }

    #[test]
    fn charset_preview_uses_on_and_off_chars() {
        let cs = CharSet {
            on: '▓', off: '░'
        };
        let p = charset_preview(cs);
        assert_eq!(p, "▓▓▓▓▓░░");
    }

    #[test]
    fn color_fg_preview_contains_ansi_escape() {
        let p = color_fg_preview(Color::Red);
        assert!(p.contains("\x1b[31m"), "red fg preview: {p}");
        assert!(p.contains("Sample"));
        assert!(p.contains("\x1b[0m"));
    }

    #[test]
    fn color_bg_preview_contains_ansi_escape() {
        let p = color_bg_preview(Color::Blue);
        assert!(p.contains("\x1b[44m"), "blue bg preview: {p}");
        assert!(p.contains("\x1b[0m"));
    }

    // --- settings_layout / settings_modal のテスト ---

    #[test]
    fn settings_layout_has_box_top_and_bottom_border() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 30);
        assert!(layout.height >= 3);
        assert!(layout.rows.first().unwrap().text.starts_with('┌'));
        assert!(layout.rows.first().unwrap().text.ends_with('┐'));
        assert!(layout.rows.last().unwrap().text.starts_with('└'));
        assert!(layout.rows.last().unwrap().text.ends_with('┘'));
    }

    #[test]
    fn settings_layout_title_in_top_border() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 30);
        assert!(
            layout.rows.first().unwrap().text.contains("Settings"),
            "title should be in top border: {}",
            layout.rows.first().unwrap().text
        );
    }

    #[test]
    fn settings_layout_footer_present() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 30);
        let joined: String = layout
            .rows
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            joined.contains("Click"),
            "footer should mention Click: {joined}"
        );
        assert!(
            joined.contains("change"),
            "footer should mention change: {joined}"
        );
    }

    #[test]
    fn settings_layout_all_rows_same_visible_width() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 30);
        let w = visible_width(&layout.rows[0].text);
        assert_eq!(w, layout.width, "layout.width should match first row width");
        for (i, r) in layout.rows.iter().enumerate() {
            assert_eq!(
                visible_width(&r.text),
                w,
                "row {i} visible width mismatch: {:?}",
                r.text
            );
        }
    }

    #[test]
    fn settings_layout_body_rows_have_side_borders() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 30);
        // 最初(上辺)と最後(下辺)以外は '|' か '├' で始まり '|' か '┤' で終わる。
        // カーソル行は先頭/末尾にANSIエスケープを含むので strip_ansi してから判定。
        for r in &layout.rows[1..layout.rows.len() - 1] {
            let stripped = strip_ansi(&r.text);
            let starts_ok = stripped.starts_with('│') || stripped.starts_with('├');
            let ends_ok = stripped.ends_with('│') || stripped.ends_with('┤');
            assert!(starts_ok, "body row should start with │ or ├: {}", r.text);
            assert!(ends_ok, "body row should end with │ or ┤: {}", r.text);
        }
    }

    #[test]
    fn settings_layout_has_section_dividers() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 100);
        let dividers = layout
            .rows
            .iter()
            .filter(|r| r.kind == SettingsRowKind::Divider)
            .count();
        // タブ下 + フッタ区切り
        assert_eq!(dividers, 2, "should have 2 dividers");
    }

    #[test]
    fn settings_layout_headers_appear_for_each_kind() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 100);
        let joined: String = layout
            .rows
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("Pattern"));
        assert!(joined.contains("Symbol"));
        assert!(joined.contains("Foreground"));
        assert!(joined.contains("Background"));
    }

    #[test]
    fn settings_layout_item_rows_use_radio_dots() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 100);
        let item_rows: Vec<_> = layout
            .rows
            .iter()
            .filter(|r| matches!(r.kind, SettingsRowKind::Item { .. }))
            .collect();
        assert!(!item_rows.is_empty());
        // 選択中の項目は ●、未選択は ○
        let selected = item_rows.iter().find(|r| r.text.contains('●'));
        assert!(selected.is_some(), "selected item should use ●");
        let unselected = item_rows.iter().find(|r| r.text.contains('○'));
        assert!(unselected.is_some(), "unselected item should use ○");
    }

    #[test]
    fn settings_layout_cursor_row_has_bold_marker_without_underline() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 1, 100);
        // cursor=1 の行は太字ハイライトのみ。背景反転・下線は使わない。
        let cursor_row = layout
            .rows
            .iter()
            .find(|r| matches!(r.kind, SettingsRowKind::Item { index } if index == 1))
            .expect("cursor item row should exist");
        assert!(
            cursor_row.text.contains("\x1b[1m"),
            "cursor row should have bold: {}",
            cursor_row.text
        );
        assert!(
            !cursor_row.text.contains("\x1b[4m") && !cursor_row.text.contains("\x1b[1;4m"),
            "cursor row should NOT have underline: {}",
            cursor_row.text
        );
        assert!(
            !cursor_row.text.contains("\x1b[7m"),
            "cursor row should NOT have reverse video: {}",
            cursor_row.text
        );
        assert!(
            cursor_row.text.contains("▶"),
            "cursor row should have ▶ marker: {}",
            cursor_row.text
        );
    }

    #[test]
    fn settings_layout_non_cursor_row_has_no_highlight() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 1, 100);
        // cursor=1 以外のアイテム行はハイライト属性を含まない
        for r in &layout.rows {
            if let SettingsRowKind::Item { index } = r.kind {
                if index != 1 {
                    assert!(
                        !r.text.contains("\x1b[1m"),
                        "non-cursor row {index} should not have bold: {}",
                        r.text
                    );
                    assert!(
                        !r.text.contains("\x1b[4m") && !r.text.contains("\x1b[1;4m"),
                        "non-cursor row {index} should not have underline: {}",
                        r.text
                    );
                    assert!(
                        !r.text.contains("\x1b[7m"),
                        "non-cursor row {index} should not have reverse: {}",
                        r.text
                    );
                }
            }
        }
    }

    #[test]
    fn settings_layout_clamps_cursor() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 999, 100);
        // クランプされて最後の項目(Background White)がカーソル行になる
        let cursor_row = layout
            .rows
            .iter()
            .find(|r| r.text.contains('▶'))
            .expect("cursor marker should appear");
        assert!(
            cursor_row.text.contains("White"),
            "clamped cursor should be White: {}",
            cursor_row.text
        );
    }

    #[test]
    fn settings_layout_scrolls_when_viewport_small() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        // viewport=8 で cursor=10 ならスクロールし、かつカーソル行が表示される
        let layout = settings_layout(&items, 10, 8);
        // カーソル行(▶)は含まれるべき
        let has_cursor = layout.rows.iter().any(|r| r.text.contains('▶'));
        assert!(has_cursor, "cursor should be visible after scroll");
        // 先頭の本文行に Pattern ヘッダは出ない(スクロール済み)
        let body_first = layout
            .rows
            .iter()
            .find(|r| r.kind == SettingsRowKind::Header)
            .expect("at least one header should be present");
        assert!(
            !body_first.text.contains("Pattern"),
            "Pattern header should be scrolled out: {}",
            body_first.text
        );
    }

    #[test]
    fn settings_layout_item_kinds_track_indices() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 100);
        let item_indices: Vec<usize> = layout
            .rows
            .iter()
            .filter_map(|r| match r.kind {
                SettingsRowKind::Item { index } => Some(index),
                _ => None,
            })
            .collect();
        // タブ式なので、現在カテゴリ(Date)のアイテムだけが含まれる
        assert_eq!(item_indices, (0..2).collect::<Vec<_>>());
    }

    #[test]
    fn settings_modal_matches_layout_texts() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 2, 30);
        let modal = settings_modal(&items, 2, 30);
        assert_eq!(modal.len(), layout.rows.len());
        for (m, r) in modal.iter().zip(layout.rows.iter()) {
            assert_eq!(m, &r.text);
        }
    }

    // --- layout_item_at (マウスヒットテスト) のテスト ---

    #[test]
    fn layout_item_at_hits_cursor_item_row() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 100);
        // 最初の Item 行のインデックスを探す
        let (row_offset, expected_index) = layout
            .rows
            .iter()
            .enumerate()
            .find(|(_, r)| matches!(r.kind, SettingsRowKind::Item { .. }))
            .map(|(i, r)| {
                (
                    i,
                    match r.kind {
                        SettingsRowKind::Item { index } => index,
                        _ => unreachable!(),
                    },
                )
            })
            .unwrap();
        let top = 5;
        let left = 10;
        // クリック座標 = modal 内のそのアイテム行
        let hit = layout_item_at(&layout, top, left, top + row_offset, left + 3);
        assert_eq!(hit, Some(expected_index));
    }

    #[test]
    fn layout_item_at_returns_none_for_header_row() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 100);
        let (row_offset, _) = layout
            .rows
            .iter()
            .enumerate()
            .find(|(_, r)| r.kind == SettingsRowKind::Header)
            .map(|(i, r)| (i, r.clone()))
            .unwrap();
        let hit = layout_item_at(&layout, 0, 0, row_offset, 3);
        assert_eq!(hit, None, "header row click should not select");
    }

    #[test]
    fn layout_item_at_returns_none_for_border_row() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 100);
        // 上辺(行0)のクリックは None
        assert_eq!(layout_item_at(&layout, 0, 0, 0, 3), None);
        // 下辺(最終行)のクリックは None
        let last = layout.height - 1;
        assert_eq!(layout_item_at(&layout, 0, 0, last, 3), None);
    }

    #[test]
    fn layout_item_at_returns_none_outside_horizontal_bounds() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 100);
        let (row_offset, _) = layout
            .rows
            .iter()
            .enumerate()
            .find(|(_, r)| matches!(r.kind, SettingsRowKind::Item { .. }))
            .map(|(i, r)| (i, r.clone()))
            .unwrap();
        // left より左のクリック
        assert_eq!(layout_item_at(&layout, 5, 10, 5 + row_offset, 9), None);
        // right (= left + width) 以降のクリック
        assert_eq!(
            layout_item_at(&layout, 5, 10, 5 + row_offset, 10 + layout.width),
            None
        );
    }

    #[test]
    fn layout_item_at_returns_none_outside_vertical_bounds() {
        let items = setting_items(Style::DEFAULT, Theme::default(), DateDisplay::default());
        let layout = settings_layout(&items, 0, 100);
        // modal 上辺より上の行
        assert_eq!(layout_item_at(&layout, 5, 0, 4, 3), None);
        // modal 下辺より下の行
        assert_eq!(layout_item_at(&layout, 5, 0, 5 + layout.height, 3), None);
    }

    #[test]
    fn visible_width_counts_only_non_escape_chars() {
        assert_eq!(visible_width("hello"), 5);
        assert_eq!(visible_width("\x1b[31mRed\x1b[0m"), 3);
        assert_eq!(visible_width("\x1b[44m     \x1b[0m"), 5);
        assert_eq!(visible_width("no escapes here"), 15);
    }
}
