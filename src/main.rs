use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// GitHub API token
    #[arg(short, long, env = "GITHUB_TOKEN")]
    token: String,

    /// Team configuration file (JSON)
    #[arg(short = 'c', long, env = "TEAMS_CONFIG", default_value = "teams.json")]
    teams_config: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Repository {
    name: String,
    full_name: String,
    language: Option<String>,
    clone_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Team {
    name: String,
    organization: String,
    repositories: Vec<String>, // repository names without org prefix
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TeamsConfig {
    teams: Vec<Team>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeStats {
    production_lines: u64,
    test_lines: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LanguageStats {
    language: String,
    stats: CodeStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReportData {
    repository_stats: HashMap<String, HashMap<String, CodeStats>>, // repo_name -> language -> stats
    team_stats: HashMap<String, HashMap<String, CodeStats>>,       // team_name -> language -> stats
    organization_stats: HashMap<String, CodeStats>,                // language -> stats
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists (ignore errors if file doesn't exist)
    let _ = dotenv::dotenv();

    let args = Args::parse();

    println!("GitHub Code Counter");

    // Initialize GitHub client
    let github_client = GitHubClient::new(&args.token);

    // Load team configuration
    let teams_config = load_teams_config(&args.teams_config)?;

    // Collect unique repositories specified in teams
    let mut target_repositories: HashSet<String> = HashSet::new();
    for team in &teams_config.teams {
        for repo_name in &team.repositories {
            let full_name = format!("{}/{}", team.organization, repo_name);
            target_repositories.insert(full_name);
        }
    }

    println!("Target repositories: {:?}", target_repositories);

    // Fetch only the specified repositories
    let mut all_repositories = Vec::new();
    for target_repo in &target_repositories {
        let parts: Vec<&str> = target_repo.split('/').collect();
        if parts.len() != 2 {
            continue;
        }
        let (owner, repo_name) = (parts[0], parts[1]);

        println!("Fetching repository: {}", target_repo);
        match github_client.get_single_repository(owner, repo_name).await {
            Ok(repository) => {
                all_repositories.push(repository);
                println!("✓ Successfully fetched: {}", target_repo);
            }
            Err(e) => {
                println!("✗ Error fetching {}: {}", target_repo, e);
                continue;
            }
        }
    }
    println!("Found {} target repositories", all_repositories.len());

    // Process each repository
    let mut report_data = ReportData {
        repository_stats: HashMap::new(),
        team_stats: HashMap::new(),
        organization_stats: HashMap::new(),
    };

    for repo in all_repositories {
        if let Some(language) = &repo.language {
            println!("Processing repository: {} ({})", repo.full_name, language);

            // Clone and analyze repository
            let stats = analyze_repository(&repo, &args.token).await?;

            // Update repository stats using full_name for uniqueness
            report_data
                .repository_stats
                .entry(repo.full_name.clone())
                .or_default()
                .insert(language.clone(), stats.clone());

            // Update organization stats
            let org_stats = report_data
                .organization_stats
                .entry(language.clone())
                .or_insert_with(|| CodeStats {
                    production_lines: 0,
                    test_lines: 0,
                });
            org_stats.production_lines += stats.production_lines;
            org_stats.test_lines += stats.test_lines;

            // Update team stats if configured
            for team in &teams_config.teams {
                // Check if this repository belongs to this team
                let team_full_name = format!("{}/{}", team.organization, repo.name);
                if repo.full_name == team_full_name && team.repositories.contains(&repo.name) {
                    let team_stats = report_data
                        .team_stats
                        .entry(team.name.clone())
                        .or_default()
                        .entry(language.clone())
                        .or_insert_with(|| CodeStats {
                            production_lines: 0,
                            test_lines: 0,
                        });
                    team_stats.production_lines += stats.production_lines;
                    team_stats.test_lines += stats.test_lines;
                }
            }
        }
    }

    // Display results
    display_report(&report_data);

    Ok(())
}

struct GitHubClient {
    client: reqwest::Client,
    token: String,
}

impl GitHubClient {
    fn new(token: &str) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            token: token.to_string(),
        }
    }

    async fn get_single_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "ghcount")
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        if response.status().is_success() {
            let mut repository: Repository = response.json().await?;
            // Ensure full_name is set correctly
            repository.full_name = format!("{}/{}", owner, repo);
            Ok(repository)
        } else {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            
            match status.as_u16() {
                401 => anyhow::bail!("認証エラー: GitHubトークンが無効です。適切な権限を持つPersonal Access Tokenを設定してください。"),
                403 => anyhow::bail!("アクセス拒否: リポジトリ {}/{} にアクセスする権限がありません。プライベートリポジトリの場合は適切な権限が必要です。", owner, repo),
                404 => anyhow::bail!("リポジトリが見つかりません: {}/{}。リポジトリ名が正しいか、アクセス権限があるか確認してください。", owner, repo),
                _ => anyhow::bail!("GitHub API エラー ({}): {}", status, error_text),
            }
        }
    }
}

async fn analyze_repository(repo: &Repository, token: &str) -> Result<CodeStats> {
    use regex::Regex;
    use std::fs;
    use std::process::Command;
    use walkdir::WalkDir;

    // Create a temporary directory for cloning
    let temp_dir = format!("/tmp/ghcount_{}", repo.name);

    // Remove existing directory if it exists
    let _ = fs::remove_dir_all(&temp_dir);

    // Clone the repository with authentication for private repositories
    let authenticated_url = if repo.clone_url.starts_with("https://github.com/") {
        repo.clone_url.replace("https://github.com/", &format!("https://{}@github.com/", token))
    } else {
        repo.clone_url.clone()
    };

    let output = Command::new("git")
        .args(["clone", "--depth", "1", &authenticated_url, &temp_dir])
        .output()?;

    if !output.status.success() {
        let error_output = String::from_utf8_lossy(&output.stderr);
        if error_output.contains("Authentication failed") || error_output.contains("access denied") {
            anyhow::bail!("認証エラー: プライベートリポジトリ {} のクローンに失敗しました。GitHubトークンに適切な権限があることを確認してください。", repo.name);
        } else {
            anyhow::bail!("リポジトリのクローンに失敗: {} - {}", repo.name, error_output);
        }
    }

    let language = repo.language.as_deref().unwrap_or("Unknown");
    let mut stats = CodeStats {
        production_lines: 0,
        test_lines: 0,
    };

    // Define file extensions and test patterns by language
    let (extensions, test_patterns) = get_language_config(language);

    // Pre-compile regexes for test patterns
    let test_regexes: Vec<Regex> = test_patterns
        .iter()
        .filter_map(|pattern| Regex::new(pattern).ok())
        .collect();

    // Walk through all files in the repository
    for entry in WalkDir::new(&temp_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            let path_str = path.to_string_lossy().to_lowercase();

            // Check if this is a source file for the detected language
            if extensions.iter().any(|ext| path_str.ends_with(ext))
                && let Ok(content) = fs::read_to_string(path)
            {
                let line_count = count_code_lines(&content);

                // Determine if this is a test file
                let is_test_file = test_regexes.iter().any(|regex| regex.is_match(&path_str));

                if is_test_file {
                    stats.test_lines += line_count;
                } else {
                    stats.production_lines += line_count;
                }
            }
        }
    }

    // Clean up temporary directory
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(stats)
}

fn get_language_config(language: &str) -> (Vec<&str>, Vec<&str>) {
    match language.to_lowercase().as_str() {
        "rust" => (
            vec![".rs"],
            vec![r"test", r"tests/", r"_test\.rs$", r"test_.*\.rs$"],
        ),
        "javascript" | "typescript" => (
            vec![".js", ".ts", ".jsx", ".tsx"],
            vec![
                r"test",
                r"tests/",
                r"spec/",
                r"__tests__/",
                r"\.test\.",
                r"\.spec\.",
            ],
        ),
        "python" => (
            vec![".py"],
            vec![r"test", r"tests/", r"test_.*\.py$", r".*_test\.py$"],
        ),
        "java" => (
            vec![".java"],
            vec![r"test", r"tests/", r"Test\.java$", r".*Test\.java$"],
        ),
        "go" => (vec![".go"], vec![r"_test\.go$"]),
        "c" | "c++" => (
            vec![".c", ".cpp", ".cc", ".cxx", ".h", ".hpp"],
            vec![r"test", r"tests/"],
        ),
        _ => (vec![".txt"], vec![r"test"]), // fallback
    }
}

fn count_code_lines(content: &str) -> u64 {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with('#')
        })
        .count() as u64
}

fn load_teams_config(path: &str) -> Result<TeamsConfig> {
    let content = std::fs::read_to_string(path)?;
    let teams_config: TeamsConfig = serde_json::from_str(&content)?;
    Ok(teams_config)
}

fn display_report(data: &ReportData) {
    println!("\n=== Repository Statistics ===");
    for (repo_name, lang_stats) in &data.repository_stats {
        println!("\nRepository: {}", repo_name);
        for (language, stats) in lang_stats {
            println!(
                "  {} - Production: {}, Test: {}",
                language, stats.production_lines, stats.test_lines
            );
        }
    }

    if !data.team_stats.is_empty() {
        println!("\n=== Team Statistics ===");
        for (team_name, lang_stats) in &data.team_stats {
            println!("\nTeam: {}", team_name);
            for (language, stats) in lang_stats {
                println!(
                    "  {} - Production: {}, Test: {}",
                    language, stats.production_lines, stats.test_lines
                );
            }
        }
    }

    println!("\n=== Organization Statistics ===");
    for (language, stats) in &data.organization_stats {
        println!(
            "{} - Production: {}, Test: {}",
            language, stats.production_lines, stats.test_lines
        );
    }
}