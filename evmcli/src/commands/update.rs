use comfy_table::Table;
use serde::Serialize;

use crate::errors::EvmError;
use crate::output::table::Tableable;

#[derive(Debug, Serialize)]
pub struct UpdateResult {
    pub current_version: String,
    pub latest_version: String,
    pub updated: bool,
    pub message: String,
}

impl Tableable for UpdateResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.add_row(vec!["Current", &self.current_version]);
        table.add_row(vec!["Latest", &self.latest_version]);
        table.add_row(vec!["Status", &self.message]);
        table
    }
}

pub async fn run(check_only: bool) -> Result<UpdateResult, EvmError> {
    let current = env!("CARGO_PKG_VERSION");

    let status = self_update::backends::github::Update::configure()
        .repo_owner("199-biotechnologies")
        .repo_name("crypto-skill")
        .bin_name("onchain")
        .current_version(current)
        .build()
        .map_err(|e| EvmError::config(format!("Update check failed: {e}")))?;

    let latest_release = status.get_latest_release()
        .map_err(|e| EvmError::config(format!("Could not check for updates: {e}")))?;

    let latest_version = latest_release.version.clone();
    let is_newer = latest_version != current;

    if !is_newer {
        return Ok(UpdateResult {
            current_version: current.to_string(),
            latest_version,
            updated: false,
            message: "Already up to date".to_string(),
        });
    }

    if check_only {
        return Ok(UpdateResult {
            current_version: current.to_string(),
            latest_version,
            updated: false,
            message: format!("Update available! Run 'onchain update' to install."),
        });
    }

    // Perform the update
    let update_status = self_update::backends::github::Update::configure()
        .repo_owner("199-biotechnologies")
        .repo_name("crypto-skill")
        .bin_name("onchain")
        .current_version(current)
        .build()
        .map_err(|e| EvmError::config(format!("Update failed: {e}")))?
        .update()
        .map_err(|e| EvmError::config(format!("Update failed: {e}")))?;

    Ok(UpdateResult {
        current_version: current.to_string(),
        latest_version: update_status.version().to_string(),
        updated: true,
        message: format!("Updated to {}", update_status.version()),
    })
}

/// Non-blocking version check — call on startup, print hint if update available.
/// Returns None if check fails or no update (don't block the user).
pub async fn check_for_update_hint() -> Option<String> {
    let current = env!("CARGO_PKG_VERSION");

    let status = self_update::backends::github::Update::configure()
        .repo_owner("199-biotechnologies")
        .repo_name("crypto-skill")
        .bin_name("onchain")
        .current_version(current)
        .build()
        .ok()?;

    let latest = status.get_latest_release().ok()?;

    if latest.version != current {
        Some(format!("Update available: {} -> {} (run 'onchain update')", current, latest.version))
    } else {
        None
    }
}
