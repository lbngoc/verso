//! T024 — the line joiner (spec 004 research R6): recogniser lines in, text to translate out.

use verso_lib::core::ocr_text::join_lines;
use verso_lib::platform::{RecognisedLine, Rect};

/// A horizontal line of `height` at (`x`, `y`); the width follows the text length.
fn line(text: &str, x: f64, y: f64, height: f64) -> RecognisedLine {
    RecognisedLine {
        text: text.into(),
        rect: Rect {
            x,
            y,
            width: text.chars().count() as f64 * height * 0.5,
            height,
        },
    }
}

#[test]
fn wrapped_lines_of_one_paragraph_become_one_line() {
    let lines = [
        line("The quick brown fox", 10.0, 10.0, 20.0),
        line("jumps over the lazy", 10.0, 34.0, 20.0),
        line("dog near the river bank.", 10.0, 58.0, 20.0),
    ];
    assert_eq!(
        join_lines(&lines),
        "The quick brown fox jumps over the lazy dog near the river bank."
    );
}

#[test]
fn a_hyphen_before_a_lowercase_letter_is_removed() {
    let lines = [
        line("Machine trans-", 10.0, 10.0, 20.0),
        line("lation is useful.", 10.0, 34.0, 20.0),
    ];
    assert_eq!(join_lines(&lines), "Machine translation is useful.");
}

#[test]
fn a_hyphen_before_a_non_lowercase_character_is_kept() {
    let lines = [
        line("In Covid-", 10.0, 10.0, 20.0),
        line("19 times", 10.0, 34.0, 20.0),
    ];
    assert_eq!(join_lines(&lines), "In Covid-19 times");
}

#[test]
fn list_markers_start_new_items() {
    for marker in ["•", "-", "*", "▪", "1.", "1)", "a)", "(1)"] {
        let first = format!("{marker} first item");
        let second = format!("{marker} second item");
        let lines = [
            line(&first, 10.0, 10.0, 20.0),
            line(&second, 10.0, 34.0, 20.0),
        ];
        assert_eq!(join_lines(&lines), format!("{first}\n{second}"), "{marker}");
    }
}

#[test]
fn a_decimal_number_is_not_a_marker() {
    let lines = [
        line("The price rose by", 10.0, 10.0, 20.0),
        line("3.5 percent", 10.0, 34.0, 20.0),
    ];
    assert_eq!(join_lines(&lines), "The price rose by 3.5 percent");
}

#[test]
fn a_large_vertical_gap_starts_a_new_block() {
    let lines = [
        line("First paragraph ends here.", 10.0, 10.0, 20.0),
        line("Second paragraph starts here.", 10.0, 80.0, 20.0),
    ];
    assert_eq!(
        join_lines(&lines),
        "First paragraph ends here.\nSecond paragraph starts here."
    );
}

#[test]
fn a_shift_of_the_left_edge_starts_a_new_block() {
    let lines = [
        line("Left column text", 10.0, 10.0, 20.0),
        line("Indented far away", 90.0, 34.0, 20.0),
    ];
    assert_eq!(join_lines(&lines), "Left column text\nIndented far away");
}

#[test]
fn a_taller_heading_stands_alone() {
    let lines = [
        line("Big Title", 10.0, 10.0, 32.0),
        line("Body text goes here", 10.0, 48.0, 20.0),
        line("and continues.", 10.0, 72.0, 20.0),
    ];
    assert_eq!(
        join_lines(&lines),
        "Big Title\nBody text goes here and continues."
    );
}

#[test]
fn cells_on_one_row_stay_in_left_to_right_order() {
    // Fed right-to-left on purpose: reading order is rebuilt from the geometry.
    let lines = [
        line("Price", 200.0, 12.0, 20.0),
        line("Name", 10.0, 10.0, 20.0),
        line("12.50", 200.0, 38.0, 20.0),
        line("Apple", 10.0, 36.0, 20.0),
    ];
    let joined = join_lines(&lines);
    let rows: Vec<&str> = joined.lines().collect();
    assert_eq!(rows[0], "Name");
    assert_eq!(rows[1], "Price");
    assert!(joined.find("Apple").unwrap() < joined.find("12.50").unwrap());
}

#[test]
fn cjk_is_joined_without_a_space() {
    let lines = [
        line("今日は天気が", 10.0, 10.0, 20.0),
        line("とても良いです", 10.0, 34.0, 20.0),
    ];
    assert_eq!(join_lines(&lines), "今日は天気がとても良いです");
    // Kana next to a Latin word: still no space at the join.
    let mixed = [
        line("Hello こんにちは", 10.0, 10.0, 20.0),
        line("世界", 10.0, 34.0, 20.0),
    ];
    assert_eq!(join_lines(&mixed), "Hello こんにちは世界");
}

#[test]
fn vietnamese_diacritics_join_with_a_space() {
    let lines = [
        line("Chúc mừng năm mới", 10.0, 10.0, 20.0),
        line("và vạn sự như ý", 10.0, 34.0, 20.0),
    ];
    assert_eq!(join_lines(&lines), "Chúc mừng năm mới và vạn sự như ý");
}

#[test]
fn vertical_lines_are_not_joined_and_keep_their_order() {
    let column = |text: &str, x: f64| RecognisedLine {
        text: text.into(),
        rect: Rect {
            x,
            y: 10.0,
            width: 20.0,
            height: 120.0,
        },
    };
    let lines = [column("縦書きの文章", 100.0), column("二列目の文章", 70.0)];
    assert_eq!(join_lines(&lines), "縦書きの文章\n二列目の文章");
}

#[test]
fn whitespace_is_collapsed_and_nothing_gives_nothing() {
    assert_eq!(
        join_lines(&[line("  a   b \t c  ", 0.0, 0.0, 20.0)]),
        "a b c"
    );
    assert_eq!(join_lines(&[]), "");
    assert_eq!(join_lines(&[line("   ", 0.0, 0.0, 20.0)]), "");
}
