use std::env;
use std::fs;
use std::io;
use std::path::Path;

use rig::agent::Agent;
use rig::client::CompletionClient;
use rig::completion::Chat;
use rig::providers::gemini::CompletionModel;

mod extract_pdf;
mod llm;

const UTILS_TYP: &str = r#"#let q(body) = {
  set text(weight: "bold", font: "Frank Ruhl Libre", size: 9.5pt)
  body
}
"#;

const MAIN_TYP_BOILERPLATE: &str = r#"#import "utils.typ": q

#set page(
  width: 4.25in, height: 6.87in,
  margin: (inside: 0.6in, outside: 0.4in, top: 0.6in, bottom: 0.4in),
  header: context {
    let p_num = counter(page).get().first()
    let headings = query(selector(heading.where(level: 1)).before(here()))
    let is_new_ep = query(heading.where(level: 1)).any(h => h.location().page() == p_num)
    if headings != () and not is_new_ep {
      let current_episode = headings.last().body
      if calc.even(p_num) { [#p_num #h(1fr) #current_episode] }
      else { [#current_episode #h(1fr) #p_num] }
    }
  }
)

#set text(font: "David Libre", size: 9pt, lang: "he")
#set par(justify: true, first-line-indent: 1.2em, leading: 0.55em)
#show regex("\[.*?\]"): it => { set text(size: 0.75em, fill: gray.darken(50%)); it }
#show heading.where(level: 1): it => {
  pagebreak(weak: true, to: "odd")
  set align(center); set text(size: 14pt); block(above: 12%, below: 0.6in, it.body)
}
#show heading.where(level: 2): it => {
  set align(right); set text(size: 10pt, weight: "bold"); block(above: 1.2em, below: 0.6em, it.body)
}

"#;

const CHAPTER_IMPORT: &str = "#import \"../utils.typ\": q\n\n";

// One episode = one output .typ file, potentially spanning many PDF pages.
struct Episode {
    name: String,
    content: String,
    pages: Vec<usize>,
    index: usize,
}

impl Episode {
    fn new(index: usize, name: String) -> Self {
        Self { name, content: String::new(), pages: Vec::new(), index }
    }

    fn file_name(&self) -> String {
        format!("episode_{:03}.typ", self.index)
    }

    fn append_page(&mut self, page: usize, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if !self.pages.contains(&page) {
            self.pages.push(page);
        }
        if !self.content.is_empty() {
            self.content.push_str("\n\n");
        }
        self.content.push_str(text);
    }

    fn write(&self, dir: &Path) -> io::Result<()> {
        let pages_str = self.pages
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        fs::write(
            dir.join(self.file_name()),
            format!("{CHAPTER_IMPORT}// pdf-pages: {pages_str}\n\n{}", self.content),
        )
    }
}

fn strip_markdown_fence(content: String) -> String {
    let content = content.trim().to_string();
    if let Some(stripped) = content.strip_prefix("```typst") {
        return stripped.strip_suffix("```").unwrap_or(stripped).trim().to_string();
    }
    if let Some(start) = content.find("```typst") {
        let inner = &content[start + 8..];
        return if let Some(end) = inner.find("```") {
            inner[..end].trim().to_string()
        } else {
            inner.trim().to_string()
        };
    }
    content
}

fn parse_first_h1(content: &str) -> Option<String> {
    content
        .lines()
        .find(|l| l.starts_with("= "))
        .map(|l| l[2..].trim().to_string())
}

fn build_format_prompt(raw_text: &str, episode_name: Option<&str>, episode_typst: &str) -> String {
    match episode_name {
        Some(name) => format!(
            "You are currently mid-episode ('{name}'). \
Here is the Typst content accumulated so far for this episode:\n\n\
```typst\n{episode_typst}\n```\n\n\
Here is the raw text from the next page. \
Determine whether it continues the current episode or begins a new one, \
and format accordingly per your instructions. \
If the page is blank or irrelevant, output only `// SKIP`.\n\nRaw text:\n\n{raw_text}"
        ),
        None => format!(
            "Here is raw text extracted from a book. \
Please format it as Typst code according to the style guide in your instructions. \
Do not include any other commentary, only the Typst code.\n\n{raw_text}"
        ),
    }
}

// Integrates one page's Typst fragment into the running episode state.
// Handles END_EPISODE splits (including mid-page episode boundaries) recursively.
fn integrate_page(
    page: usize,
    content: &str,
    counter: &mut usize,
    current: &mut Option<Episode>,
    episode_files: &mut Vec<String>,
    typst_dir: &Path,
) -> io::Result<()> {
    const END_MARKER: &str = "// END_EPISODE<";

    if let Some(end_pos) = content.find(END_MARKER) {
        let before = content[..end_pos].trim();
        let line_end = content[end_pos..]
            .find('\n')
            .map(|n| end_pos + n + 1)
            .unwrap_or(content.len());
        let after = content[line_end..].trim();

        if !before.is_empty() {
            if current.is_none() {
                let name = parse_first_h1(before)
                    .unwrap_or_else(|| format!("פרק {page}"));
                *counter += 1;
                *current = Some(Episode::new(*counter, name));
            }
            current.as_mut().unwrap().append_page(page, before);
        }

        if let Some(ep) = current.take() {
            log::info!("Completed episode '{}' (pages: {:?})", ep.name, ep.pages);
            let fname = ep.file_name();
            ep.write(typst_dir)?;
            episode_files.push(fname);
        }

        if !after.is_empty() {
            integrate_page(page, after, counter, current, episode_files, typst_dir)?;
        }
    } else {
        let h1 = parse_first_h1(content);
        let is_new = match (&h1, current.as_ref()) {
            (Some(h), Some(ep)) => ep.name != *h,
            (Some(_), None) => true,
            (None, _) => false,
        };

        if is_new {
            if let Some(ep) = current.take() {
                log::info!("Completed episode '{}' (pages: {:?})", ep.name, ep.pages);
                let fname = ep.file_name();
                ep.write(typst_dir)?;
                episode_files.push(fname);
            }
            *counter += 1;
            let mut ep = Episode::new(*counter, h1.unwrap());
            ep.append_page(page, content.trim());
            *current = Some(ep);
        } else if let Some(ep) = current.as_mut() {
            ep.append_page(page, content.trim());
        } else {
            log::warn!("Page {page}: content has no heading and no current episode — skipping");
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "/home/dev/Downloads/resisey_layla.pdf".to_string());

    let book_name = Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("book");

    let base_dir_str = format!("renewed-{book_name}");
    let base_dir = Path::new(&base_dir_str);
    let typst_dir = base_dir.join("typst");
    let pages_dir = typst_dir.join("pages");
    fs::create_dir_all(&pages_dir)?;

    let client = llm::gemini_client();
    let agent: Agent<CompletionModel> = client
        .agent("gemini-3-flash-preview")
        .preamble(include_str!("llm/typst_preamble.md"))
        .temperature(0.3)
        .build();

    let output = std::process::Command::new("pdfinfo")
        .arg(&path)
        .output()
        .map_err(|e| format!("Failed to run pdfinfo: {e}"))?;

    let info = String::from_utf8_lossy(&output.stdout);
    let page_count = info
        .lines()
        .find(|l| l.starts_with("Pages:"))
        .and_then(|l| l.split_whitespace().last())
        .and_then(|p| p.parse::<usize>().ok())
        .unwrap_or(0);

    log::info!("Processing PDF: {} ({} pages)", path, page_count);

    let mut episode_counter = 0usize;
    let mut current_episode: Option<Episode> = None;
    let mut episode_files: Vec<String> = Vec::new();

    for page in 1..=page_count {
        let page_cache = pages_dir.join(format!("{page}.typ"));

        let page_content = if page_cache.exists() {
            log::info!("Page {} loaded from cache", page);
            fs::read_to_string(&page_cache)?
        } else {
            log::info!("Extracting page {}", page);
            let raw = match extract_pdf::extract_page_content(&path, page, &agent).await {
                Ok(t) => t,
                Err(e) => {
                    log::error!("Page {page}: extraction failed: {e}");
                    continue;
                }
            };

            let ep = current_episode.as_ref();
            let prompt = build_format_prompt(
                &raw,
                ep.map(|e| e.name.as_str()),
                ep.map(|e| e.content.as_str()).unwrap_or(""),
            );

            let content = match agent.chat(&prompt, vec![]).await {
                Ok(c) => strip_markdown_fence(c),
                Err(e) => {
                    log::error!("Page {page}: formatting failed: {e}");
                    continue;
                }
            };

            fs::write(&page_cache, &content)?;
            content
        };

        if page_content.trim().starts_with("// SKIP") {
            log::info!("Page {} skipped", page);
            continue;
        }

        integrate_page(
            page,
            &page_content,
            &mut episode_counter,
            &mut current_episode,
            &mut episode_files,
            &typst_dir,
        )?;
    }

    // Flush the final episode (no END_EPISODE on last page is normal)
    if let Some(ep) = current_episode.take() {
        log::info!("Completed final episode '{}' (pages: {:?})", ep.name, ep.pages);
        let fname = ep.file_name();
        ep.write(&typst_dir)?;
        episode_files.push(fname);
    }

    let utils_path = base_dir.join("utils.typ");
    if !utils_path.exists() {
        fs::write(&utils_path, UTILS_TYP)?;
        log::info!("Generated utils.typ");
    }

    let includes = episode_files
        .iter()
        .map(|f| format!("#include \"typst/{f}\""))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        base_dir.join("main.typ"),
        format!("{MAIN_TYP_BOILERPLATE}{includes}\n"),
    )?;
    log::info!("Done. {} episodes written.", episode_files.len());

    Ok(())
}
