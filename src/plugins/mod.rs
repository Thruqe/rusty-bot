pub mod media;
pub mod system;

use crate::serialize::SerializedMessage;
use crate::util::{process_memory, uptime};
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub type CommandResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type CommandHandler = Arc<
    dyn Fn(Arc<SerializedMessage>, Vec<String>) -> BoxFuture<'static, CommandResult> + Send + Sync,
>;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CommandInfo {
    pub pattern: String,
    pub alias: Option<String>,
    pub description: String,
    pub usage: String,
    pub public: bool,
    pub owner_only: bool,
    pub group_only: bool,
    pub private_only: bool,
    pub hide_command: bool,
    pub category: String,
}

#[allow(dead_code)]
pub struct Command {
    pub info: CommandInfo,
    pub func: CommandHandler,
}

#[allow(dead_code)]
impl Command {
    pub fn new<F, Fut>(pattern: &str, func: F) -> Self
    where
        F: Fn(Arc<SerializedMessage>, Vec<String>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CommandResult> + Send + 'static,
    {
        Self {
            info: CommandInfo {
                pattern: pattern.to_string(),
                alias: None,
                description: String::new(),
                usage: String::new(),
                public: true,
                owner_only: false,
                group_only: false,
                private_only: false,
                hide_command: false,
                category: "general".to_string(),
            },
            func: Arc::new(move |m, args| Box::pin(func(m, args))),
        }
    }

    pub fn alias(mut self, alias: &str) -> Self {
        self.info.alias = Some(alias.to_string());
        self
    }

    pub fn desc(mut self, description: &str) -> Self {
        self.info.description = description.to_string();
        self
    }

    pub fn usage(mut self, usage: &str) -> Self {
        self.info.usage = usage.to_string();
        self
    }

    pub fn public(mut self, public: bool) -> Self {
        self.info.public = public;
        self
    }

    pub fn owner_only(mut self, owner_only: bool) -> Self {
        self.info.owner_only = owner_only;
        if owner_only {
            self.info.public = false;
        }
        self
    }

    pub fn group_only(mut self, group_only: bool) -> Self {
        self.info.group_only = group_only;
        self
    }

    pub fn private_only(mut self, private_only: bool) -> Self {
        self.info.private_only = private_only;
        self
    }

    pub fn hidecommand(mut self, hide: bool) -> Self {
        self.info.hide_command = hide;
        self
    }

    pub fn category(mut self, category: &str) -> Self {
        self.info.category = category.to_string();
        self
    }
}

pub fn generate_menu(
    commands: &[CommandInfo],
    prefix: &str,
    bot_name: &str,
    session_name: &str,
    is_owner: bool,
) -> String {
    let mut categories: BTreeMap<String, Vec<&CommandInfo>> = BTreeMap::new();

    for cmd in commands {
        if cmd.hide_command || (!cmd.public && !is_owner) {
            continue;
        }
        categories
            .entry(cmd.category.to_lowercase())
            .or_default()
            .push(cmd);
    }

    let total_plugins: usize = categories.values().map(|v| v.len()).sum();

    let mut out = String::new();
    out.push_str(&format!("{}\n", if bot_name.is_empty() { "wa-bot" } else { bot_name }));
    out.push_str(&format!("session: {}\n", session_name));
    out.push_str(&format!(
        "prefix: {}\n",
        if prefix.is_empty() { "none" } else { prefix }
    ));
    out.push_str(&format!("uptime: {}\n", uptime()));
    out.push_str(&format!("commands: {}\n", total_plugins));
    out.push_str(&format!("memory: {}\n\n", process_memory()));

    for (category, mut cmds) in categories {
        cmds.sort_by(|a, b| a.pattern.cmp(&b.pattern));

        out.push_str(&format!("{}:\n", category));
        for cmd in cmds {
            out.push_str(&format!("- {}{}\n", prefix, cmd.pattern));
        }
        out.push_str("\n");
    }

    out.trim_end().to_string()
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct CommandManager {
    pub prefix: String,
    pub bot_name: String,
    pub commands: Arc<Vec<Command>>,
}

impl CommandManager {
    pub async fn dispatch(&self, msg: SerializedMessage) -> CommandResult {
        let raw_text = match msg.text() {
            Some(t) => t.trim(),
            None => return Ok(()),
        };

        let effective_prefix = msg.session_prefix.as_deref().unwrap_or(&self.prefix);

        println!(
            "[debug:dispatch] raw_text={:?} prefix='{}'",
            raw_text, effective_prefix
        );

        let content = if !effective_prefix.is_empty() {
            if !raw_text.starts_with(effective_prefix) {
                return Ok(());
            }
            &raw_text[effective_prefix.len()..]
        } else {
            raw_text
        };

        let mut parts = content.split_whitespace();
        let cmd_name = match parts.next() {
            Some(cmd) => cmd.to_lowercase(),
            None => return Ok(()),
        };
        let args: Vec<String> = parts.map(String::from).collect();

        println!(
            "[debug:dispatch] cmd_name='{}' args={:?}",
            cmd_name, args
        );

        for cmd in self.commands.iter() {
            if cmd.info.pattern == cmd_name || cmd.info.alias.as_deref() == Some(&cmd_name) {
                println!(
                    "[debug:dispatch] matched command '{}', executing handler...",
                    cmd.info.pattern
                );
                if (cmd.info.owner_only || !cmd.info.public) && !msg.is_from_me {
                    println!("[debug:dispatch] rejected: owner_only / non-public");
                    return Ok(());
                }
                if cmd.info.group_only && !msg.is_group {
                    let _ = msg.reply("This command can only be used in groups.").await;
                    return Ok(());
                }
                if cmd.info.private_only && msg.is_group {
                    let _ = msg.reply("This command can only be used in private chats.").await;
                    return Ok(());
                }

                let m = Arc::new(msg);
                let res = (cmd.func)(m, args).await;
                if let Err(ref e) = res {
                    eprintln!("[debug:dispatch] command '{}' error: {:?}", cmd.info.pattern, e);
                } else {
                    println!("[debug:dispatch] command '{}' finished ok", cmd.info.pattern);
                }
                return res;
            }
        }

        println!("[debug:dispatch] no command matching '{}'", cmd_name);
        Ok(())
    }
}

pub fn init_plugins(prefix: &str, bot_name: &str) -> CommandManager {
    let mut cmds = system::commands();
    cmds.extend(media::commands());
    let pfx = prefix.to_string();
    let bname = bot_name.to_string();

    let mut infos: Vec<CommandInfo> = cmds.iter().map(|c| c.info.clone()).collect();
    infos.push(CommandInfo {
        pattern: "menu".to_string(),
        alias: Some("help".to_string()),
        description: "Display command menu".to_string(),
        usage: "".to_string(),
        public: true,
        owner_only: false,
        group_only: false,
        private_only: false,
        hide_command: true,
        category: "general".to_string(),
    });

    let shared_infos = Arc::new(infos);

    let menu_cmd = Command::new("menu", {
        let shared_infos = shared_infos.clone();
        let default_prefix = pfx.clone();
        let default_bot_name = bname.clone();
        move |m, _args| {
            let shared_infos = shared_infos.clone();
            let default_prefix = default_prefix.clone();
            let default_bot_name = default_bot_name.clone();
            async move {
                let pfx = m.session_prefix.as_deref().unwrap_or(&default_prefix);
                let bname = m.session_bot_name.as_deref().unwrap_or(&default_bot_name);
                let text = generate_menu(&shared_infos, pfx, bname, &m.session_name, m.is_from_me);
                m.reply(text).await?;
                Ok(())
            }
        }
    })
    .alias("help")
    .desc("Display the bot menu and available commands")
    .category("general")
    .public(true)
    .hidecommand(true);

    cmds.push(menu_cmd);

    CommandManager {
        prefix: pfx,
        bot_name: bname,
        commands: Arc::new(cmds),
    }
}
