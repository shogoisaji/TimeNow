//! 5x5 ビットマップデジタルフォント。
//!
//! 各グリフは5行 x 5列のパターン。`'#'` が描画ピクセル、`' '` が空白。
//! レンダラはこれを拡大して描画する。
//!
//! 複数のフォントセット(パターン)を切り替え可能。

/// 利用可能なフォントセット(パターン)。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Font {
    /// 太めの7x7デジタル表示。時計のデフォルト向け。
    Neo,
    /// ブロック体(従来の塗りつぶしスタイル)。
    #[default]
    Block,
    /// アウトライン体(輪郭のみ。Block と明確に見た目が違う)。
    Outline,
    /// 7セグメント風(デジタル時計の液晶表示風。細いセグメント)。
    Segment,
    /// ドットマトリクス風(選択肢からは外している旧フォント)。
    Dot,
    /// 斜体風(選択肢からは外している旧フォント)。
    Slant,
}

impl Font {
    /// 定義済みフォントを順番に返す(cycling用)。
    pub fn all() -> &'static [Font] {
        &[Font::Neo, Font::Block, Font::Outline, Font::Segment]
    }

    /// 次のフォントへ(cycling)。
    pub fn next(self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|&f| f == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }
}

/// グリフのパターン(5x5フォントのみ)。7x7フォント(Neo/Segment)は
/// `render_glyph` / `glyph_width` / `glyph_height` 経由で取得すること。
pub fn glyph(font: Font, c: char) -> &'static [&'static str; 5] {
    match font {
        Font::Neo => glyph_block(c),
        Font::Block => glyph_block(c),
        Font::Outline => glyph_outline(c),
        Font::Segment => glyph_block(c), // 5x5 fallback; 本来は render_glyph を使用
        Font::Dot => glyph_dot(c),
        Font::Slant => glyph_slant(c),
    }
}

pub fn render_glyph(font: Font, c: char) -> &'static [&'static str] {
    match font {
        Font::Neo => glyph_neo(c),
        Font::Segment => glyph_segment(c),
        _ => glyph(font, c),
    }
}

pub fn glyph_width(font: Font) -> usize {
    match font {
        Font::Neo | Font::Segment => 7,
        _ => GLYPH_W,
    }
}

pub fn glyph_height(font: Font) -> usize {
    match font {
        Font::Neo | Font::Segment => 7,
        _ => GLYPH_H,
    }
}

fn glyph_neo(c: char) -> &'static [&'static str; 7] {
    match c {
        '0' => &[
            " ##### ", "##   ##", "##   ##", "##   ##", "##   ##", "##   ##", " ##### ",
        ],
        '1' => &[
            "   ##  ", " ####  ", "   ##  ", "   ##  ", "   ##  ", "   ##  ", " ######",
        ],
        '2' => &[
            " ##### ", "##   ##", "     ##", "   ### ", " ###   ", "##     ", "#######",
        ],
        '3' => &[
            "###### ", "     ##", "     ##", " ##### ", "     ##", "     ##", "###### ",
        ],
        '4' => &[
            "##   ##", "##   ##", "##   ##", "#######", "     ##", "     ##", "     ##",
        ],
        '5' => &[
            "#######", "##     ", "##     ", "###### ", "     ##", "##   ##", " ##### ",
        ],
        '6' => &[
            " ##### ", "##     ", "##     ", "###### ", "##   ##", "##   ##", " ##### ",
        ],
        '7' => &[
            "#######", "     ##", "    ## ", "   ##  ", "  ##   ", "  ##   ", "  ##   ",
        ],
        '8' => &[
            " ##### ", "##   ##", "##   ##", " ##### ", "##   ##", "##   ##", " ##### ",
        ],
        '9' => &[
            " ##### ", "##   ##", "##   ##", " ######", "     ##", "     ##", " ##### ",
        ],
        ':' => &[
            "       ", "  ##   ", "  ##   ", "       ", "  ##   ", "  ##   ", "       ",
        ],
        ' ' => &[
            "       ", "       ", "       ", "       ", "       ", "       ", "       ",
        ],
        _ => &[
            "       ", " ####  ", "##  ## ", "   ##  ", "       ", "  ##   ", "       ",
        ],
    }
}

fn glyph_block(c: char) -> &'static [&'static str; 5] {
    match c.to_ascii_uppercase() {
        '0' => &["#####", "#   #", "#   #", "#   #", "#####"],
        '1' => &["  #  ", " ##  ", "# #  ", "  #  ", "#####"],
        '2' => &["#####", "    #", "#####", "#    ", "#####"],
        '3' => &["#####", "    #", "#####", "    #", "#####"],
        '4' => &["#   #", "#   #", "#####", "    #", "    #"],
        '5' => &["#####", "#    ", "#####", "    #", "#####"],
        '6' => &["#####", "#    ", "#####", "#   #", "#####"],
        '7' => &["#####", "    #", "   # ", "  #  ", "  #  "],
        '8' => &["#####", "#   #", "#####", "#   #", "#####"],
        '9' => &["#####", "#   #", "#####", "    #", "#####"],
        ':' => &["     ", "  #  ", "     ", "  #  ", "     "],
        '/' => &["    #", "   # ", "  #  ", " #   ", "#    "],
        'A' => &[" ### ", "#   #", "#####", "#   #", "#   #"],
        'B' => &["#### ", "#   #", "#### ", "#   #", "#### "],
        'C' => &[" ####", "#    ", "#    ", "#    ", " ####"],
        'D' => &["#### ", "#   #", "#   #", "#   #", "#### "],
        'E' => &["#####", "#    ", "#### ", "#    ", "#####"],
        'F' => &["#####", "#    ", "#### ", "#    ", "#    "],
        'G' => &[" ####", "#    ", "#  ##", "#   #", " ####"],
        'H' => &["#   #", "#   #", "#####", "#   #", "#   #"],
        'I' => &["#####", "  #  ", "  #  ", "  #  ", "#####"],
        'J' => &["#####", "   # ", "   # ", "#  # ", " ##  "],
        'K' => &["#   #", "#  # ", "###  ", "#  # ", "#   #"],
        'L' => &["#    ", "#    ", "#    ", "#    ", "#####"],
        'M' => &["#   #", "## ##", "# # #", "#   #", "#   #"],
        'N' => &["#   #", "##  #", "# # #", "#  ##", "#   #"],
        'O' => &[" ### ", "#   #", "#   #", "#   #", " ### "],
        'P' => &["#### ", "#   #", "#### ", "#    ", "#    "],
        'Q' => &[" ### ", "#   #", "# # #", "#  # ", " ## #"],
        'R' => &["#### ", "#   #", "#### ", "#  # ", "#   #"],
        'S' => &[" ####", "#    ", " ### ", "    #", "#### "],
        'T' => &["#####", "  #  ", "  #  ", "  #  ", "  #  "],
        'U' => &["#   #", "#   #", "#   #", "#   #", " ### "],
        'V' => &["#   #", "#   #", "#   #", " # # ", "  #  "],
        'W' => &["#   #", "#   #", "# # #", "## ##", "#   #"],
        'X' => &["#   #", " # # ", "  #  ", " # # ", "#   #"],
        'Y' => &["#   #", " # # ", "  #  ", "  #  ", "  #  "],
        'Z' => &["#####", "   # ", "  #  ", " #   ", "#####"],
        ' ' => &["     ", "     ", "     ", "     ", "     "],
        _ => &["     ", " ##  ", "#  # ", "     ", " #   "],
    }
}

fn glyph_outline(c: char) -> &'static [&'static str; 5] {
    match c {
        '0' => &["#####", "#   #", "#   #", "#   #", "#####"],
        '1' => &["  #  ", " ##  ", "  #  ", "  #  ", "#####"],
        '2' => &["#####", "    #", " ### ", "#    ", "#####"],
        '3' => &["#####", "    #", " ### ", "    #", "#####"],
        '4' => &["#   #", "#   #", "#####", "    #", "    #"],
        '5' => &["#####", "#    ", " ### ", "    #", "#####"],
        '6' => &["#####", "#    ", "#####", "#   #", "#####"],
        '7' => &["#####", "    #", "   # ", "  #  ", "  #  "],
        '8' => &["#####", "#   #", "#####", "#   #", "#####"],
        '9' => &["#####", "#   #", "#####", "    #", "#####"],
        ':' => &["     ", "  #  ", "     ", "  #  ", "     "],
        ' ' => &["     ", "     ", "     ", "     ", "     "],
        _ => &["     ", " ##  ", "#  # ", "     ", " #   "],
    }
}

/// 14セグメント風フォント(7x7)。7セグメントに斜めセグメントを加えた表示。
/// 中段(g)が中央で分割(g1/g2)され、センターにギャップができるのが特徴。
/// 横セグメントは5px(中央寄せ)、縦セグメントは両端2px幅。
/// 中段のみ " ## ## " (2px + gap + 2px) で分割を表現。
/// 全グリフで row0 と row6 に必ずピクセルを置き、高さを揃える。
fn glyph_segment(c: char) -> &'static [&'static str; 7] {
    match c {
        // 14セグメント配置(7x7):
        //   row0: a (上)
        //   row1: f(左上) + b(右上)
        //   row2: f(左上) + b(右上)
        //   row3: g1(中左) + g2(中右) — 中央ギャップ
        //   row4: e(左下) + c(右下)
        //   row5: e(左下) + c(右下)
        //   row6: d (下)
        '0' => &[
            " ##### ", "##   ##", "##   ##", "       ", "##   ##", "##   ##", " ##### ",
        ],
        // 1: 右上(b)+右下(c)セグメント。高さ揃えのため上下端に右縦の先端を配置。
        '1' => &[
            "     ##", "     ##", "     ##", "       ", "     ##", "     ##", "     ##",
        ],
        '2' => &[
            " ##### ", "     ##", "     ##", " ## ## ", "##     ", "##     ", " ##### ",
        ],
        '3' => &[
            " ##### ", "     ##", "     ##", " ## ## ", "     ##", "     ##", " ##### ",
        ],
        // 4: 左上(f)+右上(b)+中(g1/g2)+右下(c)。高さ揃えのため下端に右縦の先端を配置。
        '4' => &[
            "##   ##", "##   ##", "##   ##", " ## ## ", "     ##", "     ##", "     ##",
        ],
        '5' => &[
            " ##### ", "##     ", "##     ", " ## ## ", "     ##", "     ##", " ##### ",
        ],
        '6' => &[
            " ##### ", "##     ", "##     ", " ## ## ", "##   ##", "##   ##", " ##### ",
        ],
        // 7: 上(a)+右上(b)+右下(c)。高さ揃えのため下端に右縦の先端を配置。
        '7' => &[
            " ##### ", "     ##", "     ##", "       ", "     ##", "     ##", "     ##",
        ],
        '8' => &[
            " ##### ", "##   ##", "##   ##", " ## ## ", "##   ##", "##   ##", " ##### ",
        ],
        '9' => &[
            " ##### ", "##   ##", "##   ##", " ## ## ", "     ##", "     ##", " ##### ",
        ],
        ':' => &[
            "       ", "  ##   ", "  ##   ", "       ", "  ##   ", "  ##   ", "       ",
        ],
        ' ' => &[
            "       ", "       ", "       ", "       ", "       ", "       ", "       ",
        ],
        _ => &[
            "       ", " ####  ", "##  ## ", "   ##  ", "       ", "  ##   ", "       ",
        ],
    }
}

/// ドットマトリクス風フォント。横棒も `# # #` の点列で表現してドット感を強める。
fn glyph_dot(c: char) -> &'static [&'static str; 5] {
    match c {
        '0' => &["# # #", "#   #", "#   #", "#   #", "# # #"],
        '1' => &["  #  ", "  #  ", "  #  ", "  #  ", "  #  "],
        '2' => &["# # #", "    #", "# # #", "#    ", "# # #"],
        '3' => &["# # #", "    #", "# # #", "    #", "# # #"],
        '4' => &["#   #", "#   #", "# # #", "    #", "    #"],
        '5' => &["# # #", "#    ", "# # #", "    #", "# # #"],
        '6' => &["# # #", "#    ", "# # #", "#   #", "# # #"],
        '7' => &["# # #", "    #", "   # ", "  #  ", "  #  "],
        '8' => &["# # #", "#   #", "# # #", "#   #", "# # #"],
        '9' => &["# # #", "#   #", "# # #", "    #", "# # #"],
        ':' => &["     ", "  #  ", "     ", "  #  ", "     "],
        ' ' => &["     ", "     ", "     ", "     ", "     "],
        _ => &["     ", " ##  ", "#  # ", "     ", " #   "],
    }
}

/// 斜体風フォント。上辺を右寄せ、下辺を左寄せにして動きのある形にする。
fn glyph_slant(c: char) -> &'static [&'static str; 5] {
    match c {
        '0' => &[" ####", "#   #", "#   #", "#   #", "#### "],
        '1' => &["   # ", "  ## ", " # # ", "   # ", "#### "],
        '2' => &[" ####", "    #", " ### ", "#    ", "#### "],
        '3' => &[" ####", "    #", " ### ", "    #", "#### "],
        '4' => &["#   #", "#   #", " ####", "    #", "    #"],
        '5' => &[" ####", "#    ", "#### ", "    #", "#### "],
        '6' => &[" ####", "#    ", "#### ", "#   #", "#### "],
        '7' => &[" ####", "   # ", "  #  ", " #   ", " #   "],
        '8' => &[" ####", "#   #", "#### ", "#   #", "#### "],
        '9' => &[" ####", "#   #", "#### ", "    #", "#### "],
        ':' => &["     ", "  #  ", "     ", "  #  ", "     "],
        ' ' => &["     ", "     ", "     ", "     ", "     "],
        _ => &["     ", " ##  ", "#  # ", "     ", " #   "],
    }
}

/// グリフの幅(ピクセル列数)。
pub const GLYPH_W: usize = 5;
/// グリフの高さ(ピクセル行数)。
pub const GLYPH_H: usize = 5;
/// グリフ間のギャップ(ピクセル列数)。
pub const GAP: usize = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_by_five_fonts_have_correct_dimensions() {
        // Block / Outline は 5x5。Neo / Segment は 7x7(render_glyph 経由)。
        for &font in &[Font::Block, Font::Outline] {
            for c in "0123456789:".chars() {
                let g = glyph(font, c);
                assert_eq!(g.len(), GLYPH_H, "font {font:?} glyph {c} row count");
                for (i, row) in g.iter().enumerate() {
                    assert_eq!(row.len(), GLYPH_W, "font {font:?} glyph {c} row {i} width");
                }
            }
        }
    }

    #[test]
    fn seven_by_seven_fonts_have_correct_dimensions() {
        // Neo / Segment は 7x7(render_glyph 経由)。
        for &font in &[Font::Neo, Font::Segment] {
            for c in "0123456789:".chars() {
                let g = render_glyph(font, c);
                assert_eq!(g.len(), 7, "font {font:?} glyph {c} row count");
                for (i, row) in g.iter().enumerate() {
                    assert_eq!(row.len(), 7, "font {font:?} glyph {c} row {i} width");
                }
            }
        }
    }

    #[test]
    fn block_font_matches_legacy_glyphs() {
        // 従来のブロック体グリフが保持されていること
        assert_eq!(glyph(Font::Block, '0')[0], "#####");
        assert_eq!(glyph(Font::Block, '0')[1], "#   #");
        assert_eq!(glyph(Font::Block, ':')[1], "  #  ");
    }

    #[test]
    fn outline_font_differs_from_block() {
        // Outline は 2 が中抜き(###)になり、Block とは明確に違う
        let block2 = glyph(Font::Block, '2');
        let outline2 = glyph(Font::Outline, '2');
        assert_ne!(block2, outline2, "Block and Outline must differ");
        // Outline の 2 の中段(行2)は " ### " — 両端が空く
        assert_eq!(outline2[2], " ### ");
        assert_eq!(block2[2], "#####");
    }

    #[test]
    fn outline_colon_same_as_block() {
        // コロンは両フォントで同じ形
        assert_eq!(glyph(Font::Block, ':'), glyph(Font::Outline, ':'));
    }

    #[test]
    fn all_fonts_count_is_four() {
        // Neo / Block / Outline / Segment
        assert_eq!(Font::all().len(), 4);
    }

    #[test]
    fn segment_font_uses_7x7_segments() {
        // 14セグメント風: 上下は " ##### "(5px)、中段は " ## ## "(分割)
        let g = render_glyph(Font::Segment, '8');
        assert_eq!(g[0], " ##### ");
        assert_eq!(g[3], " ## ## ");
        assert_eq!(g[6], " ##### ");
        assert_eq!(g[1], "##   ##");
    }

    #[test]
    fn segment_font_one_is_right_segment_pair() {
        // 1 は右上(b)+右下(c)セグメント。高さ揃えのため上下端にも右縦先端。
        let g = render_glyph(Font::Segment, '1');
        assert_eq!(
            g,
            &["     ##", "     ##", "     ##", "       ", "     ##", "     ##", "     ##",]
        );
    }

    #[test]
    fn segment_font_differs_from_block() {
        // Segment(7x7) と Block(5x5) は 8 で異なる
        assert_ne!(
            render_glyph(Font::Block, '8'),
            render_glyph(Font::Segment, '8')
        );
    }

    #[test]
    fn segment_font_differs_from_neo() {
        // Segment と Neo は別物(Neo は塗りつぶし、Segment はセグメント欠けあり)
        let seg0 = render_glyph(Font::Segment, '0');
        let neo0 = render_glyph(Font::Neo, '0');
        // 0 の中段(row3)が Segment は空行、Neo は塗りつぶし
        assert_eq!(seg0[3], "       ");
        assert_ne!(seg0[3], neo0[3]);
    }

    #[test]
    fn dot_font_has_dotted_horizontal_segments() {
        // Dot の横棒は "# # #"(ドット状)
        let g = glyph(Font::Dot, '8');
        assert_eq!(g[0], "# # #");
        assert_eq!(g[2], "# # #");
        assert_eq!(g[4], "# # #");
    }

    #[test]
    fn dot_font_differs_from_block() {
        assert_ne!(glyph(Font::Block, '0'), glyph(Font::Dot, '0'));
    }

    #[test]
    fn slant_font_has_shifted_top_and_bottom() {
        // Slant の 0 は上辺が右寄せ、下辺が左寄せ
        let g = glyph(Font::Slant, '0');
        assert_eq!(g[0], " ####");
        assert_eq!(g[4], "#### ");
    }

    #[test]
    fn slant_font_differs_from_outline() {
        // Slant と Outline は 0 で異なる(Outline は上辺/下辺が全幅)
        assert_ne!(glyph(Font::Outline, '0'), glyph(Font::Slant, '0'));
    }

    #[test]
    fn all_fonts_colon_same() {
        // コロンは全フォントで同じ形
        let colon_block = glyph(Font::Block, ':');
        for &font in Font::all() {
            assert_eq!(
                glyph(font, ':'),
                colon_block,
                "colon should be same in all fonts"
            );
        }
    }

    #[test]
    fn unknown_char_returns_error_glyph_for_all_fonts() {
        for &font in Font::all() {
            let g = glyph(font, '?');
            assert!(
                g.iter().any(|r| r.contains('#')),
                "font {font:?} error glyph should be non-empty"
            );
        }
    }

    #[test]
    fn font_next_cycles_through_all() {
        let all = Font::all();
        let mut f = all[0];
        for _ in 0..all.len() {
            f = f.next();
        }
        // 一周すると元に戻る
        assert_eq!(f, all[0]);
    }

    #[test]
    fn font_next_visits_each_once() {
        let all = Font::all();
        let mut f = all[0];
        let mut visited = vec![f];
        for _ in 0..(all.len() - 1) {
            f = f.next();
            visited.push(f);
        }
        assert_eq!(visited.len(), all.len());
        for &a in all {
            assert!(visited.contains(&a), "font {a:?} should be visited");
        }
    }
}
