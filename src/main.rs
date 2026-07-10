use std::env;
use std::time::Duration;

use log::info;
use sysinfo::System;
use teloxide::macros::BotCommands;
use teloxide::types::ChatId;
use teloxide::{prelude::*, RequestError};
use tokio::time::interval;

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
    let alerts_chat_id = ChatId(chat_id);

    let monitor_bot = bot.clone();
    tokio::spawn(async move {
        monitor_loop(monitor_bot, alerts_chat_id).await;
    });

    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(handle_command);

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn handle_command(bot: Bot, msg: Message, cmd: Command) -> Result<(), RequestError> {
    match cmd {
        Command::Stats => {
            let mut sys = System::new_all();
            sys.refresh_all();
            let stats = format_stats(&mut sys);
            bot.send_message(msg.chat.id, stats).await?;
        }
    }
    Ok(())
}

async fn monitor_loop(bot: Bot, chat_id: ChatId) {
    let mut sys = System::new_all();
    let mut tick = interval(Duration::from_secs(15));

    loop {
        tick.tick().await;
        sys.refresh_all();

        let cpu_usage = global_cpu_percent(&mut sys);
        let mem_percent = memory_percent(&sys);

        let mut msg = String::new();

        if mem_percent > 80.0 || cpu_usage > 90.0 {
            let what = if mem_percent > 80.0 && cpu_usage > 90.0 {
                "CPU & Memory"
            } else if mem_percent > 80.0 {
                "Memory"
            } else {
                "CPU"
            };
            msg.push_str(&format!(
                "🚨 SEX ALERT!!1!🚨\n{what} is fucking up.\nCPU: {cpu_usage:.1}% | MEM: {mem_percent:.1}%\n\n"
            ));
        }

        msg.push_str("🚨 SEX ALERT!!1!🚨\nБЛЯЯ ВЗЛОМ С IP: 12.345.67.67");

        if let Err(e) = bot.send_message(chat_id, msg.trim()).await {
            log::warn!("Failed to send alert: {e}");
        }
    }
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

fn format_stats(sys: &mut System) -> String {
    let la = sys.load_average();
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
