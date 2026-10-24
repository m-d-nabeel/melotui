//! Right-to-left text for terminals that lay every line out left to right.

use std::borrow::Cow;
use std::sync::OnceLock;

use ar_reshaper::ArabicReshaper;
use unicode_bidi::{BidiClass, BidiInfo, bidi_class};
use unicode_segmentation::UnicodeSegmentation;

/// Whether the terminal puts right-to-left text into visual order itself.
///
/// VTE — GNOME Terminal and the terminals built on it — Konsole and mlterm
/// reorder each line on their own, joining Arabic-script letters as they go.
/// Text written to them stays in reading order; reordering it here as well
/// would turn it back round.
fn terminal_reorders() -> bool {
    static REORDERS: OnceLock<bool> = OnceLock::new();
    *REORDERS.get_or_init(|| {
        std::env::var_os("VTE_VERSION").is_some()
            || std::env::var_os("KONSOLE_VERSION").is_some()
            || std::env::var("TERM").is_ok_and(|t| t.starts_with("mlterm"))
    })
}

/// `text` as this terminal needs it written: in visual order, unless the
/// terminal reorders right-to-left text itself.
pub fn for_terminal(text: &str) -> Cow<'_, str> {
    if terminal_reorders() {
        Cow::Borrowed(text)
    } else {
        visual(text)
    }
}

/// `text` as it has to be written to a terminal that lays every line out left
/// to right: Arabic-script letters in their joined forms, and right-to-left
/// runs in visual order. Text with nothing right-to-left in it comes back
/// untouched.
///
/// Most terminals — kitty, Ghostty, Alacritty, foot, WezTerm by default — do
/// neither, so Urdu came out with its letters in reading order drawn left to
/// right, and every one in its standalone form: backwards, and unjoined.
pub fn visual(text: &str) -> Cow<'_, str> {
    if !text.chars().any(is_rtl) {
        return Cow::Borrowed(text);
    }
    // Joined first, in reading order: the form a letter takes depends on its
    // neighbours as they are read, not as they end up drawn.
    let joined = ArabicReshaper::default().reshape(text);
    let info = BidiInfo::new(&joined, None);
    let mut out = String::with_capacity(joined.len());
    for para in &info.paragraphs {
        let (levels, runs) = info.visual_runs(para, para.range.clone());
        for run in runs {
            let segment = &joined[run.clone()];
            if levels[run.start].is_rtl() {
                // Reversed by grapheme cluster, so a vowel mark stays on the
                // letter it belongs to.
                for grapheme in segment.graphemes(true).rev() {
                    out.push_str(mirrored(grapheme));
                }
            } else {
                out.push_str(segment);
            }
        }
    }
    Cow::Owned(out)
}

fn is_rtl(c: char) -> bool {
    matches!(bidi_class(c), BidiClass::R | BidiClass::AL)
}

/// The glyph a bracket takes inside right-to-left text, where it faces the
/// other way.
fn mirrored(grapheme: &str) -> &str {
    match grapheme {
        "(" => ")",
        ")" => "(",
        "[" => "]",
        "]" => "[",
        "{" => "}",
        "}" => "{",
        "<" => ">",
        ">" => "<",
        "«" => "»",
        "»" => "«",
        "‹" => "›",
        "›" => "‹",
        _ => grapheme,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_to_right_text_is_left_alone() {
        for text in ["Never gonna give you up", "कभी कभी", "君の名は"] {
            assert!(matches!(visual(text), Cow::Borrowed(_)), "{text}");
        }
    }

    #[test]
    fn hebrew_is_drawn_right_to_left() {
        // Hebrew does not join, so this is the reordering alone.
        assert_eq!(visual("שלום"), "םולש");
    }

    #[test]
    fn urdu_is_joined_and_drawn_right_to_left() {
        // Dal never joins the letter after it, so both letters of "دل" stand
        // alone; joined forms are what the next test is for. Here: both in
        // presentation forms, in visual order, lam drawn first.
        let out = visual("دل");
        let chars: Vec<char> = out.chars().collect();
        assert_eq!(chars, ['\u{FEDD}', '\u{FEA9}'], "{out:?}");
    }

    #[test]
    fn letters_that_connect_take_their_joined_forms() {
        // "بت": beh joins forwards, so it takes its initial form and teh its
        // final form; drawn right to left, teh comes first.
        let chars: Vec<char> = visual("بت").chars().collect();
        assert_eq!(chars, ['\u{FE96}', '\u{FE91}']);
    }

    #[test]
    fn urdu_letters_join_too() {
        // ے, ہ and ی are Urdu letters outside the core Arabic alphabet; a
        // reshaper that only knew Arabic would leave them unjoined.
        let out = visual("تمہارے");
        assert!(
            out.chars().all(|c| ('\u{FB50}'..='\u{FEFF}').contains(&c)),
            "an Urdu letter was left unjoined: {:?}",
            out.chars()
                .map(|c| format!("U+{:04X}", c as u32))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn numbers_and_latin_keep_their_own_direction_inside_urdu() {
        // An Urdu line with a year and an English word: those runs stay left
        // to right and the brackets around the year face the right way.
        let out = visual("دل (2024) remix");
        assert!(out.starts_with("remix (2024) "), "{out}");
    }

    #[test]
    fn a_vowel_mark_stays_on_its_letter() {
        // Hebrew with a niqqud mark: reversing by code point would move the mark
        // onto the wrong letter.
        let out = visual("שָׁלוֹם");
        let first_cluster = out.graphemes(true).next().unwrap();
        assert_eq!(first_cluster, "ם");
        assert!(out.graphemes(true).any(|g| g == "שָׁ"), "{out}");
    }
}
