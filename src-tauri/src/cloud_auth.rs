use std::sync::Mutex;

#[derive(Debug, Clone)]
pub(crate) struct CloudAuthSession {
    pub(crate) access_token: String,
    pub(crate) installation_id: String,
    pub(crate) app_version: String,
}

#[derive(Default)]
pub(crate) struct CloudAuthState {
    session: Mutex<Option<CloudAuthSession>>,
}

impl CloudAuthState {
    pub(crate) fn snapshot(&self) -> Option<CloudAuthSession> {
        self.session.lock().ok().and_then(|session| session.clone())
    }
}

#[tauri::command]
pub(crate) fn set_cloud_auth_session(
    state: tauri::State<'_, CloudAuthState>,
    access_token: Option<String>,
    installation_id: Option<String>,
    app_version: Option<String>,
) -> Result<(), String> {
    let next = match (access_token, installation_id, app_version) {
        (Some(access_token), Some(installation_id), Some(app_version))
            if access_token.split('.').count() == 3
                && access_token.len() <= 16 * 1024
                && (16..=64).contains(&installation_id.len())
                && !app_version.trim().is_empty() =>
        {
            Some(CloudAuthSession {
                access_token,
                installation_id,
                app_version,
            })
        }
        (None, _, _) => None,
        _ => return Err("The cloud authentication session is invalid.".to_string()),
    };
    *state
        .session
        .lock()
        .map_err(|_| "The cloud authentication session is unavailable.".to_string())? = next;
    Ok(())
}
