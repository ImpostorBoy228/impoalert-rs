use std::env;

use log::info;
use sysinfo::System;
use teloxide::macros::BotCommands;
use teloxide::types::ChatId;
use teloxide::{prelude::*, RequestError};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "Get full system statistics")]
    Stats,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Starting impoalert");

    let token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN must be set");
    let chat_id: i64 = env::var("CHAT_ID")
        .expect("CHAT_ID must be set")
        .parse()
        .expect("CHAT_ID must be a valid integer");

    let bot = Bot::new(token);

    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(handle_command);

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    let chat_id = ChatId(chat_id);
    let bot_clone = bot.clone();
    tokio::spawn(async move {
        let mut prev_ips: std::collections::HashSet<String> = std::collections::HashSet::new();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            let output = match std::process::Command::new("w").arg("-h").output() {
                Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
                Err(_) => continue,
            };
            let current_ips: std::collections::HashSet<String> = output
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 && parts[2] != "-" {
                        Some(parts[2].to_string())
                    } else {
                        None
                    }
                })
                .collect();
            let new_ips: Vec<String> = current_ips.difference(&prev_ips).cloned().collect();
            if !new_ips.is_empty() {
                let full_w = String::from_utf8_lossy(
                    &std::process::Command::new("w").output().unwrap().stdout,
                );
                for ip in new_ips {
                    let msg = format!(
                        "🚨 SEX ALERT!!1!🚨\nБЛЯЯ ВЗЛОМ С IP: {ip}\ninfo: ```\n{full_w}\n```"
                    );
                    let _ = bot_clone.send_message(chat_id, msg).await;
                }
            }
            prev_ips = current_ips;
        }
    });
}

async fn handle_command(bot: Bot, msg: Message, cmd: Command) -> Result<(), RequestError> {
    match cmd {
        Command::Stats => {
            let mut sys = System::new_all();
            sys.refresh_all();
            let stats = format_stats(&mut sys).await;
            bot.send_message(msg.chat.id, stats).await?;
        }
    }
    Ok(())
}

fn global_cpu_percent(sys: &mut System) -> f64 {
    let cpus = sys.cpus();
    if cpus.is_empty() {
        return 0.0;
    }
    cpus.iter().map(|c| c.cpu_usage() as f64).sum::<f64>() / cpus.len() as f64
}

fn memory_percent(sys: &System) -> f64 {
    let total = sys.total_memory();
    if total == 0 {
        return 0.0;
    }
    sys.used_memory() as f64 / total as f64 * 100.0
}

fn swap_percent(sys: &System) -> f64 {
    let total = sys.total_swap();
    if total == 0 {
        return 0.0;
    }
    sys.used_swap() as f64 / total as f64 * 100.0
}

async fn format_stats(sys: &mut System) -> String {
    let la = System::load_average();
    let cpu_usage = global_cpu_percent(sys);
    let cpu_count = sys.cpus().len();
    let mem_used = sys.used_memory() / 1024 / 1024;
    let mem_total = sys.total_memory() / 1024 / 1024;
    let mem_percent = memory_percent(sys);
    let swap_used = sys.used_swap() / 1024 / 1024;
    let swap_total = sys.total_swap() / 1024 / 1024;
    let swap_percent = swap_percent(sys);
    let proc_count = sys.processes().len();

    let top_proc = sys
        .processes()
        .iter()
        .max_by(|(_, a), (_, b)| {
            a.cpu_usage()
                .partial_cmp(&b.cpu_usage())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(pid, p)| (pid, p.name().to_string_lossy(), p.cpu_usage(), p.memory() / 1024));

    let mut out = format!(
        "📊 SYSTEM STATS\n\
         ────────────────\n\
         🔄 Load Average: {la_one:.2}, {la_five:.2}, {la_fifteen:.2}\n\
         🖥️  CPU: {cpu_usage:.1}% ({cpu_count} cores)\n\
         💾 RAM: {mem_used} MB / {mem_total} MB ({mem_percent:.1}%)\n\
         💿 SWAP: {swap_used} MB / {swap_total} MB ({swap_percent:.1}%)\n\
         📋 Processes: {proc_count}",
        la_one = la.one,
        la_five = la.five,
        la_fifteen = la.fifteen,
    );

    if let Some((pid, name, cpu, mem)) = top_proc {
        out.push_str(&format!(
            "\n\n🔥 Top CPU Process: {name} (PID: {pid})\n   CPU: {cpu:.1}% | MEM: {mem} MB"
        ));
    }

    out
}
