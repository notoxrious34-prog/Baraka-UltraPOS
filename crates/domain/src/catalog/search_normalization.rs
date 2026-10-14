//! Quincaillerie search normalization (TASK-BRK-066).
//!
//! Pure text pipeline behind the hybrid catalog search: French accent
//! folding, Arabic orthographic unification, a bidirectional Algerian
//! hardware phonetic dictionary (French trade terms <-> Arabic
//! transliterations as typed by cashiers), and a multi-token query parser
//! with per-token alias expansion. No I/O, no allocation beyond the
//! returned strings; safe to call from the `baraka_search_norm` SQLite
//! scalar function on every row.

/// One bidirectional dictionary row: canonical French term (stored
/// pre-normalized: lowercase, unaccented) plus the Arabic phonetic
/// transliterations cashiers type instead.
pub struct DictionaryEntry {
    /// Canonical French term, e.g. `boulon`.
    pub french: &'static str,
    /// Arabic transliterations, e.g. `["بولون", "بولونة"]`.
    pub arabic: &'static [&'static str],
}

/// Algerian hardware phonetic dictionary: French trade terms bidirectionally
/// mapped to the Arabic transliterations used at Quincaillerie counters.
pub static ALGERIAN_HARDWARE_DICTIONARY: &[DictionaryEntry] = &[
    // -- Visserie / Fixation -------------------------------------------
    DictionaryEntry {
        french: "vis",
        arabic: &["فيس", "فيص"],
    },
    DictionaryEntry {
        french: "boulon",
        arabic: &["بولون", "بولونة", "بولونيا"],
    },
    DictionaryEntry {
        french: "ecrou",
        arabic: &["اكرو", "إكرو", "لكرو"],
    },
    DictionaryEntry {
        french: "rondelle",
        arabic: &["رونديل", "رونديلا"],
    },
    DictionaryEntry {
        french: "cheville",
        arabic: &["شوفي", "شفي", "شيغفي", "شفيي"],
    },
    DictionaryEntry {
        french: "placo",
        arabic: &["بلاكو", "بلاكوبلاتر"],
    },
    DictionaryEntry {
        french: "tige",
        arabic: &["تيج"],
    },
    DictionaryEntry {
        french: "rivet",
        arabic: &["ريفي", "ريفاي"],
    },
    DictionaryEntry {
        french: "clou",
        arabic: &["كلو", "مسمار"],
    },
    DictionaryEntry {
        french: "pointe",
        arabic: &["بوانت"],
    },
    // -- Plomberie / Sanitaire ------------------------------------------
    DictionaryEntry {
        french: "coude",
        arabic: &["كود"],
    },
    DictionaryEntry {
        french: "te",
        arabic: &["تي"],
    },
    DictionaryEntry {
        french: "manchon",
        arabic: &["مانشون", "مانشو"],
    },
    DictionaryEntry {
        french: "mamelon",
        arabic: &["ماملون", "ماملو", "نيبل"],
    },
    DictionaryEntry {
        french: "bouchon",
        arabic: &["بوشون", "بوشو"],
    },
    DictionaryEntry {
        french: "raccord",
        arabic: &["راكور", "غيكور"],
    },
    DictionaryEntry {
        french: "vanne",
        arabic: &["فان", "فانا"],
    },
    DictionaryEntry {
        french: "robinet",
        arabic: &["روبيني", "روبينيات"],
    },
    DictionaryEntry {
        french: "mitigeur",
        arabic: &["ميتيجور", "ميكسور"],
    },
    DictionaryEntry {
        french: "melangeur",
        arabic: &["ميلونجور"],
    },
    DictionaryEntry {
        french: "siphon",
        arabic: &["سيفون"],
    },
    DictionaryEntry {
        french: "flexible",
        arabic: &["فلكسيبل"],
    },
    DictionaryEntry {
        french: "flotteur",
        arabic: &["فلوتور"],
    },
    DictionaryEntry {
        french: "clapet",
        arabic: &["كلابي"],
    },
    DictionaryEntry {
        french: "teflon",
        arabic: &["تيفلون"],
    },
    DictionaryEntry {
        french: "joint",
        arabic: &["جوان"],
    },
    DictionaryEntry {
        french: "pvc",
        arabic: &["بي في سي"],
    },
    DictionaryEntry {
        french: "ppr",
        arabic: &["بي بي ار"],
    },
    DictionaryEntry {
        french: "cuivre",
        arabic: &["نحاس"],
    },
    // -- Électricité ------------------------------------------------------
    DictionaryEntry {
        french: "cable",
        arabic: &["كابل"],
    },
    DictionaryEntry {
        french: "fil",
        arabic: &["سلك", "خيط"],
    },
    DictionaryEntry {
        french: "disjoncteur",
        arabic: &["ديجونكتور"],
    },
    DictionaryEntry {
        french: "differentiel",
        arabic: &["ديفيرونسيال"],
    },
    DictionaryEntry {
        french: "interrupteur",
        arabic: &["انتربتور", "انتربتار"],
    },
    DictionaryEntry {
        french: "prise",
        arabic: &["بريز", "بريزة"],
    },
    DictionaryEntry {
        french: "fusible",
        arabic: &["فوزيبل"],
    },
    DictionaryEntry {
        french: "douille",
        arabic: &["دوي"],
    },
    DictionaryEntry {
        french: "spot",
        arabic: &["سبوت"],
    },
    DictionaryEntry {
        french: "lampe",
        arabic: &["لامب", "لومبا"],
    },
    DictionaryEntry {
        french: "projecteur",
        arabic: &["بروجكتور"],
    },
    DictionaryEntry {
        french: "goulotte",
        arabic: &["غولوت"],
    },
    DictionaryEntry {
        french: "domino",
        arabic: &["دومينو"],
    },
    // -- Outillage ----------------------------------------------------------
    DictionaryEntry {
        french: "pince",
        arabic: &["بينس", "بانس"],
    },
    DictionaryEntry {
        french: "tournevis",
        arabic: &["تورنوفيس"],
    },
    DictionaryEntry {
        french: "marteau",
        arabic: &["مارطو"],
    },
    DictionaryEntry {
        french: "cle",
        arabic: &["كلي"],
    },
    DictionaryEntry {
        french: "molette",
        arabic: &["موليت"],
    },
    DictionaryEntry {
        french: "scie",
        arabic: &["منشار", "سي"],
    },
    DictionaryEntry {
        french: "disque",
        arabic: &["ديسك"],
    },
    DictionaryEntry {
        french: "meuleuse",
        arabic: &["مولوز"],
    },
    DictionaryEntry {
        french: "perceuse",
        arabic: &["بيرسوز"],
    },
    DictionaryEntry {
        french: "foret",
        arabic: &["فوري", "ميش"],
    },
    DictionaryEntry {
        french: "metre",
        arabic: &["ميترو"],
    },
    DictionaryEntry {
        french: "niveau",
        arabic: &["نيفو"],
    },
    DictionaryEntry {
        french: "cutter",
        arabic: &["كوتور"],
    },
    DictionaryEntry {
        french: "serre-joint",
        arabic: &["سيرجوان", "سيرفوانت"],
    },
    // -- Droguerie / Peinture / Matériaux -----------------------------------
    DictionaryEntry {
        french: "peinture",
        arabic: &["بنتورة", "صباغة"],
    },
    DictionaryEntry {
        french: "rouleau",
        arabic: &["رولو", "غولو"],
    },
    DictionaryEntry {
        french: "pinceau",
        arabic: &["بينسو"],
    },
    DictionaryEntry {
        french: "silicone",
        arabic: &["سيليكون"],
    },
    DictionaryEntry {
        french: "mousse",
        arabic: &["موس"],
    },
    DictionaryEntry {
        french: "scotch",
        arabic: &["سكوتش"],
    },
    DictionaryEntry {
        french: "colle",
        arabic: &["كولا", "كولستون"],
    },
    DictionaryEntry {
        french: "diluant",
        arabic: &["ديليون"],
    },
    DictionaryEntry {
        french: "platre",
        arabic: &["بلاتر", "جبس"],
    },
    DictionaryEntry {
        french: "ciment",
        arabic: &["سيمة", "سيمو"],
    },
    DictionaryEntry {
        french: "brouette",
        arabic: &["برويطة"],
    },
    DictionaryEntry {
        french: "pelle",
        arabic: &["بالة"],
    },
    DictionaryEntry {
        french: "pioche",
        arabic: &["بيوش"],
    },
    DictionaryEntry {
        french: "truelle",
        arabic: &["مسطرين"],
    },
    DictionaryEntry {
        french: "seau",
        arabic: &["سطل", "بيدون"],
    },
    DictionaryEntry {
        french: "corde",
        arabic: &["حبل", "كوردة"],
    },
];

/// Fold French/Latin accents to ASCII and strip Arabic diacritics
/// (tashkeel `U+064B..=U+0652`, dagger alef `U+0670`). Pure.
pub fn fold_accents_and_diacritics(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => {
                out.push('a');
            }
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => {
                out.push('e');
            }
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => {
                out.push('i');
            }
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' => {
                out.push('o');
            }
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => {
                out.push('u');
            }
            'ç' | 'Ç' => {
                out.push('c');
            }
            'ñ' | 'Ñ' => {
                out.push('n');
            }
            'œ' => {
                out.push_str("oe");
            }
            'Œ' => {
                out.push_str("OE");
            }
            'æ' => {
                out.push_str("ae");
            }
            'Æ' => {
                out.push_str("AE");
            }
            '\u{064B}'..='\u{0652}' | '\u{0670}' => {
                // Tashkeel + dagger alef: silent for search.
            }
            other => {
                out.push(other);
            }
        }
    }
    out
}

/// Unify Arabic orthographic variants: alef forms (`آأإٱ`) to `ا`,
/// taa marbuta (`ة`) to `ه`, alef maqsura (`ى`) to `ي`, and Persian/Urdu
/// `ک`/`ی` to `ك`/`ي`. Pure.
pub fn normalize_arabic(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '\u{0622}' | '\u{0623}' | '\u{0625}' | '\u{0671}' => '\u{0627}',
            '\u{0629}' => '\u{0647}',
            '\u{0649}' => '\u{064A}',
            '\u{06A9}' => '\u{0643}',
            '\u{06CC}' => '\u{064A}',
            other => other,
        })
        .collect()
}

/// Unified search normalization: lowercase, accent/diacritic folding,
/// Arabic unification, single-space collapse. Pure.
pub fn normalize_for_search(input: &str) -> String {
    let lowered: String = input.chars().flat_map(char::to_lowercase).collect();
    let folded = fold_accents_and_diacritics(&lowered);
    let unified = normalize_arabic(&folded);
    unified.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Expand one query token to itself plus every matched phonetic alias from
/// [`ALGERIAN_HARDWARE_DICTIONARY`] (both directions). The input is
/// normalized first, so raw cashier input is accepted. Pure.
pub fn expand_token_variants(token: &str) -> Vec<String> {
    let normalized = normalize_for_search(token);
    let mut variants = vec![normalized.clone()];
    for entry in ALGERIAN_HARDWARE_DICTIONARY {
        if entry.french == normalized {
            for alias in entry.arabic {
                let folded = normalize_for_search(alias);
                if !variants.iter().any(|known| known == &folded) {
                    variants.push(folded);
                }
            }
        } else if entry
            .arabic
            .iter()
            .any(|alias| normalize_for_search(alias) == normalized)
            && !variants.iter().any(|known| known == entry.french)
        {
            variants.push(entry.french.to_string());
        }
    }
    variants
}

/// Split a raw query into at most 5 token candidate groups (AND logic
/// downstream: every group must match). Pure.
pub fn tokenize_query(query: &str) -> Vec<Vec<String>> {
    normalize_for_search(query)
        .split_whitespace()
        .take(5)
        .map(expand_token_variants)
        .filter(|group| !group.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_french_accents_to_ascii() {
        assert_eq!(normalize_for_search("Câble électrique"), "cable electrique");
        assert_eq!(fold_accents_and_diacritics("ÀÇÑŒÆç"), "acnOEAEc");
        assert_eq!(normalize_for_search("ÇÂBLE 2.5MM"), "cable 2.5mm");
    }

    #[test]
    fn strips_arabic_diacritics() {
        assert_eq!(normalize_for_search("بُرغِي إِسْمَنْتِيّ 3.5"), "برغي اسمنتي 3.5");
    }

    #[test]
    fn unifies_taa_marbuta_alef_maqsurah_and_alef() {
        assert_eq!(normalize_for_search("بكرة ألومنيوم"), "بكره الومنيوم");
        assert_eq!(
            normalize_arabic("آأإٱةىکی"),
            "\u{0627}\u{0627}\u{0627}\u{0627}\u{0647}\u{064A}\u{0643}\u{064A}"
        );
    }

    #[test]
    fn dictionary_maps_both_directions() {
        let from_arabic = expand_token_variants("فيس");
        assert!(from_arabic.contains(&"فيس".to_string()));
        assert!(from_arabic.contains(&"vis".to_string()));
        let from_french = expand_token_variants("coude");
        assert!(from_french.contains(&"coude".to_string()));
        assert!(from_french.contains(&"كود".to_string()));
        // Unknown tokens expand to themselves only.
        assert_eq!(
            expand_token_variants("zzzinox"),
            vec!["zzzinox".to_string()]
        );
    }

    #[test]
    fn tokenizer_builds_phonetic_clauses() {
        let groups = tokenize_query("فيس بلاكو 25");
        assert_eq!(groups.len(), 3);
        assert!(groups[0].contains(&"vis".to_string()));
        assert!(groups[1].contains(&"placo".to_string()));
        assert_eq!(groups[2], vec!["25".to_string()]);
        // Five-token cap.
        assert_eq!(tokenize_query("a b c d e f g").len(), 5);
        // Blank queries yield no groups.
        assert!(tokenize_query("   ").is_empty());
    }

    #[test]
    fn dictionary_covers_seventy_plus_terms() {
        assert!(
            ALGERIAN_HARDWARE_DICTIONARY.len() >= 70,
            "dictionary has {} entries",
            ALGERIAN_HARDWARE_DICTIONARY.len()
        );
        for entry in ALGERIAN_HARDWARE_DICTIONARY {
            assert!(!entry.french.is_empty());
            assert!(!entry.arabic.is_empty());
            assert_eq!(entry.french, normalize_for_search(entry.french));
        }
    }
}
