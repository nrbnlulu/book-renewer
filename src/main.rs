use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
mod llm;

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
        .unwrap_or_else(|| "/home/dev/Downloads/gliley_zahav.pdf".to_string());

    // dump_pdf_pages(&path)?;

    let book_name = Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("book");

    log::info!("Grouping raw text to episodes for: {}", book_name);
    llm::orgenizer::group_raw_text_to_episodes(book_name, book_name).await?;
    log::info!("Successfully created episode files");

    Ok(())
}

