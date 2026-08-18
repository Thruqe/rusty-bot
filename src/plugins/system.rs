use super::Command;
use crate::util::{START_TIME, now, uptime};

pub fn commands() -> Vec<Command> {
    let _ = *START_TIME;

    vec![
        Command::new("ping", |m, _args| async move {
            let start = now();
            let sent = m.reply("pinging...").await?;
            let end = now();
            let latency = end - start;

            m.edit(&sent.message_id, format!("{latency} ms")).await?;
            Ok(())
        })
        .alias("speed")
        .category("system")
        .public(true)
        .hidecommand(false),

        Command::new("uptime", |m, _args| async move {
            m.reply(format!("uptime: {}", uptime())).await?;
            Ok(())
        })
        .alias("runtime")
        .category("system")
        .public(true)
        .hidecommand(false),

        Command::new("quoted", |m, _args| async move {
            println!("[debug:quoted_cmd] invoked on msg_id={}", m.id);
            if let Some(ref q) = m.quoted {
                println!(
                    "[debug:quoted_cmd] replied to msg_id={} sender={}",
                    q.id, q.sender
                );
            } else {
                println!("[debug:quoted_cmd] current message does not quote/reply to any message");
            }

            if let Some(target_msg) = m.quoted_of_replied().await {
                println!("[debug:quoted_cmd] found inner quoted message, forwarding to chat...");
                m.send_message(target_msg).await?;
                println!("[debug:quoted_cmd] inner quoted message forwarded successfully");
            } else {
                println!("[debug:quoted_cmd] replied message does not contain an inner quoted message");
                m.reply("reply to a message that quotes another message").await?;
            }
            Ok(())
        })
        .alias("q")
        .category("system")
        .public(true)
        .hidecommand(false),
    ]
}
