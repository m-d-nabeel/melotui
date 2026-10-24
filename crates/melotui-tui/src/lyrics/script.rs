//! Which text the terminal's grid can draw, and which it cannot.

/// Whether `text` contains a script that a grid of equal cells cannot draw:
/// one whose glyphs fuse, change shape or move depending on their neighbours,
/// as in the Indic scripts, or that runs right to left.
pub fn needs_shaping(text: &str) -> bool {
    text.chars().any(is_complex)
}

fn is_complex(c: char) -> bool {
    matches!(
        u32::from(c),
        // Hebrew, Arabic, Syriac, Thaana, N'Ko, Samaritan, Mandaic, and the
        // Arabic supplements and extensions.
        0x0590..=0x08FF
        // Devanagari, Bengali, Gurmukhi, Gujarati, Oriya, Tamil, Telugu,
        // Kannada, Malayalam, Sinhala.
        | 0x0900..=0x0DFF
        // Thai, Lao, Tibetan.
        | 0x0E00..=0x0FFF
        // Myanmar.
        | 0x1000..=0x109F
        // Khmer, Mongolian, and the Southeast Asian scripts that follow.
        | 0x1780..=0x1C4F
        // Vedic marks.
        | 0x1CD0..=0x1CFF
        // Brahmic and Southeast Asian extension blocks.
        | 0xA800..=0xAAFF
        // Meetei Mayek.
        | 0xABC0..=0xABFF
        // Hebrew and Arabic presentation forms.
        | 0xFB1D..=0xFDFF
        | 0xFE70..=0xFEFF
        // Historic right-to-left and Brahmic scripts.
        | 0x10800..=0x11FFF
        // Adlam, Mende Kikakui, Arabic mathematical letters.
        | 0x1E800..=0x1EEFF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex_scripts_are_recognised() {
        for text in [
            "कभी कभी मेरे दिल में",
            "تمہارے بغیر",
            "שלום",
            "ਪੰਜਾਬੀ",
            "தமிழ்",
            "ภาษาไทย",
        ] {
            assert!(needs_shaping(text), "{text}");
        }
    }

    #[test]
    fn scripts_the_grid_draws_are_left_to_it() {
        for text in [
            "Never gonna give you up",
            "Café déjà vu",
            "君の名は",
            "사랑해",
            "🎵 ♪",
            "",
        ] {
            assert!(!needs_shaping(text), "{text}");
        }
    }

    #[test]
    fn one_word_of_a_complex_script_is_enough() {
        assert!(needs_shaping("Tum hi ho, तुम ही हो"));
    }
}
