//! Joins the lines a text recogniser returns into sentences for translation (spec 004 research
//! R6). Pure: only text and boxes go in, so every rule is testable without an OS or an image.
//!
//! Boxes use a top-left origin in any unit (pixels or points); only ratios between them matter.

use crate::platform::RecognisedLine;

/// A gap above this many median line heights starts a new block.
const BLOCK_GAP: f64 = 0.9;
/// A left edge that moves by more than this many median line heights starts a new block.
const INDENT_SHIFT: f64 = 1.5;
/// A line this many times taller than the median is a heading and stands alone.
const HEADING_RATIO: f64 = 1.4;
/// A box taller than wide by this factor is vertical text.
const VERTICAL_RATIO: f64 = 1.5;
/// Two lines whose centres are within this many median heights share a row.
const ROW_TOLERANCE: f64 = 0.4;

struct Entry {
    text: String,
    x: f64,
    top: f64,
    bottom: f64,
    height: f64,
    /// Not the first cell of its row: it stays a line of its own.
    cell: bool,
}

/// The text to translate for what the recogniser saw; empty when there is none.
pub fn join_lines(lines: &[RecognisedLine]) -> String {
    let mut horizontal = Vec::new();
    let mut vertical = Vec::new();
    for line in lines {
        let text = collapse(&line.text);
        if text.is_empty() {
            continue;
        }
        let is_vertical =
            text.chars().count() >= 2 && line.rect.height > line.rect.width * VERTICAL_RATIO;
        if is_vertical {
            vertical.push(text);
        } else {
            horizontal.push((text, line));
        }
    }

    let mut output: Vec<String> = Vec::new();
    if !horizontal.is_empty() {
        output.extend(join_horizontal(&horizontal));
    }
    // Vertical columns are not joined geometrically: the recogniser's order is kept and every
    // column is its own line (CJK needs no space, so the translation still reads as text).
    output.extend(vertical);
    output.join("\n")
}

fn join_horizontal(lines: &[(String, &RecognisedLine)]) -> Vec<String> {
    let median = lower_median(lines.iter().map(|(_, line)| line.rect.height).collect());
    let median = if median > 0.0 { median } else { 1.0 };

    let mut order: Vec<usize> = (0..lines.len()).collect();
    let centre = |index: usize| lines[index].1.rect.y + lines[index].1.rect.height / 2.0;
    order.sort_by(|a, b| centre(*a).total_cmp(&centre(*b)));

    // Group into rows by vertical centre, then read each row left to right.
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for index in order {
        match rows.last_mut() {
            Some(row)
                if (centre(index) - mean(row.iter().map(|i| centre(*i)))).abs()
                    <= median * ROW_TOLERANCE =>
            {
                row.push(index)
            }
            _ => rows.push(vec![index]),
        }
    }

    let mut entries: Vec<Entry> = Vec::new();
    for mut row in rows {
        row.sort_by(|a, b| lines[*a].1.rect.x.total_cmp(&lines[*b].1.rect.x));
        for (position, index) in row.into_iter().enumerate() {
            let (text, line) = &lines[index];
            entries.push(Entry {
                text: text.clone(),
                x: line.rect.x,
                top: line.rect.y,
                bottom: line.rect.y + line.rect.height,
                height: line.rect.height,
                cell: position > 0,
            });
        }
    }

    let mut output: Vec<String> = Vec::new();
    let mut previous: Option<&Entry> = None;
    for entry in &entries {
        let continues = previous
            .map(|before| continues_block(before, entry, median))
            .unwrap_or(false);
        if continues {
            let last = output
                .last_mut()
                .expect("a continuing line has a predecessor");
            join_into(last, &entry.text);
        } else {
            output.push(entry.text.clone());
        }
        previous = Some(entry);
    }
    output
}

fn continues_block(before: &Entry, entry: &Entry, median: f64) -> bool {
    if entry.cell {
        return false;
    }
    if starts_with_marker(&entry.text) {
        return false;
    }
    if before.height > median * HEADING_RATIO || entry.height > median * HEADING_RATIO {
        return false;
    }
    if entry.top - before.bottom > median * BLOCK_GAP {
        return false;
    }
    if (entry.x - before.x).abs() > median * INDENT_SHIFT {
        return false;
    }
    true
}

/// Appends `next` to `text`: hyphenated words are re-joined and scripts written without spaces
/// are not separated.
fn join_into(text: &mut String, next: &str) {
    let first = next.chars().next().unwrap_or(' ');
    if let Some(stripped) = text.strip_suffix('-') {
        let before = stripped.chars().last();
        if before.is_some_and(char::is_alphanumeric) {
            if first.is_lowercase() {
                text.pop();
            }
            text.push_str(next);
            return;
        }
    }
    let last = text.chars().last().unwrap_or(' ');
    if !(is_unspaced(last) || is_unspaced(first)) {
        text.push(' ');
    }
    text.push_str(next);
}

/// Scripts whose words are not separated by spaces: CJK, kana, Hangul, Thai and full-width forms.
fn is_unspaced(c: char) -> bool {
    matches!(c as u32,
        0x0E00..=0x0E7F // Thai
        | 0x3000..=0x30FF // CJK punctuation, hiragana, katakana
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xAC00..=0xD7AF // Hangul
        | 0xFF00..=0xFFEF)
}

/// `•`, `-`, `1.`, `1)`, `a)`, `(1)` and friends at the start of a line.
fn starts_with_marker(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let rest = chars.as_str();
    let spaced = |tail: &str| tail.is_empty() || tail.starts_with(char::is_whitespace);
    match first {
        '•' | '·' | '▪' | '◦' | '●' => true,
        '-' | '–' | '—' | '*' => spaced(rest),
        '(' => {
            // (1) or (a)
            let inner: String = rest.chars().take_while(|c| *c != ')').collect();
            let after = &rest[inner.len()..];
            after.starts_with(')')
                && spaced(&after[1..])
                && ((!inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit()))
                    || (inner.chars().count() == 1 && inner.chars().all(char::is_alphabetic)))
        }
        c if c.is_ascii_digit() => {
            let digits = text.chars().take_while(char::is_ascii_digit).count();
            let tail = &text[digits..];
            (tail.starts_with('.') || tail.starts_with(')')) && spaced(&tail[1..])
        }
        c if c.is_alphabetic() => rest.starts_with(')') && spaced(&rest[1..]),
        _ => false,
    }
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn lower_median(mut values: Vec<f64>) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    values[(values.len() - 1) / 2]
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0, 0.0), |(sum, count), value| (sum + value, count + 1.0));
    if count == 0.0 {
        0.0
    } else {
        sum / count
    }
}
