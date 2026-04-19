use anyhow::anyhow;
use rig::{
    agent::Agent,
    client::CompletionClient,
    completion::Prompt,
    providers::gemini::{self, CompletionModel},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use super::orgenizer::Episode;

/// Shared helper functions exported to chapter files via `utils.typ`.
const UTILS_TYP: &str = r#"#let q(body) = {
  set text(weight: "bold", font: "Frank Ruhl Libre", size: 9.5pt)
  body
}
"#;

/// The import line prepended to every generated chapter file.
const CHAPTER_IMPORT: &str = "#import \"../utils.typ\": q\n\n";

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

/// Boilerplate used only for validation — inlines `q` so no file path resolution needed.
const VALIDATION_BOILERPLATE: &str = r#"#let q(body) = {
  set text(weight: "bold", font: "Frank Ruhl Libre", size: 9.5pt)
  body
}

#set text(lang: "he")

"#;

fn create_typst_agent(client: &gemini::client::Client) -> Agent<CompletionModel> {
    let preamble = include_str!("typst_preamble.md");
    client
        .agent("gemini-3-flash-preview")
        .preamble(preamble)
        .temperature(0.3)
        .build()
}

fn strip_typst_fences(response: &str) -> String {
    let response = response.trim();
    if let Some(stripped) = response.strip_prefix("```typst") {
        return stripped
            .strip_suffix("```")
            .unwrap_or(stripped)
            .trim()
            .to_string();
    }
    if let Some(start) = response.find("```typst") {
        let inner = &response[start + 8..];
        return if let Some(end) = inner.find("```") {
            inner[..end].trim().to_string()
        } else {
            inner.trim().to_string()
        };
    }
    response.to_string()
}

/// Returns `None` if valid, or `Some(error_text)` if compilation failed.
fn validate_typst(chapter_content: &str) -> anyhow::Result<Option<String>> {
    let full = format!("{VALIDATION_BOILERPLATE}\n{chapter_content}");

    let tmp_path = std::env::temp_dir().join("book_renewer_validate.typ");
    fs::write(&tmp_path, &full)?;

    let output = Command::new("typst")
        .args(["compile", "--format", "pdf", tmp_path.to_str().unwrap(), "/dev/null"])
        .output();

    match output {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log::warn!("typst not found in PATH — skipping validation");
            Ok(None)
        }
        Err(e) => Err(anyhow!("failed to run typst: {e}")),
        Ok(out) if out.status.success() => Ok(None),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let error = if stderr.is_empty() { stdout } else { stderr };
            Ok(Some(error))
        }
    }
}

fn build_initial_prompt(episode: &Episode) -> String {
    if episode.sub_headers.is_empty() {
        format!("שם הפרק: {}\nתוכן:\n{}", episode.name, episode.content)
    } else {
        format!(
            "שם הפרק: {}\nנושאים משניים: {}\nתוכן:\n{}",
            episode.name,
            episode.sub_headers.join(", "),
            episode.content
        )
    }
}

fn build_retry_prompt(episode: &Episode, previous_output: &str, compile_error: &str) -> String {
    format!(
        "{}\n\n---\nהפלט הקודם שלך היה:\n```typst\n{}\n```\n\nנכשל בקומפילציה עם השגיאה הבאה:\n{}\n\nאנא תקן את השגיאה וייצר שוב את הקובץ.",
        build_initial_prompt(episode),
        previous_output,
        compile_error
    )
}

async fn generate_validated_episode(
    agent: &Agent<CompletionModel>,
    episode: &Episode,
    idx: usize,
) -> anyhow::Result<String> {
    let mut prompt = build_initial_prompt(episode);

    for attempt in 0..5 {
        let response = agent
            .prompt(&prompt)
            .await
            .map_err(|e| anyhow!("LLM call failed on episode {idx} attempt {attempt}: {e}"))?;

        let content = strip_typst_fences(&response);

        match validate_typst(&content)? {
            None => {
                log::debug!("Episode {idx} passed validation on attempt {attempt}");
                return Ok(content);
            }
            Some(error) => {
                log::warn!(
                    "Episode {idx} attempt {attempt} failed typst validation:\n{error}"
                );
                if attempt < 4 {
                    prompt = build_retry_prompt(episode, &content, &error);
                }
            }
        }
    }

    Err(anyhow!(
        "Episode {idx} failed typst validation after 5 attempts"
    ))
}

fn sorted_json_episode_files(jsoned_dir: &Path) -> anyhow::Result<Vec<(usize, PathBuf)>> {
    let mut files: Vec<(usize, PathBuf)> = fs::read_dir(jsoned_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map_or(false, |ext| ext == "json")
                && e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map_or(false, |n| n != "episode_status.json")
        })
        .filter_map(|e| {
            let path = e.path();
            let idx = path.file_stem()?.to_str()?.parse::<usize>().ok()?;
            Some((idx, path))
        })
        .collect();
    files.sort_by_key(|(idx, _)| *idx);
    Ok(files)
}

pub async fn convert_json_episodes_to_typst(book_name: &str) -> anyhow::Result<()> {
    let base_dir_str = format!("renewed-{book_name}");
    let base_dir = Path::new(&base_dir_str);
    let jsoned_dir = base_dir.join("jsoned");
    let typst_dir = base_dir.join("typst");
    fs::create_dir_all(&typst_dir)?;

    let client = crate::llm::gemini_client();
    let agent = create_typst_agent(&client);

    let episode_files = sorted_json_episode_files(&jsoned_dir)?;
    log::info!("Found {} episodes to convert to typst", episode_files.len());

    let mut include_lines: Vec<String> = Vec::new();

    for (idx, json_path) in &episode_files {
        let typst_path = typst_dir.join(format!("{idx}.typ"));

        if typst_path.exists() {
            log::info!("Skipping already converted episode {idx}");
            include_lines.push(format!("#include \"typst/{idx}.typ\""));
            continue;
        }

        let json_content = fs::read_to_string(json_path)?;
        let episode: Episode = serde_json::from_str(&json_content)?;

        let content = generate_validated_episode(&agent, &episode, *idx).await?;
        fs::write(&typst_path, format!("{CHAPTER_IMPORT}{content}"))?;
        log::info!("Converted episode {idx}: {}", episode.name);
        include_lines.push(format!("#include \"typst/{idx}.typ\""));
    }

    write_main_typ(base_dir, &include_lines)?;
    Ok(())
}

fn write_main_typ(base_dir: &Path, include_lines: &[String]) -> anyhow::Result<()> {
    let utils_path = base_dir.join("utils.typ");
    if !utils_path.exists() {
        fs::write(&utils_path, UTILS_TYP)?;
        log::info!("Generated utils.typ");
    }

    let main_typ_path = base_dir.join("main.typ");
    if main_typ_path.exists() {
        log::info!("main.typ already exists, skipping generation");
        return Ok(());
    }
    let includes = include_lines.join("\n");
    let content = format!("{MAIN_TYP_BOILERPLATE}{includes}\n");
    fs::write(&main_typ_path, content)?;
    log::info!("Generated main.typ with {} includes", include_lines.len());
    Ok(())
}
