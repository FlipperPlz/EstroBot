use serenity::all::{
    ActionRowComponent, ButtonStyle, CommandInteraction, ComponentInteraction,
    ModalInteraction, CreateActionRow, CreateButton, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateModal, CreateInputText, InputTextStyle
};
use anyhow::{Result, anyhow};
use crate::models::{InjectionEster, IngestionMethod, Profile, TransitionPlan, Schedule};
use crate::data::store::Store;

pub fn register() -> serenity::builder::CreateCommand {
    serenity::builder::CreateCommand::new("setup")
        .description("Create or edit profile")
}

pub async fn handle_command(ctx: &serenity::prelude::Context, command: &CommandInteraction) -> Result<()> {
    let response = CreateInteractionResponseMessage::new()
        .content("Profile Options:")
        .components(vec![
            CreateActionRow::Buttons(vec![
                CreateButton::new("setup_start")
                    .label("Setup Profile")
                    .style(ButtonStyle::Primary),
                CreateButton::new("edit_profile")
                    .label("Edit Profile")
                    .style(ButtonStyle::Secondary),
            ])
        ]);

    command.create_response(&ctx.http, CreateInteractionResponse::Message(response)).await?;
    Ok(())
}

pub async fn handle_component(ctx: &serenity::prelude::Context, component: &ComponentInteraction, store: &Store) -> Result<()> {
    if component.data.custom_id.starts_with("apply:") {
        let parts: Vec<&str> = component.data.custom_id.split(':').collect();
        if parts.len() == 3 {
            let ester_short = parts[1];
            let interval_hours = parts[2].parse::<f32>()?;
            let ester = parse_ester(ester_short)?;
            let user_id = component.user.id.get();
            let now = chrono::Utc::now().timestamp();

            let profile = Profile {
                user_id,
                plan: TransitionPlan::Diy,
                dose: 0.0,
                method: ester.get_data().dose_form,
                ester,
                schedule: Schedule::Interval { hours: interval_hours },
                next_injection: now,
                first_injection: Some(now),
                timezone: "UTC".to_string(),
            };

            store.save_profile(&profile)?;

            let response = CreateInteractionResponseMessage::new()
                .content(format!("Profile applied successfully!\n\n{}", profile.get_stats()))
                .components(vec![])
                .ephemeral(true);

            component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(response)).await?;
        }
        return Ok(());
    }

    match component.data.custom_id.as_str() {
        "setup_start" => {
            let response = CreateInteractionResponseMessage::new()
                .content("Select your administration method:")
                .components(vec![
                    CreateActionRow::Buttons(vec![
                        CreateButton::new("setup_oral").label("Oral").style(ButtonStyle::Primary),
                        CreateButton::new("setup_injection").label("Injection").style(ButtonStyle::Primary),
                        CreateButton::new("setup_transdermal").label("Transdermal").style(ButtonStyle::Primary),
                    ])
                ]);
            component.create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(response)).await?;
        }
        "setup_oral" => {
            let modal = CreateModal::new("modal_setup_oral", "Oral Profile Setup")
                .components(vec![
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Dose (mg)", "dose")),
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Timezone (e.g. UTC, EST, Europe/London)", "timezone")),
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Pill schedule (Hours 0-23, comma-separated)", "pill_schedule")),
                ]);
            component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await?;
        }
        "setup_injection" => {
            let modal = CreateModal::new("modal_setup_injection", "Injection Profile Setup")
                .components(vec![
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Dose (mg)", "dose")),
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Ester (ev/ec/ee/eu)", "ester")),
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Interval (days)", "interval")),
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Timezone", "timezone")),
                ]);
            component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await?;
        }
        "setup_transdermal" => {
            let modal = CreateModal::new("modal_setup_transdermal", "Transdermal Profile Setup")
                .components(vec![
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Dose (mg)", "dose")),
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Interval (days)", "interval")),
                    CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Timezone", "timezone")),
                ]);
            component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await?;
        }
        "edit_profile" => {
            let user_id = component.user.id.get();
            match store.load_profile(user_id)? {
                Some(profile) => {
                    let mut components = vec![
                        CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Dose (mg)", "dose").value(profile.dose.to_string())),
                    ];
                    
                    match profile.method {
                        IngestionMethod::Oral => {
                            let schedule_str = if let Schedule::PillSchedule { entries } = &profile.schedule {
                                entries.iter().map(|e| (e.time_minutes / 60).to_string()).collect::<Vec<_>>().join(", ")
                            } else { String::new() };
                            components.push(CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Pill schedule (Hours 0-23, comma-separated)", "pill_schedule").value(schedule_str)));
                        }
                        IngestionMethod::Injection => {
                            let interval_days = if let Schedule::Interval { hours } = profile.schedule { hours / 24.0 } else { 0.0 };
                            components.push(CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Ester (ev/ec/ee/eu)", "ester").value(profile.ester.get_data().s_name.to_lowercase())));
                            components.push(CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Interval (days)", "interval").value(interval_days.to_string())));
                        }
                        IngestionMethod::Transdermal => {
                            let interval_days = if let Schedule::Interval { hours } = profile.schedule { hours / 24.0 } else { 0.0 };
                            components.push(CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Ester (ev/ec/ee/eu)", "ester").value(profile.ester.get_data().s_name.to_lowercase())));
                            components.push(CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Interval (days)", "interval").value(interval_days.to_string())));
                        }
                    }
                    
                    components.push(CreateActionRow::InputText(CreateInputText::new(InputTextStyle::Short, "Timezone", "timezone").value(profile.timezone)));

                    let modal = CreateModal::new("modal_edit", "Edit Profile")
                        .components(components);
                    component.create_response(&ctx.http, CreateInteractionResponse::Modal(modal)).await?;
                }
                None => {
                    let response = CreateInteractionResponseMessage::new().content("No profile found. Please setup a profile first.").ephemeral(true);
                    component.create_response(&ctx.http, CreateInteractionResponse::Message(response)).await?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub async fn handle_modal(ctx: &serenity::prelude::Context, modal: &ModalInteraction, store: &Store) -> Result<()> {
    let user_id = modal.user.id.get();
    
    match modal.data.custom_id.as_str() {
        "modal_setup_oral" | "modal_setup_injection" | "modal_setup_transdermal" => {
            let dose_str = get_modal_field(modal, "dose");
            let timezone = get_modal_field(modal, "timezone");
            let pill_schedule_str = get_modal_field(modal, "pill_schedule");
            let ester_str = get_modal_field(modal, "ester");
            let interval_str = get_modal_field(modal, "interval");

            let dose = dose_str.parse::<f32>().map_err(|_| anyhow!("Invalid dose number."))?;
            let method = match modal.data.custom_id.as_str() {
                "modal_setup_oral" => IngestionMethod::Oral,
                "modal_setup_injection" => IngestionMethod::Injection,
                "modal_setup_transdermal" => IngestionMethod::Transdermal,
                _ => unreachable!(),
            };

            let ester = if method == IngestionMethod::Injection {
                parse_ester(&ester_str)?
            } else if method == IngestionMethod::Oral {
                InjectionEster::Estradiol
            } else {
                InjectionEster::Benzoate // Placeholder for transdermal
            };

            let interval_hours = if method == IngestionMethod::Oral {
                24.0
            } else {
                let days = interval_str.parse::<f32>().unwrap_or(0.0);
                days * 24.0
            };

            let pill_entries = if method == IngestionMethod::Oral {
                crate::models::PillEntry::parse_schedule(&pill_schedule_str, dose)?
            } else {
                Vec::new()
            };

            let schedule = if !pill_entries.is_empty() {
                Schedule::PillSchedule { entries: pill_entries }
            } else {
                Schedule::Interval { hours: interval_hours }
            };

            let now = chrono::Utc::now().timestamp();
            let next_injection = schedule.calculate_next_timestamp(now);

            let profile = Profile {
                user_id,
                plan: TransitionPlan::Diy,
                dose,
                method,
                ester,
                schedule,
                next_injection,
                first_injection: Some(now),
                timezone,
            };

            store.save_profile(&profile)?;
            let content = format!("Profile setup complete!\n\n{}", profile.get_stats());
            modal.create_response(&ctx.http, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(content).ephemeral(true))).await?;
        }
        "modal_edit" => {
            let dose_str = get_modal_field(modal, "dose");
            let timezone = get_modal_field(modal, "timezone");
            let pill_schedule_str = get_modal_field(modal, "pill_schedule");
            let interval_str = get_modal_field(modal, "interval");
            let ester_str = get_modal_field(modal, "ester");

            let mut profile = store.load_profile(user_id)?.ok_or_else(|| anyhow!("No profile found."))?;
            
            if !dose_str.is_empty() {
                profile.dose = dose_str.parse::<f32>()?;
            }
            if !timezone.is_empty() {
                profile.timezone = timezone;
            }

            match profile.method {
                IngestionMethod::Oral => {
                    if !pill_schedule_str.is_empty() {
                        profile.schedule = Schedule::PillSchedule { 
                            entries: crate::models::PillEntry::parse_schedule(&pill_schedule_str, profile.dose)? 
                        };
                    }
                }
                IngestionMethod::Injection => {
                    if !ester_str.is_empty() {
                        profile.ester = parse_ester(&ester_str)?;
                    }
                    if !interval_str.is_empty() {
                        let days = interval_str.parse::<f32>()?;
                        profile.schedule = Schedule::Interval { hours: days * 24.0 };
                    }
                }
                IngestionMethod::Transdermal => {
                    if !interval_str.is_empty() {
                        let days = interval_str.parse::<f32>()?;
                        profile.schedule = Schedule::Interval { hours: days * 24.0 };
                    }
                }
            }

            profile.calculate_next();
            store.save_profile(&profile)?;

            let content = format!("Profile updated successfully!\n\n{}", profile.get_stats());
            modal.create_response(&ctx.http, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(content).ephemeral(true))).await?;
        }
        _ => return Err(anyhow!("Unknown modal ID")),
    }
    Ok(())
}

fn get_modal_field(modal: &ModalInteraction, field_id: &str) -> String {
    for row in &modal.data.components {
        if let ActionRowComponent::InputText(it) = &row.components[0] {
            if it.custom_id == field_id {
                return it.value.clone().unwrap_or_default();
            }
        }
    }
    String::new()
}

fn parse_ester(s: &str) -> Result<InjectionEster> {
    match s.to_lowercase().as_str() {
        "ev" | "valerate" => Ok(InjectionEster::Valerate),
        "ec" | "cypionate" => Ok(InjectionEster::Cypionate),
        "ee" | "enanthate" => Ok(InjectionEster::Enanthate),
        "eu" | "undecylate" => Ok(InjectionEster::Undecylate),
        "e2" | "estradiol" => Ok(InjectionEster::Estradiol),
        "eb" | "benzoate" => Ok(InjectionEster::Benzoate),
        "ecs" => Ok(InjectionEster::CypionateSuspension),
        _ => Err(anyhow!("Invalid ester. Use ev, ec, ee, eu, or e2.")),
    }
}

#[derive(Debug)]
pub struct DosingResult {
    pub ester: InjectionEster,
    pub estimated_dose_mg: f64,
    pub predicted_peak_pg_ml: f64,
    pub predicted_trough_pg_ml: f64,
    pub interval_days: f64,
    pub peak_trough_ratio: f64,
}

///Compute steady-state concentration for a nominal 1.0mg dose.
fn calculate_nominal_concentration(t: f64, interval: f64, ka: f64, ke: f64, scale: f64) -> f64 {
    let e_ke_tau = (-ke * interval).exp();
    let e_ka_tau = (-ka * interval).exp();
    let term1 = (-ke * t).exp() / (1.0 - e_ke_tau);
    let term2 = (-ka * t).exp() / (1.0 - e_ka_tau);
    (ka / (ka - ke)) * scale * (term1 - term2)
}

/// Compute the peak-to-trough ratio for intervals.
fn evaluate_interval_ratio(interval: f64, ka: f64, ke: f64, scale: f64) -> f64 {
    let num = ka * (1.0 - (-ke * interval).exp());
    let den = ke * (1.0 - (-ka * interval).exp());
    let t_max = (1.0 / (ka - ke)) * (num / den).ln();
    let bounded_t_max = t_max.clamp(0.0, interval);

    let peak = calculate_nominal_concentration(bounded_t_max, interval, ka, ke, scale);
    let trough = calculate_nominal_concentration(0.0, interval, ka, ke, scale);

    if trough > 0.0 { peak / trough } else { f64::MAX }
}

/// Selects the best practical interval based on the ester's half-life.
/// Targets an ideal peak/trough ratio <= 2.2 for smooth levels, defaulting up or down gracefully.
fn optimize_interval(ka: f64, ke: f64, scale: f64) -> f64 {
    let practical_intervals = [1.0, 3.5, 5.0, 7.0, 14.0, 28.0];
    let target_max_ratio = 2.2;
    let mut best_interval = practical_intervals[0];

    for &interval in practical_intervals.iter() {
        let ratio = evaluate_interval_ratio(interval, ka, ke, scale);
        if ratio <= target_max_ratio {
            best_interval = interval;
        } else {
            if interval == practical_intervals[0] {
                return interval;
            }
            break;
        }
    }
    best_interval
}

pub fn estimate_optimized_transition_dose(
    ester: InjectionEster,
    desired_average: f64,
    accuracy: f64,
) -> Result<DosingResult, &'static str> {
    if desired_average <= 0.0 || accuracy <= 0.0 {
        return Err("Desired average level and accuracy must be positive values.");
    }

    let params = ester.get_data();
    let ke = 2.0_f64.ln() / params.half_life_days;
    let mut ka = 2.0_f64.ln() / params.absorption_half_life_days;

    if (ka - ke).abs() < 1e-6 { ka += 1e-5; }

    let tau = optimize_interval(ka, ke, params.clearance_scale);

    let calculated_dose = (desired_average * ke * tau) / params.clearance_scale;

    let num = ka * (1.0 - (-ke * tau).exp());
    let den = ke * (1.0 - (-ka * tau).exp());
    let t_max = ((1.0 / (ka - ke)) * (num / den).ln()).clamp(0.0, tau);

    let peak = calculate_nominal_concentration(t_max, tau, ka, ke, params.clearance_scale) * calculated_dose;
    let trough = calculate_nominal_concentration(0.0, tau, ka, ke, params.clearance_scale) * calculated_dose;
    let ratio = if trough > 0.0 { peak / trough } else { 0.0 };

    let calculated_avg = (params.clearance_scale * calculated_dose) / (ke * tau);
    if (calculated_avg - desired_average).abs() > accuracy {
        return Err("Analytical convergence failed to satisfy accuracy constraints.");
    }

    Ok(DosingResult {
        ester,
        estimated_dose_mg: calculated_dose,
        predicted_peak_pg_ml: peak,
        predicted_trough_pg_ml: trough,
        interval_days: tau,
        peak_trough_ratio: ratio,
    })
}