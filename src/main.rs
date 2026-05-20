use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::error::Error;
use std::time::Duration;

const DEFAULT_SERVER: &str = "https://cryptogenesis.duckdns.org";
const DEFAULT_AGENT_ID: &str = "codex-wallet-agent";
const DEFAULT_WALLET: &str = "0xa925FdD65a0f34bb415Bae1c57536Be33AbCfA92";

#[derive(Debug, Clone)]
struct Config {
    server: String,
    agent_id: String,
    wallet: String,
    mission_id: Option<String>,
    proof: String,
    dry_run: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct MissionSummary {
    id: String,
    title: Option<String>,
    status: Option<String>,
    reward_aigen: Option<u64>,
    submission_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ActiveMissions {
    missions: Vec<MissionSummary>,
}

#[derive(Debug, Serialize)]
struct SubmitPayload<'a> {
    submitter_agent_id: &'a str,
    submitter_wallet: &'a str,
    proof: &'a str,
    metadata: Value,
}

fn main() {
    if let Err(error) = run(parse_args()) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn parse_args() -> Config {
    let mut config = Config {
        server: DEFAULT_SERVER.to_string(),
        agent_id: DEFAULT_AGENT_ID.to_string(),
        wallet: DEFAULT_WALLET.to_string(),
        mission_id: None,
        proof: "https://github.com/Sikkra/aigen-rust-oabp-agent".to_string(),
        dry_run: false,
    };

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--server" => config.server = required_value("--server", args.next()),
            "--agent-id" => config.agent_id = required_value("--agent-id", args.next()),
            "--wallet" => config.wallet = required_value("--wallet", args.next()),
            "--mission" => config.mission_id = Some(required_value("--mission", args.next())),
            "--proof" => config.proof = required_value("--proof", args.next()),
            "--dry-run" => config.dry_run = true,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_help();
                std::process::exit(2);
            }
        }
    }

    config
}

fn required_value(flag: &str, value: Option<String>) -> String {
    value.unwrap_or_else(|| {
        eprintln!("{flag} requires a value");
        std::process::exit(2);
    })
}

fn print_help() {
    println!(
        "aigen-rust-oabp-agent\n\n\
         Usage:\n\
           cargo run -- --dry-run\n\
           cargo run -- --mission <mission_id> --proof <github_repo_url>\n\n\
         Options:\n\
           --server <url>       OABP server, default {DEFAULT_SERVER}\n\
           --agent-id <id>      Submitter agent id, default {DEFAULT_AGENT_ID}\n\
           --wallet <address>   Payout wallet, default project Base wallet\n\
           --mission <id>       Mission to read and submit; defaults to first open mission\n\
           --proof <text>       Proof URL or text to submit\n\
           --dry-run            Fetch active missions and one detail without posting"
    );
}

fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    let active = fetch_active_missions(&client, &config.server)?;
    println!("fetched {} active missions", active.missions.len());

    let selected = select_mission(&active.missions, config.mission_id.as_deref())
        .ok_or("no matching open mission found")?;
    println!(
        "selected mission {}: {}",
        selected.id,
        selected.title.as_deref().unwrap_or("(untitled)")
    );

    let detail = fetch_mission_detail(&client, &config.server, &selected.id)?;
    println!(
        "read mission detail {} ({} bytes)",
        selected.id,
        detail.to_string().len()
    );

    if config.dry_run {
        println!("dry run complete; not submitting");
        return Ok(());
    }

    let response = submit_solution(
        &client,
        &config.server,
        &selected.id,
        &config.agent_id,
        &config.wallet,
        &config.proof,
    )?;
    println!(
        "submission accepted: {}",
        serde_json::to_string_pretty(&response)?
    );
    Ok(())
}

fn api_url(server: &str, path: &str) -> String {
    format!(
        "{}/{}",
        server.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

fn fetch_active_missions(client: &Client, server: &str) -> Result<ActiveMissions, Box<dyn Error>> {
    let response = client.get(api_url(server, "/missions/active")).send()?;
    Ok(response.error_for_status()?.json()?)
}

fn fetch_mission_detail(
    client: &Client,
    server: &str,
    mission_id: &str,
) -> Result<Value, Box<dyn Error>> {
    let response = client
        .get(api_url(server, &format!("/missions/{mission_id}")))
        .send()?;
    Ok(response.error_for_status()?.json()?)
}

fn submit_solution(
    client: &Client,
    server: &str,
    mission_id: &str,
    agent_id: &str,
    wallet: &str,
    proof: &str,
) -> Result<Value, Box<dyn Error>> {
    let payload = SubmitPayload {
        submitter_agent_id: agent_id,
        submitter_wallet: wallet,
        proof,
        metadata: json!({
            "client": "aigen-rust-oabp-agent",
            "language": "rust",
            "http": "reqwest",
            "aip": "AIP-1",
            "operations": [
                "GET /missions/active",
                "GET /missions/{id}",
                "POST /missions/{id}/submit"
            ]
        }),
    };

    let response = client
        .post(api_url(server, &format!("/missions/{mission_id}/submit")))
        .json(&payload)
        .send()?;
    Ok(response.error_for_status()?.json()?)
}

fn select_mission<'a>(
    missions: &'a [MissionSummary],
    requested: Option<&str>,
) -> Option<&'a MissionSummary> {
    if let Some(requested_id) = requested {
        return missions.iter().find(|mission| mission.id == requested_id);
    }

    missions
        .iter()
        .filter(|mission| mission.status.as_deref().unwrap_or("open") == "open")
        .min_by_key(|mission| {
            (
                mission.submission_count.unwrap_or(u64::MAX),
                std::cmp::Reverse(mission.reward_aigen.unwrap_or(0)),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_url_trims_duplicate_slashes() {
        assert_eq!(
            api_url("https://example.com/", "/missions/active"),
            "https://example.com/missions/active"
        );
    }

    #[test]
    fn select_requested_mission_by_id() {
        let missions = vec![
            MissionSummary {
                id: "mis_a".to_string(),
                title: Some("A".to_string()),
                status: Some("open".to_string()),
                reward_aigen: Some(10),
                submission_count: Some(1),
            },
            MissionSummary {
                id: "mis_b".to_string(),
                title: Some("B".to_string()),
                status: Some("open".to_string()),
                reward_aigen: Some(500),
                submission_count: Some(0),
            },
        ];

        let selected = select_mission(&missions, Some("mis_a")).unwrap();
        assert_eq!(selected.id, "mis_a");
    }

    #[test]
    fn select_lowest_submission_highest_reward_open_mission() {
        let missions = vec![
            MissionSummary {
                id: "mis_closed".to_string(),
                title: None,
                status: Some("closed".to_string()),
                reward_aigen: Some(900),
                submission_count: Some(0),
            },
            MissionSummary {
                id: "mis_low".to_string(),
                title: None,
                status: Some("open".to_string()),
                reward_aigen: Some(50),
                submission_count: Some(0),
            },
            MissionSummary {
                id: "mis_high".to_string(),
                title: None,
                status: Some("open".to_string()),
                reward_aigen: Some(500),
                submission_count: Some(0),
            },
        ];

        let selected = select_mission(&missions, None).unwrap();
        assert_eq!(selected.id, "mis_high");
    }

    #[test]
    fn payload_contains_required_submitter_fields() {
        let payload = SubmitPayload {
            submitter_agent_id: "agent-1",
            submitter_wallet: "0xabc",
            proof: "https://github.com/example/repo",
            metadata: json!({"client": "test"}),
        };

        let value = serde_json::to_value(payload).unwrap();
        assert_eq!(value["submitter_agent_id"], "agent-1");
        assert_eq!(value["submitter_wallet"], "0xabc");
        assert_eq!(value["proof"], "https://github.com/example/repo");
    }
}
