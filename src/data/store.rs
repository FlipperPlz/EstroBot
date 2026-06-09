use crate::models::{DailyReminder, Profile};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Store {
    dir_path: PathBuf,
}

impl Store {
    pub fn new(dir_path: impl AsRef<Path>) -> Result<Self> {
        let dir_path = dir_path.as_ref().to_path_buf();
        if !dir_path.exists() {
            fs::create_dir_all(&dir_path)?;
        }
        Ok(Self { dir_path })
    }

    fn profile_path(&self, user_id: u64) -> PathBuf {
        self.dir_path.join(format!("{}.json", user_id))
    }

    fn reminders_path(&self, user_id: u64) -> PathBuf {
        self.dir_path.join(format!("{}_reminders.json", user_id))
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<()> {
        let path = self.profile_path(profile.user_id);
        let json = serde_json::to_string_pretty(profile)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_profile(&self, user_id: u64) -> Result<Option<Profile>> {
        let path = self.profile_path(user_id);
        if !path.exists() {
            return Ok(None);
        }
        let json = fs::read_to_string(path)?;
        let profile = serde_json::from_str(&json)?;
        Ok(Some(profile))
    }

    pub fn save_daily_reminders(&self, user_id: u64, reminders: &[DailyReminder]) -> Result<()> {
        let path = self.reminders_path(user_id);
        let json = serde_json::to_string_pretty(reminders)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_daily_reminders(&self, user_id: u64) -> Result<Vec<DailyReminder>> {
        let path = self.reminders_path(user_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let json = fs::read_to_string(path)?;
        let reminders = serde_json::from_str(&json)?;
        Ok(reminders)
    }

    pub fn get_all_user_ids(&self) -> Result<Vec<u64>> {
        let mut user_ids = Vec::new();
        for entry in fs::read_dir(&self.dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if file_name.ends_with(".json") && !file_name.ends_with("_reminders.json") {
                        if let Some(id_str) = file_name.strip_suffix(".json") {
                            if let Ok(id) = id_str.parse::<u64>() {
                                user_ids.push(id);
                            }
                        }
                    }
                }
            }
        }
        Ok(user_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use tempfile::tempdir;

    #[test]
    fn test_store_save_load_profile() {
        let dir = tempdir().unwrap();
        let store = Store::new(dir.path()).unwrap();

        let profile = Profile {
            user_id: 12345,
            plan: TransitionPlan::Diy,
            dose: 4.0,
            method: IngestionMethod::Injection,
            ester: InjectionEster::Enanthate,
            schedule: Schedule::Interval { hours: 168.0 },
            next_injection: 1622505600,
            first_injection: Some(1621900800),
            timezone: "UTC".to_string(),
        };

        store.save_profile(&profile).unwrap();
        let loaded = store.load_profile(12345).unwrap().unwrap();

        assert_eq!(loaded.user_id, profile.user_id);
        assert_eq!(loaded.dose, profile.dose);
    }

    #[test]
    fn test_store_reminders() {
        let dir = tempdir().unwrap();
        let store = Store::new(dir.path()).unwrap();
        let user_id = 999;

        let reminders = vec![
            DailyReminder {
                user_id,
                time_minutes: 480,
                label: "Blocker".to_string(),
                last_sent_day: None,
            }
        ];

        store.save_daily_reminders(user_id, &reminders).unwrap();
        let loaded = store.load_daily_reminders(user_id).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].label, "Blocker");
    }
}

