use crate::normalize_language;

pub(crate) fn translate_text_impl(
    text: String,
    target_lang: String,
    source_lang: Option<String>,
) -> Result<String, String> {
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return Ok(String::new());
    }

    let target = normalize_language(&target_lang);
    if target.is_empty() {
        return Err("Langue cible invalide.".to_string());
    }
    let source = source_lang
        .as_deref()
        .map(normalize_language)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "auto".to_string());
    let url = "https://translate.googleapis.com/translate_a/single";
    let client = reqwest::blocking::Client::new();
    let translate_once = |sl: &str| -> Result<String, String> {
        let params = [
            ("client", "gtx".to_string()),
            ("sl", sl.to_string()),
            ("tl", target.clone()),
            ("dt", "t".to_string()),
            ("q", cleaned.to_string()),
        ];

        let response = match client.post(url).form(&params).send() {
            Ok(resp) => resp,
            Err(_) => client
                .get(url)
                .query(&[
                    ("client", "gtx"),
                    ("sl", sl),
                    ("tl", target.as_str()),
                    ("dt", "t"),
                    ("q", cleaned),
                ])
                .send()
                .map_err(|e| format!("Traduction impossible (reseau): {e}"))?,
        };

        if !response.status().is_success() {
            return Err(format!(
                "Traduction impossible: serveur {}",
                response.status()
            ));
        }

        let payload = response
            .text()
            .map_err(|e| format!("Lecture reponse traduction impossible: {e}"))?;
        let json: serde_json::Value =
            serde_json::from_str(&payload).map_err(|e| format!("Reponse traduction invalide: {e}"))?;

        let mut out = String::new();
        if let Some(segments) = json.get(0).and_then(|v| v.as_array()) {
            for seg in segments {
                if let Some(part) = seg.get(0).and_then(|v| v.as_str()) {
                    out.push_str(part);
                }
            }
        }

        Ok(out.trim().to_string())
    };

    let first = translate_once(&source)?;
    if !first.is_empty() && !first.eq_ignore_ascii_case(cleaned) {
        return Ok(first);
    }

    if source != "auto" {
        let second = translate_once("auto")?;
        if !second.is_empty() {
            return Ok(second);
        }
    }

    if !first.is_empty() {
        return Ok(first);
    }

    Err("La traduction n'a renvoye aucun texte.".to_string())
}

pub(crate) fn apply_voice_commands(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let normalized = trimmed
        .to_lowercase()
        .replace('’', "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut out = format!(" {} ", normalized);

    let escapes = [
        (" le mot point ", " __WORD_POINT__ "),
        (" mot point ", " __WORD_POINT__ "),
        (" le mot virgule ", " __WORD_VIRGULE__ "),
        (" mot virgule ", " __WORD_VIRGULE__ "),
    ];
    for (from, to) in escapes {
        out = out.replace(from, to);
    }

    let replacements = [
        (" ponctuation point d'interrogation ", "? "),
        (" point d'interrogation ", "? "),
        (" point d interrogation ", "? "),
        (" point interrogation ", "? "),
        (" ponctuation point d'exclamation ", "! "),
        (" point d'exclamation ", "! "),
        (" point d exclamation ", "! "),
        (" point exclamation ", "! "),
        (" ponctuation point virgule ", "; "),
        (" point virgule ", "; "),
        (" ponctuation deux-points ", ": "),
        (" deux-points ", ": "),
        (" deux points ", ": "),
        (" ponctuation virgule ", ", "),
        (" virgule ", ", "),
        (" ponctuation point ", ". "),
        (" point final ", ". "),
        (" ouvrir parenthèse ", " ("),
        (" ouvrir parenthese ", " ("),
        (" fermer parenthèse ", ") "),
        (" fermer parenthese ", ") "),
        (" nouvelle ligne ", "\n"),
        (" retour a la ligne ", "\n"),
        (" retour à la ligne ", "\n"),
        (" retour ligne ", "\n"),
    ];

    for (from, to) in replacements {
        out = out.replace(from, to);
    }

    let newline_commands = [
        "nouvelle ligne",
        "retour a la ligne",
        "retour à la ligne",
        "retour ligne",
    ];
    for _ in 0..2 {
        for command in newline_commands {
            out = out.replace(&format!("\n{command} "), "\n");
            out = out.replace(&format!(" {command}\n"), "\n");
            out = out.replace(&format!(" {command} "), "\n");
            out = out.replace(&format!("{command} "), "\n");
            out = out.replace(&format!(" {command}"), "\n");
        }
    }

    out = out
        .replace(" .", ".")
        .replace(" ,", ",")
        .replace(" ;", ";")
        .replace(" :", ":")
        .replace(" ?", "?")
        .replace(" !", "!")
        .replace("( ", "(")
        .replace(" )", ")");

    out = out.replace("\r\n", "\n").replace('\r', "\n");
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out = out.replace(" \n", "\n").replace("\n ", "\n");
    out = out
        .replace("__WORD_POINT__", "point")
        .replace("__WORD_VIRGULE__", "virgule");

    capitalize_sentences(out.trim())
}

pub(crate) fn capitalize_sentences(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut capitalize_next = true;
    let mut previous_non_whitespace: Option<char> = None;
    let mut consecutive_newlines: usize = 0;

    for c in input.chars() {
        if c == '\n' {
            consecutive_newlines += 1;
        } else if !c.is_whitespace() {
            consecutive_newlines = 0;
        }

        if capitalize_next && c.is_alphabetic() {
            for up in c.to_uppercase() {
                output.push(up);
            }
            capitalize_next = false;
            previous_non_whitespace = Some(c);
            continue;
        }

        output.push(c);

        if matches!(c, '.' | '!' | '?') {
            capitalize_next = true;
        } else if c == '\n' {
            // Keep paragraph-start capitalization for explicit blank lines,
            // but avoid forcing uppercase after a single line break (e.g. after a comma).
            if consecutive_newlines >= 2 && previous_non_whitespace.is_some() {
                capitalize_next = true;
            }
        } else if !c.is_whitespace() {
            capitalize_next = false;
            previous_non_whitespace = Some(c);
        }
    }

    output
}
