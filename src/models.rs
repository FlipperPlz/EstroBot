use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionPlan {
    Diy,
    Prescribed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IngestionMethod {
    Oral,
    Injection,
    Transdermal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InjectionEster {
    Benzoate,
    Valerate,
    Cypionate,
    CypionateSuspension,
    Enanthate,
    Undecylate,
    PolyestradiolPhosphate,
    Estradiol,
}

impl InjectionEster {
    pub fn get_data(&self) -> EsterData {
        match self {
            Self::Benzoate => EsterData { s_name: "EB", dose_form: IngestionMethod::Injection, half_life_days: 3.5, peak_time_days: 0.5, k_absorption: 1.505, k_elimination: 0.354 },
            Self::Valerate => EsterData { s_name: "EV", dose_form: IngestionMethod::Injection, half_life_days: 3.5, peak_time_days: 1.2, k_absorption: 0.925, k_elimination: 0.165 },
            Self::Cypionate => EsterData { s_name: "EC", dose_form: IngestionMethod::Injection, half_life_days: 7.5, peak_time_days: 2.0, k_absorption: 0.257, k_elimination: 0.08 },
            Self::CypionateSuspension => EsterData { s_name: "ECS", dose_form: IngestionMethod::Injection, half_life_days: 9.0, peak_time_days: 2.0, k_absorption: 0.57, k_elimination: 0.113 },
            Self::Enanthate => EsterData { s_name: "EE", dose_form: IngestionMethod::Injection, half_life_days: 6.55, peak_time_days: 2.0, k_absorption: 1.51, k_elimination: 0.12 },
            Self::Undecylate => EsterData { s_name: "EU", dose_form: IngestionMethod::Injection, half_life_days: 20.0, peak_time_days: 7.0, k_absorption: 0.035, k_elimination: 0.029 },
            Self::Estradiol => EsterData { s_name: "E2", dose_form: IngestionMethod::Oral, half_life_days: 0.5, peak_time_days: 0.1, k_absorption: 36.5, k_elimination: 1.135 },
            _ => EsterData { s_name: "Unknown", dose_form: IngestionMethod::Injection, half_life_days: 5.0, peak_time_days: 1.0, k_absorption: 0.0, k_elimination: 0.0 },
        }
    }

    pub fn get_intervals(&self) -> Vec<f32> {
        match self {
            Self::Valerate => vec![3.5, 5.0],
            Self::Enanthate | Self::Cypionate => vec![5.0, 7.0, 10.0],
            Self::Undecylate => vec![7.0, 14.0, 21.0],
            _ => vec![7.0],
        }
    }
}

pub struct EsterData {
    pub s_name: &'static str,
    pub dose_form: IngestionMethod,
    pub half_life_days: f32,
    pub peak_time_days: f32,
    pub k_absorption: f32,
    pub k_elimination: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub user_id: u64,
    pub plan: TransitionPlan,
    pub dose: f32,
    pub method: IngestionMethod,
    pub ester: InjectionEster,
    pub schedule: Schedule,
    pub next_injection: i64,
    pub first_injection: Option<i64>,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PillEntry {
    pub time_minutes: u32,
    pub dose: f32,
    pub last_sent_day: Option<u32>,
}

impl PillEntry {
    pub fn parse_schedule(schedule_str: &str, dose: f32) -> anyhow::Result<Vec<Self>> {
        let mut entries = Vec::new();
        for part in schedule_str.split(',') {
            let part = part.trim().to_lowercase();
            if part.is_empty() { continue; }

            let is_pm = part.ends_with("pm");
            let is_am = part.ends_with("am");

            if !is_pm && !is_am {
                return Err(anyhow::anyhow!("Time must specify AM or PM (e.g., 12pm, 8:30am): {}", part));
            }

            let time_part = &part[..part.len() - 2].trim();
            let (hour, minute) = if time_part.contains(':') {
                let pieces: Vec<&str> = time_part.split(':').collect();
                if pieces.len() != 2 { return Err(anyhow::anyhow!("Invalid time format: {}", part)); }
                let h = pieces[0].parse::<u32>().map_err(|_| anyhow::anyhow!("Invalid hour: {}", pieces[0]))?;
                let m = pieces[1].parse::<u32>().map_err(|_| anyhow::anyhow!("Invalid minute: {}", pieces[1]))?;
                (h, m)
            } else {
                let h = time_part.parse::<u32>().map_err(|_| anyhow::anyhow!("Invalid hour: {}", time_part))?;
                (h, 0)
            };

            if hour == 0 || hour > 12 || minute >= 60 {
                return Err(anyhow::anyhow!("Invalid time (use 1-12 for hours): {}", part));
            }

            let mut final_hour = hour % 12;
            if is_pm {
                final_hour += 12;
            }

            entries.push(Self {
                time_minutes: final_hour * 60 + minute,
                dose,
                last_sent_day: None,
            });
        }
        Ok(entries)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Schedule {
    Interval { hours: f32 },
    PillSchedule { entries: Vec<PillEntry> },
}

impl Schedule {
    pub fn calculate_next_timestamp(&self, current_ts: i64) -> i64 {
        match self {
            Self::Interval { hours } => {
                let interval_seconds = (hours * 60.0 * 60.0) as i64;
                current_ts + interval_seconds
            }
            Self::PillSchedule { entries } => {
                if entries.is_empty() { return current_ts; }
                let midnight = current_ts - (current_ts % 86400);
                let mut found: Option<i64> = None;
                for entry in entries {
                    let ts = midnight + (entry.time_minutes as i64) * 60;
                    if ts > current_ts {
                        found = Some(ts);
                        break;
                    }
                }
                found.unwrap_or_else(|| midnight + 86400 + (entries[0].time_minutes as i64) * 60)
            }
        }
    }
}

impl Profile {
    pub fn calculate_next(&mut self) {
        self.next_injection = self.schedule.calculate_next_timestamp(self.next_injection);
    }

    pub fn get_stats(&self) -> String {
        {
            let mut s = format!(
                "**HRT Profile Stats**\nPlan: {:?}\nMethod: {:?}\nEster: {}\nDose: {:.2} mg\nTimezone: {}\n",
                self.plan,
                self.method,
                self.ester.get_data().s_name,
                self.dose,
                self.timezone,
            );


            match &self.schedule {
                Schedule::Interval { hours } => {
                    s.push_str(&format!("Interval: {:.1} hours\n", hours));
                }
                Schedule::PillSchedule { entries } => {
                    s.push_str("Pill schedule:\n");
                    for entry in entries {
                        let hours = entry.time_minutes / 60;
                        let minutes = entry.time_minutes % 60;
                        s.push_str(&format!(" - {:02}:{:02} => {:.2} mg\n", hours, minutes, entry.dose));
                    }
                }
            }

            s.push_str(&format!("Next Injection: <t:{}:F>", self.next_injection));

            s
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReminder {
    pub user_id: u64,
    pub time_minutes: u32,
    pub label: String,
    pub last_sent_day: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_next() {
        let mut profile = Profile {
            user_id: 1,
            plan: TransitionPlan::Diy,
            dose: 4.0,
            method: IngestionMethod::Injection,
            ester: InjectionEster::Enanthate,
            schedule: Schedule::Interval { hours: 168.0 },
            next_injection: 1000,
            first_injection: None,
            timezone: "UTC".to_string(),
        };

        profile.calculate_next();
        let expected = 1000 + (168 * 60 * 60);
        assert_eq!(profile.next_injection, expected);
    }
}
