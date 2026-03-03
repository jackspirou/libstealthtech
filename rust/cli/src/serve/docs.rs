//! Documentation page generation from embedded Markdown files.
//!
//! Embeds the `docs/` directory at compile time and converts each Markdown
//! file to a styled HTML page during `stealthtech export`.

use pulldown_cmark::{html, Options, Parser};
use rust_embed::Embed;

/// Embedded documentation Markdown files from the repository root `docs/` directory.
#[derive(Embed)]
#[folder = "../../docs/"]
struct DocFiles;

/// Metadata for a single documentation page.
struct DocMeta {
    slug: &'static str,
    title: &'static str,
    description: &'static str,
}

/// Ordered registry of all documentation pages.
const DOC_PAGES: &[DocMeta] = &[
    DocMeta {
        slug: "protocol-mapping",
        title: "Protocol Mapping",
        description: "Complete BLE protocol specification — packet formats, command encoding tables, and notification protocol.",
    },
    DocMeta {
        slug: "architecture",
        title: "Architecture",
        description: "Crate dependency graph, BLE state machine, GATT discovery flow, and characteristic map.",
    },
    DocMeta {
        slug: "reverse-engineering",
        title: "Reverse Engineering Guide",
        description: "RE methodology, GATT enumeration, traffic capture, tools, and security considerations.",
    },
    DocMeta {
        slug: "firmware-analysis",
        title: "Firmware Analysis",
        description: "MCU binary string analysis, AT command discovery, and hardware architecture from firmware.",
    },
    DocMeta {
        slug: "hardware-teardown",
        title: "Hardware Teardown",
        description: "FCC filings, patents, component supply chain, and WiSA wireless architecture.",
    },
];

/// Convert Markdown text to HTML.
fn markdown_to_html(markdown: &str) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_HEADING_ATTRIBUTES;
    let parser = Parser::new_ext(markdown, options);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

/// Rewrite inter-doc `.md` links to `.html` in rendered HTML.
fn rewrite_doc_links(html: &str) -> String {
    let mut result = html.to_string();
    for meta in DOC_PAGES {
        let md_link = format!("{}.md", meta.slug);
        let html_link = format!("{}.html", meta.slug);
        result = result.replace(&md_link, &html_link);
    }
    result
}

/// Wrap rendered HTML body in a complete document with styling and navigation.
fn doc_page_html(slug: &str, title: &str, description: &str, body: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} — StealthTech Docs</title>
    <meta name="description" content="{description}">
    <link rel="canonical" href="https://stealthtech.app/docs/{slug}.html">
    <meta property="og:title" content="{title} — StealthTech Docs">
    <meta property="og:description" content="{description}">
    <meta property="og:type" content="article">
    <meta property="og:url" content="https://stealthtech.app/docs/{slug}.html">
    <meta property="og:site_name" content="StealthTech">
    <link rel="stylesheet" href="../style.css">
    <link rel="icon" href="../favicon.svg" type="image/svg+xml">
    <meta name="theme-color" content="#0071e3" media="(prefers-color-scheme: light)">
    <meta name="theme-color" content="#0a84ff" media="(prefers-color-scheme: dark)">
</head>
<body>
    <header>
        <div class="header-left">
            <a href="/" class="header-home">StealthTech</a>
            <span class="subtitle">Docs</span>
        </div>
        <div class="header-right">
            <a class="docs-link" href="/docs/" title="All documentation">Index</a>
            <a class="github-link" href="https://github.com/jackspirou/libstealthtech" target="_blank" rel="noopener" title="View source on GitHub" aria-label="View source on GitHub">
                <svg width="20" height="20" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8z"/></svg>
            </a>
            <button class="header-icon-btn" id="theme-toggle" title="Toggle theme" aria-label="Toggle theme">
                <span id="theme-icon">&#9788;</span>
            </button>
        </div>
    </header>
    <main class="doc-content">
        <article>
{body}
        </article>
    </main>
    <script>
    (function() {{
        var K = "stealthtech-theme", T = ["auto","light","dark"];
        var i = Math.max(0, T.indexOf(localStorage.getItem(K) || "auto"));
        function a() {{
            var t = T[i];
            if (t === "auto") document.documentElement.removeAttribute("data-theme");
            else document.documentElement.setAttribute("data-theme", t);
            document.getElementById("theme-icon").textContent = t === "auto" ? "\u2699" : t === "light" ? "\u2600" : "\u263E";
            localStorage.setItem(K, t);
        }}
        a();
        document.getElementById("theme-toggle").addEventListener("click", function() {{ i = (i + 1) % T.length; a(); }});
    }})();
    </script>
</body>
</html>"##,
        title = title,
        description = description,
        slug = slug,
        body = body,
    )
}

/// Render all embedded Markdown docs to HTML pages.
///
/// Returns a list of `(filename, html_content)` pairs ready to write to disk.
pub fn render_all() -> Vec<(String, String)> {
    let mut pages = Vec::new();
    for meta in DOC_PAGES {
        let md_filename = format!("{}.md", meta.slug);
        let Some(file) = DocFiles::get(&md_filename) else {
            eprintln!(
                "  warning: docs/{} not found in embedded files",
                md_filename
            );
            continue;
        };
        let markdown = String::from_utf8_lossy(&file.data);
        let body = markdown_to_html(&markdown);
        let body = rewrite_doc_links(&body);
        let html = doc_page_html(meta.slug, meta.title, meta.description, &body);
        pages.push((format!("{}.html", meta.slug), html));
    }
    pages
}

/// Generate the documentation index page.
pub fn render_index() -> String {
    let mut cards = String::new();
    for meta in DOC_PAGES {
        cards.push_str(&format!(
            r#"        <a class="doc-index-card" href="{slug}.html">
            <h3>{title}</h3>
            <p>{description}</p>
        </a>
"#,
            slug = meta.slug,
            title = meta.title,
            description = meta.description,
        ));
    }

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Documentation — StealthTech</title>
    <meta name="description" content="Technical documentation for the StealthTech Sound + Charge open-source project — BLE protocol, architecture, firmware analysis, and more.">
    <link rel="canonical" href="https://stealthtech.app/docs/">
    <meta property="og:title" content="Documentation — StealthTech">
    <meta property="og:description" content="Technical documentation for the StealthTech Sound + Charge open-source project.">
    <meta property="og:type" content="website">
    <meta property="og:url" content="https://stealthtech.app/docs/">
    <meta property="og:site_name" content="StealthTech">
    <link rel="stylesheet" href="../style.css">
    <link rel="icon" href="../favicon.svg" type="image/svg+xml">
    <meta name="theme-color" content="#0071e3" media="(prefers-color-scheme: light)">
    <meta name="theme-color" content="#0a84ff" media="(prefers-color-scheme: dark)">
</head>
<body>
    <header>
        <div class="header-left">
            <a href="/" class="header-home">StealthTech</a>
            <span class="subtitle">Docs</span>
        </div>
        <div class="header-right">
            <a class="github-link" href="https://github.com/jackspirou/libstealthtech" target="_blank" rel="noopener" title="View source on GitHub" aria-label="View source on GitHub">
                <svg width="20" height="20" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8z"/></svg>
            </a>
            <button class="header-icon-btn" id="theme-toggle" title="Toggle theme" aria-label="Toggle theme">
                <span id="theme-icon">&#9788;</span>
            </button>
        </div>
    </header>
    <main class="doc-content">
        <h1>Documentation</h1>
        <p>Technical documentation for the Lovesac Sactionals StealthTech Sound + Charge open-source project. Learn how the BLE protocol works, explore the Rust implementation, and contribute to reverse engineering efforts.</p>
        <div class="doc-index">
{cards}        </div>
    </main>
    <script>
    (function() {{
        var K = "stealthtech-theme", T = ["auto","light","dark"];
        var i = Math.max(0, T.indexOf(localStorage.getItem(K) || "auto"));
        function a() {{
            var t = T[i];
            if (t === "auto") document.documentElement.removeAttribute("data-theme");
            else document.documentElement.setAttribute("data-theme", t);
            document.getElementById("theme-icon").textContent = t === "auto" ? "\u2699" : t === "light" ? "\u2600" : "\u263E";
            localStorage.setItem(K, t);
        }}
        a();
        document.getElementById("theme-toggle").addEventListener("click", function() {{ i = (i + 1) % T.length; a(); }});
    }})();
    </script>
</body>
</html>"##,
        cards = cards,
    )
}

/// Generate a sitemap.xml that includes the app and all documentation pages.
pub fn generate_sitemap() -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  \
         <url>\n    <loc>https://stealthtech.app/</loc>\n    \
         <changefreq>monthly</changefreq>\n    \
         <priority>1.0</priority>\n  \
         </url>\n  \
         <url>\n    <loc>https://stealthtech.app/docs/</loc>\n    \
         <changefreq>monthly</changefreq>\n    \
         <priority>0.8</priority>\n  \
         </url>\n",
    );
    for meta in DOC_PAGES {
        xml.push_str(&format!(
            "  <url>\n    \
             <loc>https://stealthtech.app/docs/{slug}.html</loc>\n    \
             <changefreq>monthly</changefreq>\n    \
             <priority>0.6</priority>\n  \
             </url>\n",
            slug = meta.slug,
        ));
    }
    xml.push_str("</urlset>\n");
    xml
}
