use regex::Regex;
use std::sync::OnceLock;

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap()
}

pub fn sanitize_html_for_llm(html: &str) -> String {
    static SCRIPT: OnceLock<Regex> = OnceLock::new();
    static STYLE: OnceLock<Regex> = OnceLock::new();
    static SVG: OnceLock<Regex> = OnceLock::new();
    static HEAD: OnceLock<Regex> = OnceLock::new();
    static COMMENTS: OnceLock<Regex> = OnceLock::new();
    static NOSCRIPT: OnceLock<Regex> = OnceLock::new();
    static IFRAME: OnceLock<Regex> = OnceLock::new();
    static ATTR: OnceLock<Regex> = OnceLock::new();
    static VOID_TAGS: OnceLock<Regex> = OnceLock::new();
    static BLOCK_TAGS: OnceLock<Regex> = OnceLock::new();
    static ALL_TAGS: OnceLock<Regex> = OnceLock::new();
    static BLANK_LINES: OnceLock<Regex> = OnceLock::new();
    static SPACES: OnceLock<Regex> = OnceLock::new();
    static NUM_ENTITY: OnceLock<Regex> = OnceLock::new();

    let mut result = html.to_string();

    // --- Remove tags with their entire content ---
    result = SCRIPT
        .get_or_init(|| re(r"(?is)<script[^>]*>.*?</script>"))
        .replace_all(&result, "")
        .to_string();

    result = STYLE
        .get_or_init(|| re(r"(?is)<style[^>]*>.*?</style>"))
        .replace_all(&result, "")
        .to_string();

    result = SVG
        .get_or_init(|| re(r"(?is)<svg[^>]*>.*?</svg>"))
        .replace_all(&result, "")
        .to_string();

    result = HEAD
        .get_or_init(|| re(r"(?is)<head[^>]*>.*?</head>"))
        .replace_all(&result, "")
        .to_string();

    result = COMMENTS
        .get_or_init(|| re(r"(?s)<!--.*?-->"))
        .replace_all(&result, "")
        .to_string();

    result = NOSCRIPT
        .get_or_init(|| re(r"(?is)<noscript[^>]*>.*?</noscript>"))
        .replace_all(&result, "")
        .to_string();

    result = IFRAME
        .get_or_init(|| re(r"(?is)<iframe[^>]*>.*?</iframe>"))
        .replace_all(&result, "")
        .to_string();

    // --- Strip noisy attributes ---
    result = ATTR
        .get_or_init(|| re(
            r#"(?i)\s*(on\w+|style|class|id|data-[\w-]+|aria-[\w-]+|role|tabindex|xmlns\S*)\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]*)"#,
        ))
        .replace_all(&result, "").to_string();

    // --- Void/self-closing tags → newline (no text content) ---
    result = VOID_TAGS
        .get_or_init(|| re(
            r"(?i)<(img|input|br|hr|meta|link|source|track|wbr|area|base|col|embed|param|picture)\b[^>]*/?>",
        ))
        .replace_all(&result, "\n").to_string();

    // --- Block-level tags → newline so words don't concatenate ---
    // FIX: this was the main cause of "se-grade 24/7 supportPricing" etc.
    result = BLOCK_TAGS
        .get_or_init(|| re(
            r"(?i)</?(?:p|div|section|article|aside|header|footer|main|nav|ul|ol|li|dl|dt|dd|h[1-6]|blockquote|pre|table|thead|tbody|tfoot|tr|th|td|figure|figcaption|details|summary|dialog|form|fieldset|legend)\b[^>]*>",
        ))
        .replace_all(&result, "\n").to_string();

    // --- Strip all remaining tags ---
    result = ALL_TAGS
        .get_or_init(|| re(r"<[^>]+>"))
        .replace_all(&result, "")
        .to_string();

    // --- Decode named HTML entities ---
    let named_entities: &[(&str, &str)] = &[
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&apos;", "'"),
        ("&nbsp;", " "),
        ("&mdash;", "—"),
        ("&ndash;", "–"),
        ("&hellip;", "…"),
        ("&copy;", "©"),
        ("&reg;", "®"),
        ("&trade;", "™"),
        // FIX: these were missing
        ("&middot;", "·"),
        ("&bull;", "•"),
        ("&laquo;", "«"),
        ("&raquo;", "»"),
        ("&lsquo;", "'"),
        ("&rsquo;", "'"),
        ("&ldquo;", "\u{201C}"),
        ("&rdquo;", "\u{201D}"),
        ("&minus;", "−"),
        ("&times;", "×"),
        ("&divide;", "÷"),
        ("&euro;", "€"),
        ("&pound;", "£"),
        ("&yen;", "¥"),
        ("&cent;", "¢"),
        ("&sect;", "§"),
        ("&para;", "¶"),
        ("&dagger;", "†"),
    ];
    for (entity, replacement) in named_entities {
        result = result.replace(entity, replacement);
    }

    // FIX: decode numeric entities &#NNN; and &#xHHH;
    result = NUM_ENTITY
        .get_or_init(|| re(r"&#([xX][0-9a-fA-F]+|[0-9]+);"))
        .replace_all(&result, |caps: &regex::Captures| {
            let s = &caps[1];
            let code_point = if s.starts_with('x') || s.starts_with('X') {
                u32::from_str_radix(&s[1..], 16).ok()
            } else {
                s.parse::<u32>().ok()
            };
            code_point
                .and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_default()
        })
        .to_string();

    // --- Whitespace cleanup ---
    result = BLANK_LINES
        .get_or_init(|| re(r"\n{3,}"))
        .replace_all(&result, "\n\n")
        .to_string();

    result = SPACES
        .get_or_init(|| re(r"[ \t]{2,}"))
        .replace_all(&result, " ")
        .to_string();

    result = result
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() || true) // keep structure; blank lines collapsed above
        .collect::<Vec<_>>()
        .join("\n");

    result.trim().to_string()
}
