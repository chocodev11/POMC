use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Account {
    pub username: String,
    pub uuid: String,
}

#[derive(Deserialize)]
struct MojangPatchNotes {
    entries: Vec<MojangEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MojangEntry {
    title: String,
    version: String,
    date: String,
    short_text: String,
    image: MojangImage,
    #[serde(rename = "type")]
    entry_type: String,
    #[serde(rename = "contentPath")]
    content_path: String,
}

#[derive(Deserialize)]
struct MojangImage {
    url: String,
}

#[derive(Deserialize)]
struct MojangContent {
    body: String,
}

#[derive(Serialize)]
pub struct PatchNote {
    pub title: String,
    pub version: String,
    pub date: String,
    pub summary: String,
    pub image_url: String,
    pub entry_type: String,
    pub content_path: String,
}

const PATCH_NOTES_URL: &str = "https://launchercontent.mojang.com/v2/javaPatchNotes.json";
const IMAGE_BASE: &str = "https://launchercontent.mojang.com";

#[tauri::command]
pub async fn get_patch_notes(count: Option<usize>) -> Result<Vec<PatchNote>, String> {
    let limit = count.unwrap_or(6);
    let resp: MojangPatchNotes = reqwest::get(PATCH_NOTES_URL)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp
        .entries
        .into_iter()
        .take(limit)
        .map(|e| PatchNote {
            title: e.title,
            date: e.date.chars().take(10).collect(),
            summary: e.short_text,
            image_url: format!("{IMAGE_BASE}{}", e.image.url),
            entry_type: e.entry_type,
            content_path: e.content_path,
            version: e.version,
        })
        .collect())
}

#[tauri::command]
pub async fn get_patch_content(content_path: String) -> Result<String, String> {
    let url = format!("{IMAGE_BASE}/v2/{content_path}");
    let content: MojangContent = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(content.body)
}

#[derive(Deserialize)]
struct SessionProfile {
    properties: Vec<SessionProperty>,
}

#[derive(Deserialize)]
struct SessionProperty {
    value: String,
}

#[derive(Deserialize)]
struct TexturesPayload {
    textures: Textures,
}

#[derive(Deserialize)]
struct Textures {
    #[serde(rename = "SKIN")]
    skin: Option<SkinTexture>,
}

#[derive(Deserialize)]
struct SkinTexture {
    url: String,
}

#[tauri::command]
pub async fn get_skin_url(uuid: String) -> Result<String, String> {
    let clean_uuid = uuid.replace('-', "");
    let url = format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{clean_uuid}"
    );
    let profile: SessionProfile = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let value = &profile
        .properties
        .first()
        .ok_or("No properties")?
        .value;

    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|e| e.to_string())?;
    let payload: TexturesPayload =
        serde_json::from_slice(&decoded).map_err(|e| e.to_string())?;

    payload
        .textures
        .skin
        .map(|s| s.url)
        .ok_or_else(|| "No skin texture".to_string())
}

#[tauri::command]
pub fn get_all_accounts() -> Vec<crate::auth::AuthAccount> {
    crate::auth::get_all_accounts()
}

#[tauri::command]
pub async fn add_account() -> Result<crate::auth::AuthAccount, String> {
    let (_, device_code, expires_in, interval) =
        crate::auth::start_device_code_flow().await?;
    crate::auth::poll_for_token(&device_code, expires_in, interval).await
}

#[tauri::command]
pub fn remove_account(uuid: String) {
    crate::auth::remove_account(&uuid);
}

#[derive(Deserialize)]
struct VersionManifest {
    versions: Vec<VersionEntry>,
}

#[derive(Deserialize, Serialize)]
struct VersionEntry {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
}

#[derive(Serialize, Clone)]
pub struct GameVersion {
    pub id: String,
    pub version_type: String,
}

static VERSION_CACHE: std::sync::OnceLock<Vec<GameVersion>> = std::sync::OnceLock::new();

async fn fetch_versions() -> Result<&'static Vec<GameVersion>, String> {
    if let Some(cached) = VERSION_CACHE.get() {
        return Ok(cached);
    }

    let manifest: VersionManifest =
        reqwest::get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

    let versions: Vec<GameVersion> = manifest
        .versions
        .into_iter()
        .map(|v| GameVersion {
            id: v.id,
            version_type: v.version_type,
        })
        .collect();

    Ok(VERSION_CACHE.get_or_init(|| versions))
}

#[tauri::command]
pub async fn get_versions(show_snapshots: Option<bool>) -> Result<Vec<GameVersion>, String> {
    let all = fetch_versions().await?;
    let include_snapshots = show_snapshots.unwrap_or(false);
    Ok(all
        .iter()
        .filter(|v| include_snapshots || v.version_type == "release")
        .cloned()
        .collect())
}

#[tauri::command]
pub async fn refresh_account(uuid: String) -> Result<crate::auth::AuthAccount, String> {
    crate::auth::try_restore_or_refresh(&uuid)
        .await
        .ok_or_else(|| "Failed to refresh account".to_string())
}

#[tauri::command]
pub async fn ensure_assets(app: tauri::AppHandle, version: String) -> Result<(), String> {
    if crate::downloader::needs_download(&version) {
        crate::downloader::download(&app, &version).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn launch_game(username: String, server: Option<String>) -> Result<String, String> {
    let exe = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("no parent dir")?
        .join("pomc.exe");

    if !exe.exists() {
        return Err(format!("Game binary not found at {}", exe.display()));
    }

    let assets = crate::downloader::assets_dir();
    let mut cmd = tokio::process::Command::new(&exe);
    cmd.arg("--username")
        .arg(&username)
        .arg("--assets-dir")
        .arg(assets.to_string_lossy().as_ref());

    if let Some(server) = &server {
        cmd.arg("--server").arg(server);
    }

    cmd.spawn().map_err(|e| e.to_string())?;

    Ok(format!("Launched as {username}"))
}
