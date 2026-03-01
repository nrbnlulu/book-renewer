use anyhow::anyhow;
use rig::{
    agent::Agent,
    client::CompletionClient,
    completion::Prompt,
    providers::gemini::{self, CompletionModel},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EpisodeStatusKind {
    Completed,
    InProgress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeStatusEntry {
    pub episode_name: String,
    pub status: EpisodeStatusKind,
    pub index: usize,
    pub pages: Vec<String>,
}

type StatusCache = HashMap<String, EpisodeStatusEntry>;

fn load_status_cache(path: &Path) -> StatusCache {
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            log::warn!("Failed to parse episode status cache: {e}");
            HashMap::new()
        }),
        Err(_) => HashMap::new(),
    }
}

fn save_status_cache(path: &Path, cache: &StatusCache) -> anyhow::Result<()> {
    let content = serde_json::to_string_pretty(cache)?;
    fs::write(path, content)?;
    Ok(())
}

fn is_completed(cache: &StatusCache, name: &str) -> bool {
    cache
        .get(name)
        .map_or(false, |e| e.status == EpisodeStatusKind::Completed)
}

fn mark_completed(cache: &mut StatusCache, name: &str, index: usize, pages: Vec<String>) {
    cache
        .entry(name.to_string())
        .and_modify(|e| {
            e.status = EpisodeStatusKind::Completed;
            e.index = index;
            e.pages = pages.clone();
        })
        .or_insert(EpisodeStatusEntry {
            episode_name: name.to_string(),
            status: EpisodeStatusKind::Completed,
            index,
            pages,
        });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub name: String,
    pub content: String,
    pub sub_headers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonOrganizerInput {
    pub current_episode: Option<Episode>,
    pub new_raw_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EpisodeAction {
    AppendLastEpisode {
        name: String,
        data: String,
        new_topics: Vec<String>,
    },
    NewEpisode {
        name: String,
        data: String,
        new_topics: Vec<String>,
    },
}

fn strip_fences(response: &str) -> anyhow::Result<String> {
    let response = response.trim();
    if let Some(stripped) = response.strip_prefix("```json") {
        let result = stripped
            .strip_suffix("```")
            .unwrap_or(stripped)
            .trim()
            .to_string();
        log::debug!("Stripped code fences successfully\n {result}");
        Ok(result)
    } else if let Some(start) = response.find("```json") {
        let inner = &response[start + 7..];
        let result = if let Some(end) = inner.find("```") {
            inner[..end].trim().to_string()
        } else {
            inner.trim().to_string()
        };
        log::debug!("Stripped code fences successfully (inline)");
        Ok(result)
    } else {
        log::warn!("Response does not contain ```json code fences");
        anyhow::bail!(
            "Response does not contain ```json code fences: {}",
            response
        );
    }
}

/// Fix common JSON escaping issues in LLM output
/// Converts literal newlines/tabs within strings to escaped sequences
fn fix_json_escaping(json: &str) -> String {
    let mut result = String::with_capacity(json.len());
    let mut in_string = false;
    let mut escape_next = false;

    for ch in json.chars() {
        if escape_next {
            result.push(ch);
            escape_next = false;
            continue;
        }

        if ch == '\\' && in_string {
            escape_next = true;
            result.push(ch);
            continue;
        }

        if ch == '"' {
            in_string = !in_string;
            result.push(ch);
            continue;
        }

        if in_string {
            // Escape literal newlines and tabs within strings
            match ch {
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                _ => result.push(ch),
            }
        } else {
            result.push(ch);
        }
    }

    result
}

async fn get_actions_from_prompt(
    agent: &Agent<CompletionModel>,
    input_serialized: &str,
    page_path: &Path,
) -> anyhow::Result<Vec<EpisodeAction>> {
    let response = agent.prompt(input_serialized).await?;
    let stripped = strip_fences(&response)?;
    let fixed = fix_json_escaping(&stripped);
    serde_json::from_str::<Vec<EpisodeAction>>(&fixed)
        .map_err(|e| anyhow!("llm json output parse error: {e}\n page_path: {page_path:?}"))
}

fn write_episode(output_dir: &Path, counter: usize, episode: &Episode) -> anyhow::Result<()> {
    let json_content = serde_json::to_string_pretty(episode)?;
    let episode_path = output_dir.join(format!("{counter}.json"));
    fs::write(&episode_path, json_content)?;
    log::debug!("Wrote episode {} to {:?}", counter, episode_path);
    Ok(())
}

fn create_llm_agent(preamble: &str, client: &gemini::client::Client) -> Agent<CompletionModel> {
    client
        .agent("gemini-2.5-flash")
        .preamble(preamble)
        .temperature(0.5)
        .build()
}

pub fn create_toon_orgenizer_agent(client: &gemini::client::Client) -> Agent<CompletionModel> {
    let preamble = include_str!("orgenizer_preamble.md");
    create_llm_agent(preamble, client)
}

fn raw_pages_files(input_dir: &str) -> anyhow::Result<Vec<PathBuf>> {
    // Collect and sort page files
    let mut page_files: Vec<_> = fs::read_dir(input_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension().map_or(false, |ext| ext == "txt")
                && entry
                    .path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map_or(false, |n| n.starts_with("page_"))
        })
        .map(|entry| entry.path())
        .collect();

    page_files.sort_by_key(|path| {
        path.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix("page_"))
            .and_then(|n| n.strip_suffix(".txt"))
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap()
    });
    Ok(page_files)
}

pub async fn group_raw_text_to_episodes(input_dir: &str, book_name: &str) -> anyhow::Result<()> {
    log::info!("Starting episode grouping from: {}", input_dir);
    let page_files = raw_pages_files(input_dir)?;

    let client = crate::llm::gemini_client();
    let agent = create_toon_orgenizer_agent(&client);

    // Create output directory: renewed-<book_name>/jsoned/
    let output_dir = Path::new(&format!("renewed-{book_name}")).join("jsoned");
    fs::create_dir_all(&output_dir)?;
    log::info!("Output directory: {:?}", output_dir);

    let status_path = output_dir.join("episode_status.json");
    let mut status_cache = load_status_cache(&status_path);
    log::info!("Loaded {} episode status entries", status_cache.len());

    log::info!("Found {} page files to process", page_files.len());

    let mut current_episode: Option<Episode> = None;
    // Resume counter from highest completed index in the cache
    let mut episode_counter: usize = status_cache
        .values()
        .filter(|e| e.status == EpisodeStatusKind::Completed)
        .map(|e| e.index)
        .max()
        .unwrap_or(0);
    // true when current_episode is one that was already completed — skip writing it again
    let mut current_episode_already_completed = false;
    let mut current_episode_pages: Vec<String> = Vec::new();

    for (idx, page_path) in page_files.iter().enumerate() {
        log::debug!(
            "Processing page {}/{}: {:?}",
            idx + 1,
            page_files.len(),
            page_path
        );

        let page_name = page_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let new_raw_text = fs::read_to_string(page_path)?;

        let input = JsonOrganizerInput {
            current_episode: current_episode.clone(),
            new_raw_text,
        };

        let input_serialized = serde_json::to_string(&input)?;

        let mut actions: anyhow::Result<Vec<EpisodeAction>> =
            get_actions_from_prompt(&agent, &input_serialized, page_path).await;
        for _ in 0..5 {
            if let Err(last_err) = actions {
                log::warn!(
                    "failed to get actions for page: {:?}\n {:?}",
                    page_path,
                    last_err
                );
            } else {
                break;
            }
            actions = get_actions_from_prompt(&agent, &input_serialized, page_path).await;
        }
        let actions = actions?;

        log::debug!("Received {} actions from agent", actions.len());

        for action in actions {
            match action {
                EpisodeAction::AppendLastEpisode {
                    name: _,
                    data,
                    new_topics,
                } => {
                    if current_episode_already_completed {
                        log::trace!("Skipping append to already-completed episode");
                        continue;
                    }
                    let episode = current_episode.as_mut().ok_or(anyhow!(
                        "AppendLastEpisode called but no current episode exists"
                    ))?;
                    if !current_episode_pages.contains(&page_name) {
                        current_episode_pages.push(page_name.clone());
                    }
                    episode.content.push_str(&data);
                    episode.sub_headers.extend(new_topics);
                }
                EpisodeAction::NewEpisode {
                    name,
                    data,
                    new_topics,
                } => {
                    // Flush the previous episode if it wasn't already written
                    if let Some(episode) = current_episode.take() {
                        if !current_episode_already_completed {
                            // This page marks the boundary — include it in the outgoing episode
                            if !current_episode_pages.contains(&page_name) {
                                current_episode_pages.push(page_name.clone());
                            }
                            episode_counter += 1;
                            write_episode(&output_dir, episode_counter, &episode)?;
                            mark_completed(
                                &mut status_cache,
                                &episode.name,
                                episode_counter,
                                std::mem::take(&mut current_episode_pages),
                            );
                            save_status_cache(&status_path, &status_cache)?;
                            log::info!("Completed episode {}: {}", episode_counter, episode.name);
                        }
                    }

                    // Skip episodes that are already fully organized
                    if is_completed(&status_cache, &name) {
                        log::info!("Skipping already completed episode: {}", name);
                        current_episode = Some(Episode {
                            name,
                            content: String::new(),
                            sub_headers: Vec::new(),
                        });
                        current_episode_already_completed = true;
                        current_episode_pages = Vec::new();
                    } else {
                        log::debug!("Started new episode: {}", name);
                        current_episode = Some(Episode {
                            name,
                            content: data,
                            sub_headers: new_topics,
                        });
                        current_episode_already_completed = false;
                        current_episode_pages = vec![page_name.clone()];
                    }
                }
            }
        }
    }

    // Flush the final in-progress episode
    if let Some(episode) = current_episode {
        if !current_episode_already_completed {
            episode_counter += 1;
            write_episode(&output_dir, episode_counter, &episode)?;
            mark_completed(
                &mut status_cache,
                &episode.name,
                episode_counter,
                current_episode_pages,
            );
            save_status_cache(&status_path, &status_cache)?;
            log::info!(
                "Completed final episode {}: {}",
                episode_counter,
                episode.name
            );
        }
    }

    log::info!("Successfully created {} episodes", episode_counter);

    Ok(())
}
