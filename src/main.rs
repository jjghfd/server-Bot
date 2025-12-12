use azalea::prelude::*;
use anyhow::Result;
use regex::Regex;
use reqwest::Client as HttpClient;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

#[tokio::main]
async fn main() {
    let config = load_config().unwrap();

    let account = if config.bot.is_offline {
        Account::offline(&config.bot.username)
    } else {
        // 这里可以添加在线登录的逻辑
        Account::offline(&config.bot.username)
    };

    ClientBuilder::new()
        .set_handler(handle)
        .start(account, &config.bot.server_address)
        .await
        .unwrap();
}

#[derive(Deserialize, Debug)]
struct Config {
    bot: BotConfig,
    bluemap: BluemapConfig,
}

#[derive(Deserialize, Debug)]
struct BotConfig {
    username: String,
    server_address: String,
    is_offline: bool,
}

#[derive(Deserialize, Debug)]
struct BluemapConfig {
    api_url: String,
}

#[derive(Clone, Component)]
pub struct State {
    super_ops: Vec<String>,
    ops: Vec<String>,
    http_client: HttpClient,
    config: Config,
}

impl Default for State {
    fn default() -> Self {
        let config = load_config().unwrap();
        // 默认超级超管
        let super_ops = vec![
            "NOI_zl".to_string(),
            "Mc＿MintyCool".to_string()
        ];
        
        Self {
            super_ops,
            ops: Vec::new(),
            http_client: HttpClient::new(),
            config,
        }
    }
}

impl State {
    // 检查是否为超级超管
    fn is_super_op(&self, player: &str) -> bool {
        self.super_ops.contains(&player.to_string())
    }
    
    // 检查是否为超管（包括超级超管）
    fn is_op(&self, player: &str) -> bool {
        self.is_super_op(player) || self.ops.contains(&player.to_string())
    }
    
    // 添加超管
    fn add_op(&mut self, player: &str) {
        if !self.is_op(player) {
            self.ops.push(player.to_string());
        }
    }
    
    // 移除超管
    fn remove_op(&mut self, player: &str) {
        if let Some(index) = self.ops.iter().position(|p| p == player) {
            self.ops.remove(index);
        }
    }
}

fn load_config() -> Result<Config> {
    let config_path = Path::new("config.toml");
    let config_content = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_content)?;
    Ok(config)
}

static COMMAND_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^%([a-zA-Z0-9_]+)\s*(.*)$").unwrap()
});

async fn handle(bot: Client, event: Event, mut state: State) -> Result<()> {
    match event {
        Event::Chat(m) => {
            let (sender, content) = m.split_sender_and_content();
            if sender.is_none() {
                return Ok(());
            }

            let sender_name = sender.unwrap();
            if let Some(captures) = COMMAND_REGEX.captures(&content) {
                let command = captures.get(1).unwrap().as_str();
                let args = captures.get(2).unwrap().as_str().trim();

                match command {
                    "开盒" => handle_open_box(&bot, sender_name, args, &state).await?,
                    "tpa" => handle_tpa(&bot, sender_name, args, &state).await?,
                    "设置传送点" => handle_set_home(&bot, sender_name, args, &state).await?,
                    "op" => handle_op(&bot, sender_name, args, &mut state).await?,
                    "deop" => handle_deop(&bot, sender_name, args, &mut state).await?,
                    "op查询" => handle_op_query(&bot, sender_name, &state).await?,
                    _ => bot.chat(format!("未知命令: {}", command)),
                }
            }
        },
        _ => {}
    }

    Ok(())
}

async fn handle_open_box(bot: &Client, sender: &str, args: &str, state: &State) -> Result<()> {
    if args.is_empty() {
        bot.chat("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 请输入玩家名字，格式: %开盒 [名字]");
        return Ok(());
    }

    let player_name = args;
    // 调用BlueMap API获取玩家位置
    match get_player_position(state, player_name).await {
        Ok((x, y, z)) => {
            bot.chat(format!("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] {}玩家目前位置在 {} {} {}", player_name, x, y, z));
        },
        Err(e) => {
            bot.chat(format!("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 获取玩家位置失败: {:?}", e));
        }
    }
    Ok(())
}

async fn get_player_position(state: &State, player_name: &str) -> Result<(i32, i32, i32)> {
    let api_url = format!("{}/players", state.config.bluemap.api_url);
    let response = state.http_client
        .get(&api_url)
        .send()
        .await?;
    
    let players: serde_json::Value = response.json().await?;
    
    if let Some(players_array) = players.as_array() {
        for player in players_array {
            if let Some(name) = player.get("name").and_then(|n| n.as_str()) {
                if name == player_name {
                    let x = player.get("position").and_then(|p| p.get("x").and_then(|x| x.as_f64())).unwrap_or(0.0) as i32;
                    let y = player.get("position").and_then(|p| p.get("y").and_then(|y| y.as_f64())).unwrap_or(0.0) as i32;
                    let z = player.get("position").and_then(|p| p.get("z").and_then(|z| z.as_f64())).unwrap_or(0.0) as i32;
                    return Ok((x, y, z));
                }
            }
        }
    }
    
    Err(anyhow::anyhow!("玩家未找到"))
}

async fn handle_tpa(bot: &Client, sender: &str, args: &str, state: &State) -> Result<()> {
    if !state.is_op(sender) {
        bot.chat("&#f877f8[&#df8af0樱&#c79ee7花&#aeb1df雪&#95c5d7机&#7cd8cf器&#64ecc6人&#4bffbe] &#47fac5您&#44f5cc暂&#40f0d3无&#3decdb管&#39e7e2理&#36e2e9权&#32ddf0限");
        return Ok(());
    }

    match args {
        "me" => {
            bot.chat(format!("/tpa {}", sender));
            bot.chat("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] &#55f7c1正&#4afdc1在&#47f9c7t&#44f5cdp&#41f1d3a&#3eedd8请&#3be9de接&#38e5e4受&#35e1ea请&#32ddf0求");
        },
        "you" => {
            bot.chat("/tpa here");
            bot.chat("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] &#55f7c1正&#4afdc1在&#47f9c7t&#44f5cdp&#41f1d3a&#3eedd8请&#3be9de接&#38e5e4受&#35e1ea请&#32ddf0求");
        },
        _ => {
            bot.chat("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 请使用正确的tpa指令，格式: %tpa me 或 %tpa you");
        }
    }
    Ok(())
}

async fn handle_set_home(bot: &Client, sender: &str, args: &str, state: &State) -> Result<()> {
    if args.is_empty() {
        bot.chat("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 请输入传送点名字，格式: %设置传送点 [传送点名字]");
        return Ok(());
    }

    let home_name = args;
    
    if !state.is_op(sender) {
        bot.chat("&#f877f8[&#df8af0樱&#c79ee7花&#aeb1df雪&#95c5d7机&#7cd8cf器&#64ecc6人&#4bffbe] &#47fac5您&#44f5cc暂&#40f0d3无&#3decdb管&#39e7e2理&#36e2e9权&#32ddf0限");
        return Ok(());
    }

    // 先传送到玩家那里
    bot.chat(format!("/tpa {}", sender));
    bot.chat("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] &#55f7c1正&#4afdc1在&#47f9c7t&#44f5cdp&#41f1d3a&#3eedd8请&#3be9de接&#38e5e4受&#35e1ea请&#32ddf0求");
    // 这里应该等待传送完成，然后设置家
    // 目前先直接设置
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    bot.chat(format!("/sethome {}", home_name));
    bot.chat(format!("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] &#55f7c1已&#4afdc1设&#47f9c7置&#44f5cd传&#41f1d3送&#3eedd8点&#3be9de{}", home_name));
    
    Ok(())
}

// 处理添加op命令
async fn handle_op(bot: &Client, sender: &str, args: &str, state: &mut State) -> Result<()> {
    if args.is_empty() {
        bot.chat("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 请输入要添加的玩家名字，格式: %op [玩家名字]");
        return Ok(());
    }
    
    // 只有超级超管可以添加op
    if !state.is_super_op(sender) {
        bot.chat("&#f877f8[&#df8af0樱&#c79ee7花&#aeb1df雪&#95c5d7机&#7cd8cf器&#64ecc6人&#4bffbe] &#47fac5您&#44f5cc暂&#40f0d3无&#3decdb管&#39e7e2理&#36e2e9权&#32ddf0限");
        return Ok(());
    }
    
    let player_name = args;
    
    if state.is_op(player_name) {
        bot.chat(format!("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 玩家 {} 已经是op或超级超管", player_name));
        return Ok(());
    }
    
    state.add_op(player_name);
    bot.chat(format!("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 已成功将玩家 {} 添加为op", player_name));
    
    Ok(())
}

// 处理移除op命令
async fn handle_deop(bot: &Client, sender: &str, args: &str, state: &mut State) -> Result<()> {
    if args.is_empty() {
        bot.chat("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 请输入要移除的玩家名字，格式: %deop [玩家名字]");
        return Ok(());
    }
    
    // 只有超级超管可以移除op
    if !state.is_super_op(sender) {
        bot.chat("&#f877f8[&#df8af0樱&#c79ee7花&#aeb1df雪&#95c5d7机&#7cd8cf器&#64ecc6人&#4bffbe] &#47fac5您&#44f5cc暂&#40f0d3无&#3decdb管&#39e7e2理&#36e2e9权&#32ddf0限");
        return Ok(());
    }
    
    let player_name = args;
    
    if state.is_super_op(player_name) {
        bot.chat(format!("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 无法移除超级超管 {}", player_name));
        return Ok(());
    }
    
    if !state.ops.contains(&player_name.to_string()) {
        bot.chat(format!("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 玩家 {} 不是op", player_name));
        return Ok(());
    }
    
    state.remove_op(player_name);
    bot.chat(format!("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 已成功将玩家 {} 移除op权限", player_name));
    
    Ok(())
}

// 处理op查询命令
async fn handle_op_query(bot: &Client, sender: &str, state: &State) -> Result<()> {
    // 只有op以上权限可以查询
    if !state.is_op(sender) {
        bot.chat("&#f877f8[&#df8af0樱&#c79ee7花&#aeb1df雪&#95c5d7机&#7cd8cf器&#64ecc6人&#4bffbe] &#47fac5您&#44f5cc暂&#40f0d3无&#3decdb管&#39e7e2理&#36e2e9权&#32ddf0限");
        return Ok(());
    }
    
    let super_ops_list = state.super_ops.join(", ");
    let ops_list = if state.ops.is_empty() {
        "无".to_string()
    } else {
        state.ops.join(", ")
    };
    
    bot.chat(format!("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 超级超管：{}", super_ops_list));
    bot.chat(format!("&#f877f8[&#e487f1樱&#cf97ea花&#bba7e4雪&#a7b7dd机&#92c7d6器&#7ed7cf人&#6ae7c8] 超管：{}", ops_list));
    
    Ok(())
}
