use anyhow::{Context, Result};
use rig::OneOrMany;
use rig::completion::Chat;
use rig::completion::Message;
use rig::message::{ImageDetail, ImageMediaType, UserContent};
use std::fs;
use std::path::Path;
use std::process::Command;

pub async fn extract_page_content(
    pdf_path: &str,
    page: usize,
    agent: &rig::agent::Agent<rig::providers::gemini::CompletionModel>,
) -> Result<String> {
    let pdf_name = Path::new(pdf_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let temp_dir = std::env::temp_dir().join(format!("book_renewer_{}_{}", pdf_name, page));
    fs::create_dir_all(&temp_dir)?;

    let out_prefix = temp_dir.join("page");

    // 1. Render the specific page to a PNG using pdftocairo
    // -f and -l specify the first and last page to render (same page here)
    let status = Command::new("pdftocairo")
        .args([
            "-png",
            "-f",
            &page.to_string(),
            "-l",
            &page.to_string(),
            pdf_path,
            out_prefix.to_str().unwrap(),
        ])
        .status()
        .context("Failed to run pdftocairo")?;

    if !status.success() {
        return Err(anyhow::anyhow!("pdftocairo failed with status: {}", status));
    }

    // pdftocairo appends "-1.png" (or similar) to the prefix
    let png_path = fs::read_dir(&temp_dir)?
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("png"))
        .map(|e| e.path())
        .context("Could not find generated PNG file")?;

    // 2. Read the image and encode to base64
    use base64::{Engine as _, engine::general_purpose};
    let image_data = fs::read(&png_path).context("Failed to read rendered PNG")?;
    let base64_image = general_purpose::STANDARD.encode(&image_data);

    // 3. Send to Gemini with instructions
    const ALEF_LAMED_PNG: &[u8] = include_bytes!("../alef-lamed.png");
    let alef_lamed_b64 = general_purpose::STANDARD.encode(ALEF_LAMED_PNG);

    let prompt_text = UserContent::text(
        "Please extract all the raw text from the image, only the hebrew text \
יש עברית מרוקאית ועברית רגילה
שניהים מכילים אותו תוכן אבל אני רוצה שתוציא רק את העברית עברית.

אל תשנה שום מילה. שים לב שהמחבר כותב 'אל' מחובר — כלומר האות א' ואות ל' מחוברות זו לזו. \
בתמונה הראשונה תראה דוגמה: הטקסט שם הוא \"אלה וכאלה\", שים לב לצורת החיבור של א\"ל. \
כשתראה צורה כזו בטקסט, כתוב אותה כרגיל עם אלף ולמד.

- לפעמים המחבר משתמש ב `  או  '
כדי להמנע מכתיבת שמות קודש (נגיד אחרי י או ה) שים לב לזה ותתעלם מהניקודים הללו

- שים לב שלפעמים יש מילה אחרונה בעמוד שהיא בעצם שייכת לעמוד הבא (אופייני לטקסטים תורניים) אז אל תכלול אותה
בתוצאה. היא בדכ תהיה בindentation שונה מהשאר.
",
    );
    let example_image = UserContent::image_base64(
        alef_lamed_b64,
        Some(ImageMediaType::PNG),
        Some(ImageDetail::High),
    );
    let prompt_image = UserContent::image_base64(
        base64_image,
        Some(ImageMediaType::PNG),
        Some(ImageDetail::High),
    );

    let message = Message::User {
        content: OneOrMany::many(vec![prompt_text, example_image, prompt_image]).unwrap(),
    };

    // Use chat to get the response
    let response = agent.chat(message, vec![]).await?;

    // Cleanup
    let _ = fs::remove_dir_all(temp_dir);

    Ok(response)
}
