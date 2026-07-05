//! timenow TUI: フルスクリーンで巨大なデジタル時計を表示。
//! `q` または `Enter` で終了。端末リサイズに追従して最大サイズで描画。
//!
//! 実行時キー操作でカスタマイズ:
//! - `p` : パターン(フォント)切替
//! - `s` : シンボル(ON/OFF文字)切替
//! - `c` : 前景色切替
//! - `b` : 背景色切替
//! - `h` : ヘルプダイアログ表示/非表示
//! - `o` : 設定ページ(modal)表示/非表示(←→でカテゴリ、↑↓で即時変更)
//! - `q` / `Q` / `Enter` : 終了

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::{Datelike, Local, Timelike};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use unicode_width::UnicodeWidthChar;

use timenow::{clock, config};

const TICK: Duration = Duration::from_millis(200);

/// 生のターミナル状態を管理し、Drop時に確実に復帰するRAIIガード。
/// エラーやパニック時でも raw mode / alternate screen を解除する。
struct TerminalGuard {
    stdout: io::Stdout,
}

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        let stdout = io::stdout();
        // raw mode を有効化してから alternate screen に入る。
        // raw mode 有効化後にガードを生成し、以降のステップが失敗しても Drop で確実に復帰する。
        terminal::enable_raw_mode()?;
        let mut guard = Self { stdout };
        match execute!(
            &mut guard.stdout,
            EnterAlternateScreen,
            Hide,
            EnableMouseCapture
        ) {
            Ok(()) => {
                guard.stdout.flush()?;
                Ok(guard)
            }
            Err(e) => {
                // ここで return すると guard が drop して raw mode が解除される。
                drop(guard);
                Err(e)
            }
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // 復帰処理はベストエフォート。個別のエラーは無視して次へ進む。
        let _ = execute!(
            &mut self.stdout,
            Show,
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = terminal::disable_raw_mode();
        let _ = self.stdout.flush();
    }
}

/// 描画状態(スタイル + カラー)。キー操作で更新される。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DrawState {
    style: clock::Style,
    theme: clock::Theme,
    date_display: clock::DateDisplay,
}

impl Default for DrawState {
    fn default() -> Self {
        Self::from(config::AppConfig::default())
    }
}

impl From<config::AppConfig> for DrawState {
    fn from(config: config::AppConfig) -> Self {
        Self {
            style: config.style,
            theme: config.theme,
            date_display: config.date_display,
        }
    }
}

impl From<DrawState> for config::AppConfig {
    fn from(state: DrawState) -> Self {
        Self {
            style: state.style,
            theme: state.theme,
            date_display: state.date_display,
        }
    }
}

/// 設定ページの状態。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SettingsState {
    open: bool,
    cursor: usize,
}

/// 再描画が必要か判定するためのキー。いずれかの要素が変化したときだけ描画する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DrawKey {
    hour: u32,
    minute: u32,
    month: u32,
    day: u32,
    colon_on: bool,
    cols: u16,
    rows: u16,
    style: clock::Style,
    theme: clock::Theme,
    date_display: clock::DateDisplay,
    show_help: bool,
    settings: SettingsState,
}

fn main() -> io::Result<()> {
    // --version / -v: ターミナルモードに入る前に標準出力へ表示して終了。
    if env::args().any(|a| a == "--version" || a == "-v") {
        println!("timenow {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let mut guard = TerminalGuard::enter()?;
    run(&mut guard.stdout)
}

fn run(stdout: &mut io::Stdout) -> io::Result<()> {
    let mut state = load_state();
    let mut show_help = false;
    let mut settings = SettingsState::default();
    let mut last_draw: Option<DrawKey> = None;
    loop {
        // 現在時刻
        let now = Local::now();
        let hour = now.hour();
        let minute = now.minute();
        let month = now.month();
        let day = now.day();
        let second = now.second();
        let colon_on = clock::colon_visible(second);

        let (cols, rows) = terminal::size()?;
        let key = DrawKey {
            hour,
            minute,
            month,
            day,
            colon_on,
            cols,
            rows,
            style: state.style,
            theme: state.theme,
            date_display: state.date_display,
            show_help,
            settings,
        };

        // 変化があったときだけ描画
        if last_draw != Some(key) {
            // 構造的変更(サイズ/スタイル/色/レイアウト)のときだけ全クリアが必要。
            // コロン点滅・时分更新だけなら同じセルを上書きすれば済み、クリアしないことでチラつきを防ぐ。
            let needs_clear = last_draw.is_none_or(|prev| structural_changed(&prev, &key));
            draw(stdout, &key, needs_clear)?;
            last_draw = Some(key);
        }

        // 入力を TICK だけ待つ
        let deadline = Instant::now() + TICK;
        while Instant::now() < deadline {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or(Duration::ZERO);
            if remaining.is_zero() {
                break;
            }
            if !event::poll(remaining)? {
                break;
            }
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    if settings.open {
                        // 設定ページ表示中: ↑↓は現在カテゴリ内で即時変更、←→はカテゴリ移動。
                        let items =
                            clock::setting_items(state.style, state.theme, state.date_display);
                        match k.code {
                            KeyCode::Up => {
                                settings.cursor = move_in_section(&items, settings.cursor, -1);
                                apply_setting(&items, settings.cursor, &mut state);
                                persist_state(&state);
                            }
                            KeyCode::Down => {
                                settings.cursor = move_in_section(&items, settings.cursor, 1);
                                apply_setting(&items, settings.cursor, &mut state);
                                persist_state(&state);
                            }
                            KeyCode::Left => {
                                settings.cursor =
                                    section_cursor(&items, settings.cursor, SectionStep::Previous);
                            }
                            KeyCode::Right => {
                                settings.cursor =
                                    section_cursor(&items, settings.cursor, SectionStep::Next);
                            }
                            KeyCode::Home => {
                                settings.cursor =
                                    edge_in_section(&items, settings.cursor, SectionEdge::First);
                                apply_setting(&items, settings.cursor, &mut state);
                                persist_state(&state);
                            }
                            KeyCode::End => {
                                settings.cursor =
                                    edge_in_section(&items, settings.cursor, SectionEdge::Last);
                                apply_setting(&items, settings.cursor, &mut state);
                                persist_state(&state);
                            }
                            KeyCode::PageUp => {
                                settings.cursor = move_in_section(&items, settings.cursor, -4);
                                apply_setting(&items, settings.cursor, &mut state);
                                persist_state(&state);
                            }
                            KeyCode::PageDown => {
                                settings.cursor = move_in_section(&items, settings.cursor, 4);
                                apply_setting(&items, settings.cursor, &mut state);
                                persist_state(&state);
                            }
                            KeyCode::Char(' ') => {
                                apply_setting(&items, settings.cursor, &mut state);
                                persist_state(&state);
                            }
                            KeyCode::Char('o')
                            | KeyCode::Char('O')
                            | KeyCode::Esc
                            | KeyCode::Enter
                            | KeyCode::Char('q')
                            | KeyCode::Char('Q') => {
                                settings.open = false;
                            }
                            _ => {}
                        }
                    } else {
                        match k.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Enter => {
                                return Ok(());
                            }
                            KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Esc => {
                                show_help = !show_help;
                            }
                            KeyCode::Char('o') | KeyCode::Char('O') => {
                                settings.open = true;
                                let items = clock::setting_items(
                                    state.style,
                                    state.theme,
                                    state.date_display,
                                );
                                settings.cursor = clock::selected_index_for_kind(
                                    &items,
                                    clock::SettingKind::Date,
                                )
                                .unwrap_or(0);
                            }
                            KeyCode::Char('p') => {
                                state.style.font = state.style.font.next();
                                persist_state(&state);
                            }
                            KeyCode::Char('s') => {
                                state.style.chars = state.style.chars.next();
                                persist_state(&state);
                            }
                            KeyCode::Char('c') => {
                                state.theme.fg = state.theme.fg.next();
                                persist_state(&state);
                            }
                            KeyCode::Char('b') => {
                                state.theme.bg = state.theme.bg.next();
                                persist_state(&state);
                            }
                            KeyCode::Char('d') | KeyCode::Char('D') => {
                                state.date_display = state.date_display.next();
                                persist_state(&state);
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(m) if settings.open => {
                    // 設定 modal 表示中の左クリックで選択肢を直接選択
                    let items = clock::setting_items(state.style, state.theme, state.date_display);
                    match m.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            let (cols, rows) = terminal::size()?;
                            let vp_h = (rows as usize).saturating_sub(2).max(8);
                            let layout = clock::settings_layout(&items, settings.cursor, vp_h);
                            // 描画側と同じ下部寄せの m_top を使う。
                            let m_top = (rows as usize).saturating_sub(layout.height + 1);
                            let m_left = ((cols as usize).saturating_sub(layout.width)) / 2;
                            if let Some(idx) = clock::layout_item_at(
                                &layout,
                                m_top,
                                m_left,
                                m.row as usize,
                                m.column as usize,
                            ) {
                                settings.cursor = idx;
                                apply_setting(&items, settings.cursor, &mut state);
                                persist_state(&state);
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            settings.cursor = move_in_section(&items, settings.cursor, -1);
                            apply_setting(&items, settings.cursor, &mut state);
                            persist_state(&state);
                        }
                        MouseEventKind::ScrollDown => {
                            settings.cursor = move_in_section(&items, settings.cursor, 1);
                            apply_setting(&items, settings.cursor, &mut state);
                            persist_state(&state);
                        }
                        _ => {}
                    }
                }
                Event::Resize(_, _) => { /* 次の外側ループで size() を再読込して再描画 */
                }
                _ => {}
            }
        }
    }
}

/// 構造的変更(再クリアが必要)かどうかを判定する。
/// コロン点滅・時・分の変化だけなら同じセルを上書きすれば済むためクリア不要。
/// それ以外(サイズ/スタイル/色/日付表示/ヘルプ/設定/月日)の変化は
/// レイアウトが変わる可能性があるため全クリアが必要。
fn structural_changed(prev: &DrawKey, curr: &DrawKey) -> bool {
    prev.cols != curr.cols
        || prev.rows != curr.rows
        || prev.style != curr.style
        || prev.theme != curr.theme
        || prev.date_display != curr.date_display
        || prev.show_help != curr.show_help
        || prev.settings != curr.settings
        || prev.month != curr.month
        || prev.day != curr.day
}

fn draw(stdout: &mut io::Stdout, key: &DrawKey, needs_clear: bool) -> io::Result<()> {
    let text = clock::format_hhmm(key.hour, key.minute);
    let date_text = clock::format_date(key.month, key.day, key.date_display);
    let chrome_h = if key.rows >= 14 { 6 } else { 0 };
    let chrome_w = if key.cols >= 72 { 6 } else { 0 };
    let available_cols = (key.cols as usize).saturating_sub(chrome_w).max(1);
    let date_lines = date_text
        .as_ref()
        .map(|d| compact_date_lines(d))
        .unwrap_or_default();
    let date_h = if !date_lines.is_empty() && key.rows >= 12 {
        date_lines.len() + 1
    } else {
        0
    };
    let available_rows = (key.rows as usize).saturating_sub(chrome_h + date_h).max(1);
    let scale = clock::resolve_scale(available_cols, available_rows, &key.style);
    let lines = clock::render(&text, key.colon_on, scale, &key.style);

    let rendered_h = lines.len();
    let rendered_w = lines
        .first()
        .map(|l| clock::visible_width_pub(l))
        .unwrap_or(0);

    let top_base = if chrome_h > 0 { 3 } else { 0 } + date_h;
    // 設定ページ表示中は時計を上寄せして modal と重ならないようにする。
    let top = if key.settings.open {
        top_base + (available_rows.saturating_sub(rendered_h)) / 4
    } else {
        top_base + (available_rows.saturating_sub(rendered_h)) / 2
    };
    let left = ((key.cols as usize).saturating_sub(rendered_w)) / 2;

    // 背景色で画面全体を塗りつぶすため、テーマ設定後にクリア。
    // 多くの端末で ANSIクリアは現在の背景色で埋める。
    // ただし毎フレームクリアするとコロン点滇のたびに空白フレームが見えてチラつくため、
    // 構造的変更(サイズ/スタイル/色/レイアウト)のときだけクリアする。
    // さらに synchronized output(\x1b[?2028h / \x1b[?2029l)で端末にアトミック更新を指示し、
    // 部分描画のチラつきを防ぐ。
    let prefix = clock::theme_prefix(key.theme);
    let mut buf = String::new();
    buf.push_str("\x1b[?2028h"); // Begin synchronized update
    buf.push_str(&prefix);
    if needs_clear {
        buf.push_str("\x1b[2J");
    }
    draw_chrome(&mut buf, key);

    if !date_lines.is_empty() && key.rows >= 12 {
        let date_w = date_lines
            .first()
            .map(|line| clock::visible_width_pub(line))
            .unwrap_or(0);
        let date_left = ((key.cols as usize).saturating_sub(date_w)) / 2;
        let date_top = top.saturating_sub(date_lines.len() + 1).max(1);
        buf.push_str("\x1b[22m\x1b[1m");
        for (i, line) in date_lines.iter().enumerate() {
            let y = date_top + i;
            buf.push_str(&format!("\x1b[{};{}H{}", y, date_left + 1, line));
        }
        buf.push_str("\x1b[22m");
        buf.push_str(&clock::theme_prefix(key.theme));
    }

    if key.rows >= 12 && key.cols >= 64 {
        let shadow_style = clock::Style {
            chars: clock::CharSet::Fill {
                on: '░', off: ' '
            },
            ..key.style
        };
        let shadow = clock::render(&text, key.colon_on, scale, &shadow_style);
        let shadow_top = (top + 1).min(key.rows as usize);
        let shadow_left = (left + 2).min(key.cols as usize);
        buf.push_str("\x1b[2m\x1b[90m");
        for (i, line) in shadow.iter().enumerate() {
            let y = shadow_top + i;
            if y < key.rows as usize {
                buf.push_str(&format!("\x1b[{};{}H{}", y + 1, shadow_left + 1, line));
            }
        }
        buf.push_str("\x1b[22m");
        buf.push_str(&clock::theme_prefix(key.theme));
    }

    buf.push_str("\x1b[22m\x1b[1m");
    for (i, line) in lines.iter().enumerate() {
        let y = top + i;
        // 行を組み立て: カーソル移動 + 内容
        buf.push_str(&format!("\x1b[{};{}H{}", y + 1, left + 1, line));
    }
    buf.push_str("\x1b[22m");

    // ヘルプダイアログをオーバーレイ(画面中央)
    if key.show_help {
        let dialog = clock::help_dialog(0);
        let d_h = dialog.len();
        let d_w = dialog.first().map(|l| l.chars().count()).unwrap_or(0);
        let d_top = ((key.rows as usize).saturating_sub(d_h)) / 2;
        let d_left = ((key.cols as usize).saturating_sub(d_w)) / 2;
        // ダイアログはデフォルト色(端末の前景/背景)で描画して視認性を確保
        buf.push_str("\x1b[39;49m");
        for (i, line) in dialog.iter().enumerate() {
            let y = d_top + i;
            buf.push_str(&format!("\x1b[{};{}H{}", y + 1, d_left + 1, line));
        }
    }

    // 設定ページを modal として下部にオーバーレイ(枠線付き)。
    // 時計を見ながら設定できるよう、画面中央ではなく下部に寄せる。
    if key.settings.open {
        let items = clock::setting_items(key.style, key.theme, key.date_display);
        let vp_h = (key.rows as usize).saturating_sub(2).max(8);
        let modal = clock::settings_modal(&items, key.settings.cursor, vp_h);
        let m_h = modal.len();
        let m_w = modal
            .iter()
            .map(|l| clock::visible_width_pub(l))
            .max()
            .unwrap_or(0);
        // 下部寄せ: 画面下端から1行余白を空けて配置。
        let m_top = (key.rows as usize).saturating_sub(m_h + 1);
        let m_left = ((key.cols as usize).saturating_sub(m_w)) / 2;
        // modal はデフォルト色で描画して視認性を確保
        buf.push_str("\x1b[39;49m");
        for (i, line) in modal.iter().enumerate() {
            let y = m_top + i;
            buf.push_str(&format!("\x1b[{};{}H{}", y + 1, m_left + 1, line));
        }
    }

    // リセット(属性を元に戻す) + synchronized update 終了
    buf.push_str("\x1b[0m");
    buf.push_str("\x1b[?2029l"); // End synchronized update
    stdout.write_all(buf.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum SectionStep {
    Previous,
    Next,
}

#[derive(Clone, Copy, Debug)]
enum SectionEdge {
    First,
    Last,
}

fn section_cursor(items: &[clock::SettingItem], cursor: usize, step: SectionStep) -> usize {
    let current = clock::active_setting_kind(items, cursor);
    let kinds = clock::SettingKind::all();
    let current_kind_idx = kinds.iter().position(|&kind| kind == current).unwrap_or(0);
    let target_kind_idx = match step {
        SectionStep::Previous => current_kind_idx
            .checked_sub(1)
            .unwrap_or_else(|| kinds.len().saturating_sub(1)),
        SectionStep::Next => (current_kind_idx + 1) % kinds.len(),
    };
    let target = kinds[target_kind_idx];
    clock::selected_index_for_kind(items, target)
        .or_else(|| clock::first_index_for_kind(items, target))
        .unwrap_or(cursor)
}

fn move_in_section(items: &[clock::SettingItem], cursor: usize, delta: isize) -> usize {
    let current = clock::active_setting_kind(items, cursor);
    let section = section_indices(items, current);
    if section.is_empty() {
        return cursor;
    }
    let current_pos = section.iter().position(|&idx| idx == cursor).unwrap_or(0);
    let next_pos =
        (current_pos as isize + delta).clamp(0, section.len().saturating_sub(1) as isize) as usize;
    section[next_pos]
}

fn edge_in_section(items: &[clock::SettingItem], cursor: usize, edge: SectionEdge) -> usize {
    let current = clock::active_setting_kind(items, cursor);
    let section = section_indices(items, current);
    match edge {
        SectionEdge::First => section.first().copied().unwrap_or(cursor),
        SectionEdge::Last => section.last().copied().unwrap_or(cursor),
    }
}

fn section_indices(items: &[clock::SettingItem], kind: clock::SettingKind) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| (item.kind == kind).then_some(idx))
        .collect()
}

fn apply_setting(items: &[clock::SettingItem], cursor: usize, state: &mut DrawState) {
    if let Some(item) = items.get(cursor) {
        state.style = item.style;
        state.theme = item.theme;
        state.date_display = item.date_display;
    }
}

fn compact_date_lines(text: &str) -> Vec<String> {
    let mut lines = vec![String::new(), String::new(), String::new(), String::new()];
    for (char_idx, ch) in text.chars().enumerate() {
        let glyph = compact_date_glyph(ch);
        for (row, pattern) in glyph.iter().enumerate() {
            if char_idx > 0 {
                lines[row].push_str("  ");
            }
            lines[row].push_str(pattern);
        }
    }
    lines
}

fn compact_date_glyph(ch: char) -> &'static [&'static str; 4] {
    match ch {
        '0' => &["┏━┓", "┃┃┃", "┃┃┃", "┗━┛"],
        '1' => &["┏┓", "┃┃", "┃┃", "┗┛"],
        '2' => &["┏━┓", "┣┫┃", "┃┏┫", "┗━┛"],
        '3' => &["┏━┓", "┣┛┃", "┣┓┃", "┗━┛"],
        '4' => &["┏┳┓", "┃┃┃", "┗┓┃", " ┗┛"],
        '5' => &["┏━┓", "┃━┫", "┣┓┃", "┗━┛"],
        '6' => &["┏━┓", "┃┗┫", "┃┃┃", "┗━┛"],
        '7' => &["┏━┓", "┃┃┃", "┗┫┃", " ┗┛"],
        '8' => &["┏━┓", "┃━┃", "┃━┃", "┗━┛"],
        '9' => &["┏━┓", "┃┃┃", "┣┓┃", "┗━┛"],
        '/' => &["   ", "  ╱", " ╱ ", "   "],
        ' ' => &["   ", "   ", "   ", "   "],
        _ => &["   ", "   ", "   ", "   "],
    }
}

fn draw_chrome(buf: &mut String, key: &DrawKey) {
    if key.cols < 48 || key.rows < 10 {
        return;
    }

    let cols = key.cols as usize;
    let date = format!("{}/{}", key.month, key.day);
    let left = " TimeNow";
    let right = format!("{:02}:{:02}  {} ", key.hour, key.minute, date);
    write_line(buf, cols, 1, 2, "\x1b[2m", left);
    let right_w = clock::visible_width_pub(&right);
    let right_col = cols.saturating_sub(right_w + 2);
    write_line(buf, cols, 1, right_col, "\x1b[2m", &right);

    if cols >= 72 {
        let rail = "─".repeat(cols.saturating_sub(8));
        write_line(buf, cols, 2, 4, "\x1b[2m", &rail);
    }

    let footer = " p pattern  s symbol  c fg  b bg  d date  +/- size  o settings  h help  q quit ";
    let footer_w = clock::visible_width_pub(footer);
    let footer_col = (cols.saturating_sub(footer_w)) / 2;
    write_line(buf, cols, key.rows as usize, footer_col, "\x1b[2m", footer);
}

fn write_line(buf: &mut String, cols: usize, row: usize, col: usize, attr: &str, text: &str) {
    let row = row.max(1);
    let col = col.max(1);
    let max_width = cols.saturating_sub(col.saturating_sub(1));
    let text = clip_to_width(text, max_width);
    buf.push_str(&format!("\x1b[{};{}H{}{}\x1b[22m", row, col, attr, text));
}

fn clip_to_width(text: &str, max_width: usize) -> String {
    let mut out = String::new();
    let mut width = 0;
    for ch in text.chars() {
        let ch_w = ch.width().unwrap_or(1);
        if width + ch_w > max_width {
            break;
        }
        out.push(ch);
        width += ch_w;
    }
    out
}

fn load_state() -> DrawState {
    let default = config::AppConfig::default();
    let Some(path) = config_path() else {
        return DrawState::from(default);
    };
    let Ok(text) = fs::read_to_string(path) else {
        return DrawState::from(default);
    };

    DrawState::from(config::parse_config(&text, default))
}

fn persist_state(state: &DrawState) {
    let Some(path) = config_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, config::format_config(config::AppConfig::from(*state)));
}

fn config_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/timenow/setting.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_for(hour: u32, minute: u32, month: u32, day: u32, cols: u16, rows: u16) -> DrawKey {
        DrawKey {
            hour,
            minute,
            month,
            day,
            colon_on: true,
            cols,
            rows,
            style: clock::Style::DEFAULT,
            theme: clock::Theme::default(),
            date_display: clock::DateDisplay::Numeric,
            show_help: false,
            settings: SettingsState::default(),
        }
    }

    #[test]
    fn draw_chrome_uses_key_time_and_date() {
        let key = key_for(9, 5, 3, 7, 80, 24);
        let mut buf = String::new();
        draw_chrome(&mut buf, &key);
        assert!(buf.contains("09:05"), "chrome must show key hour:minute");
        assert!(buf.contains("3/7"), "chrome must show key month/day");
    }

    #[test]
    fn draw_chrome_skips_when_too_small() {
        let key = key_for(9, 5, 3, 7, 40, 8);
        let mut buf = String::new();
        draw_chrome(&mut buf, &key);
        assert!(buf.is_empty(), "chrome must not draw on tiny terminals");
    }

    #[test]
    fn structural_changed_false_for_colon_blink_only() {
        let prev = key_for(12, 30, 7, 5, 80, 24);
        let mut curr = prev;
        curr.colon_on = !prev.colon_on;
        assert!(!structural_changed(&prev, &curr));
    }

    #[test]
    fn structural_changed_false_for_minute_update_only() {
        let prev = key_for(12, 30, 7, 5, 80, 24);
        let mut curr = prev;
        curr.minute = 31;
        assert!(!structural_changed(&prev, &curr));
    }

    #[test]
    fn structural_changed_true_for_resize() {
        let prev = key_for(12, 30, 7, 5, 80, 24);
        let mut curr = prev;
        curr.cols = 100;
        assert!(structural_changed(&prev, &curr));
    }

    #[test]
    fn structural_changed_true_for_style_or_theme() {
        let prev = key_for(12, 30, 7, 5, 80, 24);
        let mut curr = prev;
        curr.style.font = clock::Style::DEFAULT.font.next();
        assert!(structural_changed(&prev, &curr));
        let mut curr2 = prev;
        curr2.theme.fg = clock::Color::Red;
        assert!(structural_changed(&prev, &curr2));
    }

    #[test]
    fn structural_changed_true_for_help_or_settings_toggle() {
        let prev = key_for(12, 30, 7, 5, 80, 24);
        let mut curr = prev;
        curr.show_help = true;
        assert!(structural_changed(&prev, &curr));
        let mut curr2 = prev;
        curr2.settings.open = true;
        assert!(structural_changed(&prev, &curr2));
    }
}
