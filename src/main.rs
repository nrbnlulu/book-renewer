use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
mod llm;

async fn repair_missing_episodes(book_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let jsoned_dir = format!("renewed-{book_name}/jsoned");
    let status_path = format!("{jsoned_dir}/episode_status.json");

    let content = match fs::read_to_string(&status_path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };
    let cache: HashMap<String, llm::orgenizer::EpisodeStatusEntry> =
        serde_json::from_str(&content)?;

    for (name, entry) in &cache {
        let json_path = format!("{jsoned_dir}/{}.json", entry.index);
        if Path::new(&json_path).exists() {
            continue;
        }
        log::warn!(
            "Missing episode JSON for '{}' at index {} — regenerating",
            name,
            entry.index
        );
        llm::orgenizer::regenerate_episode_from_pages(
            book_name,
            book_name,
            name,
            &entry.pages,
            entry.index,
        )
        .await?;
    }
    Ok(())
}

fn dump_pdf_pages(pdf_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Dumping PDF pages from: {}", pdf_path);

    let output = Command::new("pdftotext")
        .args(["-layout", "-enc", "UTF-8", pdf_path, "-"])
        .output()
        .map_err(|e| format!("Failed to run pdftotext (is poppler-utils installed?): {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftotext failed: {stderr}").into());
    }

    let text = String::from_utf8_lossy(&output.stdout);

    let pdf_name = Path::new(pdf_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let output_dir = Path::new(pdf_name);
    fs::create_dir_all(output_dir)?;

    let mut page_count = 0;
    for (i, page) in text.split('\x0C').enumerate() {
        if page.trim().is_empty() {
            continue;
        }

        let page_path = output_dir.join(format!("page_{}.txt", i + 1));
        fs::write(&page_path, page.trim_end())?;
        page_count += 1;
    }

    log::info!("Dumped {} pages to {:?}", page_count, output_dir);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "/home/dev/Downloads/resisey_layla.pdf".to_string());

    // dump_pdf_pages(&path)?;

    let book_name = Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("book");

    log::info!("Grouping raw text to episodes for: {}", book_name);
    llm::orgenizer::group_raw_text_to_episodes(book_name, book_name).await?;
    log::info!("Successfully created episode files");

    log::info!("Checking for missing episode JSON files");
    repair_missing_episodes(book_name).await?;

    log::info!("Converting JSON episodes to Typst for: {}", book_name);
    llm::json_to_typst::convert_json_episodes_to_typst(book_name).await?;
    log::info!("Successfully converted episodes to Typst");

    Ok(())
}

