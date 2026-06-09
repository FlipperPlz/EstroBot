use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use chrono::Utc;
use serenity::all::Http;
use serenity::model::id::UserId;
use serenity::model::channel::PrivateChannel;
use tokio::time::sleep;
use tracing::error;

use crate::data::store::Store;
use crate::models::Schedule;

pub async fn start_reminder_thread(store: Arc<Store>, http: Arc<Http>) {
    loop {
        let now = Utc::now().timestamp();
        
        match store.get_all_user_ids() {
            Ok(user_ids) => {
                for user_id in user_ids {
                    match store.load_profile(user_id) {
                        Ok(Some(mut profile)) => {
                            // Injection reminders (long-interval)
                            if profile.next_injection <= now {
                                let message = format!(
                                    "Time for your injection! Dose: {:.2} mg of {}.",
                                    profile.dose, profile.ester.get_data().s_name
                                );

                                if let Err(e) = send_dm(&http, user_id, &message).await {
                                    error!("Error sending DM to {}: {}", user_id, e);
                                }

                                profile.calculate_next();
                                if let Err(e) = store.save_profile(&profile) {
                                    error!("Error saving profile for {}: {}", user_id, e);
                                }
                            }

                            match &mut profile.schedule {
                                Schedule::PillSchedule { entries } => {
                                    let mut changed = false;
                                    let current_minutes = (Utc::now().timestamp() % 86400 / 60) as u32;
                                    let current_day = (Utc::now().timestamp() / 86400) as u32;

                                    for entry in entries.iter_mut() {
                                        if entry.time_minutes <= current_minutes {
                                            if entry.last_sent_day.map_or(true, |d| d < current_day) {
                                                let message = format!(
                                                    "Time for your medication: {:.2} mg of {} at {:02}:{:02}.",
                                                    entry.dose,
                                                    profile.ester.get_data().s_name,
                                                    entry.time_minutes / 60,
                                                    entry.time_minutes % 60
                                                );

                                                if let Err(e) = send_dm(&http, user_id, &message).await {
                                                    error!("Error sending DM to {}: {}", user_id, e);
                                                }

                                                entry.last_sent_day = Some(current_day);
                                                changed = true;
                                            }
                                        }
                                    }

                                    if changed {
                                        if let Err(e) = store.save_profile(&profile) {
                                            error!("Error saving profile for {}: {}", user_id, e);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        Ok(None) => {}
                        Err(e) => error!("Error loading profile for {}: {}", user_id, e),
                    }

                    match store.load_daily_reminders(user_id) {
                        Ok(mut reminders) => {
                            let mut changed = false;
                            let current_minutes = (Utc::now().timestamp() % 86400 / 60) as u32;
                            let current_day = (Utc::now().timestamp() / 86400) as u32;

                            for reminder in &mut reminders {
                                if reminder.time_minutes <= current_minutes {
                                    if reminder.last_sent_day.map_or(true, |day| day < current_day) {
                                        let message = format!("Daily Reminder: {}", reminder.label);
                                        if let Err(e) = send_dm(&http, user_id, &message).await {
                                            error!("Error sending DM to {}: {}", user_id, e);
                                        }
                                        reminder.last_sent_day = Some(current_day);
                                        changed = true;
                                    }
                                }
                            }

                            if changed {
                                if let Err(e) = store.save_daily_reminders(user_id, &reminders) {
                                    error!("Error saving daily reminders for {}: {}", user_id, e);
                                }
                            }
                        }
                        Err(e) => error!("Error loading reminders for {}: {}", user_id, e),
                    }
                }
            }
            Err(e) => error!("Error getting all user IDs: {}", e),
        }

        sleep(Duration::from_secs(60)).await;
    }
}

async fn send_dm(http: &Http, user_id: u64, content: &str) -> Result<()> {
    let user = UserId::new(user_id);
    let dm_channel: PrivateChannel = user.create_dm_channel(http).await?;
    dm_channel.say(http, content).await?;
    Ok(())
}

