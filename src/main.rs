//! # GitHub Code Counter (ghcount)
//! 
//! GitHubリポジトリのプロダクションコードとテストコードの行数を分析し、
//! チーム・組織レベルで統計を提供するコマンドラインツール。
//! 
//! ## 主要機能
//! - プロダクション vs テストコード分析
//! - 複数プログラミング言語対応
//! - チーム・組織レベル集計
//! - clocとの統合による詳細分析
//! - 言語フィルタリング

use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// コマンドライン引数の定義
/// 
/// 全ての引数は対応する環境変数からも設定可能
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// GitHub API token
    #[arg(short, long, env = "GITHUB_TOKEN")]
    token: String,

    /// Team configuration file (JSON)
    #[arg(short = 'c', long, env = "TEAMS_CONFIG", default_value = "teams.json")]
    teams_config: String,

    /// Enable debug mode to show non-code lines (comments, empty lines, strings)
    #[arg(short = 'd', long, env = "DEBUG_MODE")]
    debug: bool,

    /// Use cloc for counting instead of built-in analyzer
    #[arg(long, env = "USE_CLOC")]
    use_cloc: bool,

    /// Filter repositories by programming languages (comma-separated)
    /// Example: "Java,TypeScript,Python"
    #[arg(long, env = "LANGUAGES", value_delimiter = ',')]
    languages: Option<Vec<String>>,
}

/// GitHubリポジトリの情報を表現する構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Repository {
    name: String,
    full_name: String,
    language: Option<String>,
    clone_url: String,
}

/// チームの設定を表現する構造体
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

/// コード統計情報を格納する構造体
/// 
/// プロダクションコード、テストコード、コメント、空行、文字列行の
/// 行数を個別に追跡する
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeStats {
    production_lines: u64,
    test_lines: u64,
    comment_lines: u64,
    empty_lines: u64,
    string_lines: u64,
}

#[derive(Debug, Clone)]
struct LineStats {
    code_lines: u64,
    comment_lines: u64,
    empty_lines: u64,
    string_lines: u64,
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
    cloc_results: HashMap<String, ClocResult>,                     // repo_name -> cloc result
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClocLanguageResult {
    language: String,
    files: u64,
    blank_lines: u64,
    comment_lines: u64,
    code_lines: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClocResult {
    header: String,
    languages: Vec<ClocLanguageResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClocTestResult {
    test_code_lines: u64,
    test_comment_lines: u64,
    test_blank_lines: u64,
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
        cloc_results: HashMap::new(),
    };

    for repo in all_repositories {
        if let Some(language) = &repo.language {
            // Apply language filter if specified
            if let Some(ref filter_languages) = args.languages {
                let language_matches = filter_languages.iter().any(|filter_lang| {
                    language.to_lowercase() == filter_lang.to_lowercase()
                });
                
                if !language_matches {
                    println!("Skipping repository: {} ({}) - not in language filter", repo.full_name, language);
                    continue;
                }
            }

            println!("Processing repository: {} ({})", repo.full_name, language);

            // Clone and analyze repository
            let (stats, cloc_result_opt) = if args.use_cloc {
                println!("Using cloc for analysis...");
                let (stats, cloc_result) = analyze_repository_with_cloc(&repo, &args.token).await?;
                (stats, Some(cloc_result))
            } else {
                let stats = analyze_repository(&repo, &args.token, args.debug).await?;
                (stats, None)
            };

            // Store cloc result if available
            if let Some(cloc_result) = cloc_result_opt {
                report_data.cloc_results.insert(repo.full_name.clone(), cloc_result);
            }

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
                    comment_lines: 0,
                    empty_lines: 0,
                    string_lines: 0,
                });
            org_stats.production_lines += stats.production_lines;
            org_stats.test_lines += stats.test_lines;
            org_stats.comment_lines += stats.comment_lines;
            org_stats.empty_lines += stats.empty_lines;
            org_stats.string_lines += stats.string_lines;

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
                            comment_lines: 0,
                            empty_lines: 0,
                            string_lines: 0,
                        });
                    team_stats.production_lines += stats.production_lines;
                    team_stats.test_lines += stats.test_lines;
                    team_stats.comment_lines += stats.comment_lines;
                    team_stats.empty_lines += stats.empty_lines;
                    team_stats.string_lines += stats.string_lines;
                }
            }
        }
    }

    // Display results
    display_report(&report_data, args.debug, args.use_cloc, args.languages.as_ref());

    Ok(())
}

struct GitHubClient {
    client: reqwest::Client,
    token: String,
}

impl GitHubClient {
    /// 新しいGitHubクライアントを作成する
    fn new(token: &str) -> Self {
        let client = reqwest::Client::new();
        Self {
            client,
            token: token.to_string(),
        }
    }

    /// GitHub API から単一のリポジトリ情報を取得する
    /// 
    /// # 引数
    /// * `owner` - リポジトリの所有者（組織名またはユーザー名）
    /// * `repo` - リポジトリ名
    /// 
    /// # 戻り値
    /// リポジトリ情報または詳細なエラー情報
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

async fn analyze_repository(repo: &Repository, token: &str, debug_mode: bool) -> Result<CodeStats> {
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
        comment_lines: 0,
        empty_lines: 0,
        string_lines: 0,
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
                let line_stats = if debug_mode {
                    count_lines_detailed(&content, language)
                } else {
                    let code_count = count_code_lines(&content);
                    LineStats {
                        code_lines: code_count,
                        comment_lines: 0,
                        empty_lines: 0,
                        string_lines: 0,
                    }
                };

                // Determine if this is a test file
                let is_test_file = test_regexes.iter().any(|regex| regex.is_match(&path_str));

                if is_test_file {
                    stats.test_lines += line_stats.code_lines;
                } else {
                    stats.production_lines += line_stats.code_lines;
                }
                
                if debug_mode {
                    stats.comment_lines += line_stats.comment_lines;
                    stats.empty_lines += line_stats.empty_lines;
                    stats.string_lines += line_stats.string_lines;
                }
            }
        }
    }

    // Clean up temporary directory
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(stats)
}

/// clocを使用してリポジトリを分析する関数
/// 
/// このアプローチは外部ツールclocを使用して詳細な言語統計を提供します。
/// 
/// # 引数
/// * `repo` - 分析対象のGitHubリポジトリ情報
/// * `token` - GitHub認証用のPersonal Access Token
/// 
/// # 戻り値
/// CodeStatsとClocResultのタプル（成功時）、またはエラー
/// 
/// # エラー
/// * リポジトリクローンの失敗
/// * clocの実行エラー
/// * 認証エラー
async fn analyze_repository_with_cloc(repo: &Repository, token: &str) -> Result<(CodeStats, ClocResult)> {
    use std::fs;
    use std::process::Command;

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

    // Run cloc on the cloned repository
    let language = repo.language.as_deref().unwrap_or("Unknown");
    let (cloc_result, test_result) = run_cloc(&temp_dir, language)?;

    // Clean up temporary directory
    let _ = fs::remove_dir_all(&temp_dir);

    // Convert cloc result to CodeStats using test results
    let stats = convert_cloc_to_code_stats(&cloc_result, &test_result, language)?;

    Ok((stats, cloc_result))
}

/// clocコマンドを実行してコード統計を取得する
/// 
/// 2段階のアプローチを使用:
/// 1. 全ファイルの統計を取得
/// 2. テストディレクトリを除外してプロダクションコードのみの統計を取得
/// 3. 差分計算でテストコードの行数を算出
/// 
/// # 引数
/// * `directory` - 分析対象のディレクトリパス
/// * `_language` - 対象言語（現在は未使用、将来の拡張用）
/// 
/// # 戻り値
/// 全体統計とテスト統計のタプル
fn run_cloc(directory: &str, _language: &str) -> Result<(ClocResult, ClocTestResult)> {
    use std::process::Command;

    // Check if cloc is available
    let cloc_check = Command::new("cloc")
        .arg("--version")
        .output();

    if cloc_check.is_err() {
        anyhow::bail!("clocがインストールされていません。clocをインストールしてから再実行してください。");
    }

    // Run cloc with JSON output for all files
    let output = Command::new("cloc")
        .args([
            "--json",
            "--exclude-dir=.git,node_modules,target,build,dist,vendor",
            directory
        ])
        .output()?;

    if !output.status.success() {
        let error_output = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("clocの実行に失敗しました: {}", error_output);
    }

    let json_output = String::from_utf8_lossy(&output.stdout);
    let cloc_result = parse_cloc_json(&json_output)?;

    // Run cloc excluding test directories to get production code
    let production_result = run_cloc_production_only(directory)?;
    
    // Calculate test code by subtracting production from total
    let test_result = calculate_test_lines(&cloc_result, &production_result, _language)?;
    
    Ok((cloc_result, test_result))
}

/// プロダクションコードのみを対象としてclocを実行
/// 
/// テストディレクトリとテストファイルを除外してプロダクションコードの統計のみを取得します。
/// 複数のパターンを使用してテストファイルを正確に識別します。
/// 
/// # 引数
/// * `directory` - 分析対象のディレクトリパス
/// 
/// # 戻り値
/// プロダクションコードのみのcloc結果
fn run_cloc_production_only(directory: &str) -> Result<ClocResult> {
    use std::process::Command;

    // Run cloc excluding common test directories using --fullpath and --not-match-d
    let output = Command::new("cloc")
        .args([
            "--json",
            "--exclude-dir=.git,node_modules,target,build,dist,vendor",
            "--fullpath",
            "--not-match-d=(test|tests|spec|specs|__tests__|src/test|src/test/java|test/java|src/integrationTest|src/testFixtures|cypress|e2e)",
            "--not-match-f=\\.(test|spec)\\.(js|ts|jsx|tsx)$",
            directory
        ])
        .output()?;

    if !output.status.success() {
        let error_output = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("clocのプロダクション実行に失敗しました: {}", error_output);
    }

    let json_output = String::from_utf8_lossy(&output.stdout);
    parse_cloc_json(&json_output)
}

/// 全体統計からプロダクション統計を差し引いてテストコード行数を計算
/// 
/// 指定された言語のみを対象として、テストコードの行数を正確に計算します。
/// 全体の行数からプロダクションコードの行数を差し引くことで、
/// テストコードの実際の行数を算出します。
/// 
/// # 引数
/// * `total_result` - 全ファイルのcloc結果
/// * `production_result` - プロダクションコードのみのcloc結果
/// * `target_language` - 対象とする言語名
/// 
/// # 戻り値
/// テストコードの統計情報
fn calculate_test_lines(total_result: &ClocResult, production_result: &ClocResult, target_language: &str) -> Result<ClocTestResult> {
    // Get total lines for the target language only
    let mut total_code_lines = 0u64;
    let mut total_comment_lines = 0u64;
    let mut total_blank_lines = 0u64;

    for lang_result in &total_result.languages {
        if lang_result.language.to_lowercase() == target_language.to_lowercase() {
            total_code_lines += lang_result.code_lines;
            total_comment_lines += lang_result.comment_lines;
            total_blank_lines += lang_result.blank_lines;
        }
    }

    // Get production lines for the target language only
    let mut production_code_lines = 0u64;
    let mut production_comment_lines = 0u64;
    let mut production_blank_lines = 0u64;

    for lang_result in &production_result.languages {
        if lang_result.language.to_lowercase() == target_language.to_lowercase() {
            production_code_lines += lang_result.code_lines;
            production_comment_lines += lang_result.comment_lines;
            production_blank_lines += lang_result.blank_lines;
        }
    }

    // Calculate test lines by subtracting production from total for the target language
    let test_code_lines = total_code_lines.saturating_sub(production_code_lines);
    let test_comment_lines = total_comment_lines.saturating_sub(production_comment_lines);
    let test_blank_lines = total_blank_lines.saturating_sub(production_blank_lines);

    println!("  Production lines detected: {}", production_code_lines);
    println!("  Test lines calculated: {}", test_code_lines);

    Ok(ClocTestResult {
        test_code_lines,
        test_comment_lines,
        test_blank_lines,
    })
}

/// clocのJSON出力を解析してClocResult構造体に変換
/// 
/// clocコマンドの--jsonオプションで出力されるJSON形式の結果を
/// 構造化されたRustデータ型に変換します。
/// 
/// # 引数
/// * `json_str` - clocのJSON出力文字列
/// 
/// # 戻り値
/// 解析されたcloc結果
fn parse_cloc_json(json_str: &str) -> Result<ClocResult> {
    let json_value: serde_json::Value = serde_json::from_str(json_str)?;
    
    let mut languages = Vec::new();
    let mut header = "cloc output".to_string();

    if let Some(obj) = json_value.as_object() {
        for (key, value) in obj {
            if key == "header" {
                if let Some(header_obj) = value.as_object() {
                    if let Some(version) = header_obj.get("cloc_version") {
                        header = format!("cloc version {}", version.as_str().unwrap_or("unknown"));
                    }
                }
                continue;
            }
            
            if key == "SUM" {
                continue;
            }

            if let Some(lang_data) = value.as_object() {
                let language = key.clone();
                let files = lang_data.get("nFiles").and_then(|v| v.as_u64()).unwrap_or(0);
                let blank_lines = lang_data.get("blank").and_then(|v| v.as_u64()).unwrap_or(0);
                let comment_lines = lang_data.get("comment").and_then(|v| v.as_u64()).unwrap_or(0);
                let code_lines = lang_data.get("code").and_then(|v| v.as_u64()).unwrap_or(0);

                languages.push(ClocLanguageResult {
                    language,
                    files,
                    blank_lines,
                    comment_lines,
                    code_lines,
                });
            }
        }
    }

    Ok(ClocResult { header, languages })
}

/// cloc結果をCodeStats構造体に変換
/// 
/// cloc分析の結果とテスト行数の計算結果を組み合わせて、
/// アプリケーション標準のCodeStats構造体に変換します。
/// 指定された言語のみを対象として統計を計算します。
/// 
/// # 引数
/// * `cloc_result` - cloc分析の全体結果
/// * `test_result` - テストコード行数の計算結果
/// * `target_language` - 対象とする言語名
/// 
/// # 戻り値
/// 標準化されたコード統計
fn convert_cloc_to_code_stats(cloc_result: &ClocResult, test_result: &ClocTestResult, target_language: &str) -> Result<CodeStats> {
    let mut stats = CodeStats {
        production_lines: 0,
        test_lines: 0,
        comment_lines: 0,
        empty_lines: 0,
        string_lines: 0,
    };

    // Get code lines for the target language only
    let mut target_code_lines = 0u64;
    let mut target_comment_lines = 0u64;
    let mut target_blank_lines = 0u64;

    for lang_result in &cloc_result.languages {
        if lang_result.language.to_lowercase() == target_language.to_lowercase() {
            target_code_lines += lang_result.code_lines;
            target_comment_lines += lang_result.comment_lines;
            target_blank_lines += lang_result.blank_lines;
        }
    }

    // Subtract test lines from target language total to get production lines
    stats.test_lines = test_result.test_code_lines;
    stats.production_lines = target_code_lines.saturating_sub(test_result.test_code_lines);
    stats.comment_lines = target_comment_lines;
    stats.empty_lines = target_blank_lines;

    Ok(stats)
}

/// プログラミング言語に応じたファイル拡張子とテストパターンを取得
/// 
/// 各プログラミング言語に特有のファイル拡張子とテストファイルの識別パターンを
/// 定義しています。テストパターンは正規表現として使用されます。
/// 
/// # 引数
/// * `language` - プログラミング言語名（大文字小文字不問）
/// 
/// # 戻り値
/// ファイル拡張子のリストとテストパターンのリストのタプル
/// 
/// # サポート言語
/// - Rust: .rs ファイル、test/tests ディレクトリ、_test.rs パターン
/// - JavaScript/TypeScript: .js/.ts/.jsx/.tsx、様々なテストパターン
/// - Python: .py ファイル、test_ と _test パターン
/// - Java: .java ファイル、Test/Tests クラス、test ディレクトリ
/// - Go: .go ファイル、_test.go パターン
/// - C/C++: 各種拡張子、test ディレクトリ
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
            vec![r"src/test/", r"/test/", r"/tests/", r"Test[^/]*\.java$", r"[^/]*Test\.java$", r"[^/]*Tests\.java$"],
        ),
        "go" => (vec![".go"], vec![r"_test\.go$"]),
        "c" | "c++" => (
            vec![".c", ".cpp", ".cc", ".cxx", ".h", ".hpp"],
            vec![r"test", r"tests/"],
        ),
        _ => (vec![".txt"], vec![r"test"]), // fallback
    }
}

/// ファイル内容からコード行数をカウント（シンプル版）
/// 
/// 空行とコメント行を除外してコード行数のみをカウントします。
/// デバッグモードが無効の場合に使用される軽量な実装です。
/// 
/// # 引数
/// * `content` - ファイルの内容
/// 
/// # 戻り値
/// コード行数
fn count_code_lines(content: &str) -> u64 {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with('#')
        })
        .count() as u64
}

/// ファイル内容から詳細な行統計を取得
/// 
/// コード行、コメント行、空行、文字列行を分類してカウントします。
/// デバッグモードが有効の場合に使用され、詳細な統計情報を提供します。
/// 
/// # 引数
/// * `content` - ファイルの内容
/// * `language` - プログラミング言語（将来の拡張用、現在は共通ロジック）
/// 
/// # 戻り値
/// 詳細な行統計情報
fn count_lines_detailed(content: &str, language: &str) -> LineStats {
    let mut stats = LineStats {
        code_lines: 0,
        comment_lines: 0,
        empty_lines: 0,
        string_lines: 0,
    };

    let mut in_multiline_comment = false;
    let mut in_multiline_string = false;

    for line in content.lines() {
        let trimmed = line.trim();
        
        if trimmed.is_empty() {
            stats.empty_lines += 1;
            continue;
        }

        let mut is_comment_line = false;
        let mut is_string_line = false;
        let mut is_code_line = false;

        // Language-specific comment patterns
        match language.to_lowercase().as_str() {
            "rust" => {
                if in_multiline_comment {
                    is_comment_line = true;
                    if trimmed.contains("*/") {
                        in_multiline_comment = false;
                    }
                } else if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                    is_comment_line = true;
                } else if trimmed.starts_with("/*") {
                    is_comment_line = true;
                    in_multiline_comment = !trimmed.contains("*/");
                } else {
                    is_code_line = true;
                    // Check for string literals
                    if trimmed.contains("\"") || trimmed.contains("'") || trimmed.contains("r\"") || trimmed.contains("r#\"") {
                        is_string_line = true;
                    }
                }
            },
            "javascript" | "typescript" => {
                if in_multiline_comment {
                    is_comment_line = true;
                    if trimmed.contains("*/") {
                        in_multiline_comment = false;
                    }
                } else if trimmed.starts_with("//") {
                    is_comment_line = true;
                } else if trimmed.starts_with("/*") {
                    is_comment_line = true;
                    in_multiline_comment = !trimmed.contains("*/");
                } else {
                    is_code_line = true;
                    if trimmed.contains("\"") || trimmed.contains("'") || trimmed.contains("`") {
                        is_string_line = true;
                    }
                }
            },
            "python" => {
                if in_multiline_string {
                    is_string_line = true;
                    if trimmed.contains("\"\"\"") || trimmed.contains("'''") {
                        in_multiline_string = false;
                    }
                } else if trimmed.starts_with("#") {
                    is_comment_line = true;
                } else if trimmed.contains("\"\"\"") || trimmed.contains("'''") {
                    if trimmed.matches("\"\"\"").count() == 1 || trimmed.matches("'''").count() == 1 {
                        is_string_line = true;
                        in_multiline_string = true;
                    } else {
                        is_string_line = true;
                    }
                } else {
                    is_code_line = true;
                    if trimmed.contains("\"") || trimmed.contains("'") {
                        is_string_line = true;
                    }
                }
            },
            "java" => {
                if in_multiline_comment {
                    is_comment_line = true;
                    if trimmed.contains("*/") {
                        in_multiline_comment = false;
                    }
                } else if trimmed.starts_with("//") {
                    is_comment_line = true;
                } else if trimmed.starts_with("/*") {
                    is_comment_line = true;
                    in_multiline_comment = !trimmed.contains("*/");
                } else {
                    is_code_line = true;
                    if trimmed.contains("\"") || trimmed.contains("'") {
                        is_string_line = true;
                    }
                }
            },
            "go" => {
                if in_multiline_comment {
                    is_comment_line = true;
                    if trimmed.contains("*/") {
                        in_multiline_comment = false;
                    }
                } else if trimmed.starts_with("//") {
                    is_comment_line = true;
                } else if trimmed.starts_with("/*") {
                    is_comment_line = true;
                    in_multiline_comment = !trimmed.contains("*/");
                } else {
                    is_code_line = true;
                    if trimmed.contains("\"") || trimmed.contains("'") || trimmed.contains("`") {
                        is_string_line = true;
                    }
                }
            },
            "c" | "c++" => {
                if in_multiline_comment {
                    is_comment_line = true;
                    if trimmed.contains("*/") {
                        in_multiline_comment = false;
                    }
                } else if trimmed.starts_with("//") || trimmed.starts_with("#") {
                    is_comment_line = true;
                } else if trimmed.starts_with("/*") {
                    is_comment_line = true;
                    in_multiline_comment = !trimmed.contains("*/");
                } else {
                    is_code_line = true;
                    if trimmed.contains("\"") || trimmed.contains("'") {
                        is_string_line = true;
                    }
                }
            },
            _ => {
                // Generic fallback
                if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("--") {
                    is_comment_line = true;
                } else {
                    is_code_line = true;
                    if trimmed.contains("\"") || trimmed.contains("'") {
                        is_string_line = true;
                    }
                }
            }
        }

        // Count the line type
        if is_comment_line {
            stats.comment_lines += 1;
        } else if is_code_line {
            stats.code_lines += 1;
            if is_string_line {
                stats.string_lines += 1;
            }
        }
    }

    stats
}

/// チーム設定ファイル（JSON）を読み込み、パースする
/// 
/// teams.json ファイルからチーム、組織、リポジトリの紐づけ情報を
/// 読み込み、構造体に変換します。
/// 
/// # 引数
/// * `path` - チーム設定ファイルのパス
/// 
/// # 戻り値
/// パースされたチーム設定
/// 
/// # エラー
/// * ファイル読み込みエラー
/// * JSONパースエラー
fn load_teams_config(path: &str) -> Result<TeamsConfig> {
    let content = std::fs::read_to_string(path)?;
    let teams_config: TeamsConfig = serde_json::from_str(&content)?;
    Ok(teams_config)
}

/// cloc分析結果をフォーマットして表示
/// 
/// clocコマンドの結果を見やすい表形式で表示します。
/// 言語別統計、プロダクション対テストの割合、総計情報を表示します。
/// 
/// # 引数
/// * `cloc_result` - cloc分析の結果
/// * `repo_stats` - プロダクション対テストの統計（オプション）
/// * `language_filter` - 表示対象言語のフィルタ（オプション）
fn display_cloc_result(cloc_result: &ClocResult, repo_stats: Option<&CodeStats>, language_filter: Option<&Vec<String>>) {
    println!("\n=== Cloc Analysis Results ===");
    println!("{}", cloc_result.header);
    println!("\n{:<20} {:>8} {:>12} {:>12} {:>12}", "Language", "Files", "Blank", "Comment", "Code");
    println!("{}", "=".repeat(70));
    
    let mut total_files = 0;
    let mut total_blank = 0;
    let mut total_comment = 0;
    let mut total_code = 0;
    let mut displayed_languages_count = 0;

    for lang in &cloc_result.languages {
        // Apply language filter if specified
        let should_display = if let Some(filter_languages) = language_filter {
            filter_languages.iter().any(|filter_lang| {
                lang.language.to_lowercase() == filter_lang.to_lowercase()
            })
        } else {
            true
        };

        if should_display {
            println!(
                "{:<20} {:>8} {:>12} {:>12} {:>12}",
                lang.language, lang.files, lang.blank_lines, lang.comment_lines, lang.code_lines
            );
            total_files += lang.files;
            total_blank += lang.blank_lines;
            total_comment += lang.comment_lines;
            total_code += lang.code_lines;
            displayed_languages_count += 1;
        }
    }

    if displayed_languages_count > 1 {
        println!("{}", "-".repeat(70));
        println!(
            "{:<20} {:>8} {:>12} {:>12} {:>12}",
            "SUM", total_files, total_blank, total_comment, total_code
        );
    }

    // Display production vs test breakdown if available
    if let Some(stats) = repo_stats {
        println!("\n=== Production vs Test Code Breakdown ===");
        println!("{:<20} {:>12}", "Category", "Lines");
        println!("{}", "=".repeat(35));
        println!("{:<20} {:>12}", "Production Code", stats.production_lines);
        println!("{:<20} {:>12}", "Test Code", stats.test_lines);
        println!("{}", "-".repeat(35));
        let total_code_lines = stats.production_lines + stats.test_lines;
        println!("{:<20} {:>12}", "Total Code", total_code_lines);
        
        if total_code_lines > 0 {
            let production_percentage = (stats.production_lines as f64 / total_code_lines as f64) * 100.0;
            let test_percentage = (stats.test_lines as f64 / total_code_lines as f64) * 100.0;
            println!("\n=== Code Distribution ===");
            println!("Production: {:.1}% ({} lines)", production_percentage, stats.production_lines);
            println!("Test:       {:.1}% ({} lines)", test_percentage, stats.test_lines);
        }
    }
}

/// メインレポートを表示（リポジトリ、チーム、組織レベルの統計）
/// 
/// 分析結果を階層的に表示します:
/// 1. リポジトリ別統計
/// 2. チーム別統計（存在する場合）
/// 3. 組織全体統計
/// 
/// デバッグモードが有効の場合、コメント、空行、文字列行も表示します。
/// 
/// # 引数
/// * `data` - 集計されたレポートデータ
/// * `debug_mode` - 詳細情報表示モード
/// * `use_cloc` - cloc使用フラグ（現在は未使用）
/// * `language_filter` - 表示対象言語のフィルタ（オプション）
fn display_report(data: &ReportData, debug_mode: bool, use_cloc: bool, language_filter: Option<&Vec<String>>) {
    println!("\n=== Repository Statistics ===");
    for (repo_name, lang_stats) in &data.repository_stats {
        println!("\nRepository: {}", repo_name);
        for (language, stats) in lang_stats {
            if debug_mode {
                println!(
                    "  {} - Production: {}, Test: {}, Comments: {}, Empty: {}, Strings: {}",
                    language, 
                    stats.production_lines, 
                    stats.test_lines, 
                    stats.comment_lines, 
                    stats.empty_lines, 
                    stats.string_lines
                );
            } else {
                println!(
                    "  {} - Production: {}, Test: {}",
                    language, stats.production_lines, stats.test_lines
                );
            }
        }
    }

    if !data.team_stats.is_empty() {
        println!("\n=== Team Statistics ===");
        for (team_name, lang_stats) in &data.team_stats {
            println!("\nTeam: {}", team_name);
            for (language, stats) in lang_stats {
                if debug_mode {
                    println!(
                        "  {} - Production: {}, Test: {}, Comments: {}, Empty: {}, Strings: {}",
                        language, 
                        stats.production_lines, 
                        stats.test_lines, 
                        stats.comment_lines, 
                        stats.empty_lines, 
                        stats.string_lines
                    );
                } else {
                    println!(
                        "  {} - Production: {}, Test: {}",
                        language, stats.production_lines, stats.test_lines
                    );
                }
            }
        }
    }

    println!("\n=== Organization Statistics ===");
    for (language, stats) in &data.organization_stats {
        if debug_mode {
            println!(
                "{} - Production: {}, Test: {}, Comments: {}, Empty: {}, Strings: {}",
                language, 
                stats.production_lines, 
                stats.test_lines, 
                stats.comment_lines, 
                stats.empty_lines, 
                stats.string_lines
            );
        } else {
            println!(
                "{} - Production: {}, Test: {}",
                language, stats.production_lines, stats.test_lines
            );
        }
    }

    // Display cloc detailed results if available
    if use_cloc && !data.cloc_results.is_empty() {
        println!("\n=== Detailed Cloc Analysis ===");
        for (repo_name, cloc_result) in &data.cloc_results {
            println!("\n--- Repository: {} ---", repo_name);
            
            // Get repository stats for production/test breakdown
            let repo_stats = data.repository_stats.get(repo_name)
                .and_then(|lang_stats| lang_stats.values().next());
            
            display_cloc_result(cloc_result, repo_stats, language_filter);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_language_config() {
        // Test Rust configuration
        let (extensions, patterns) = get_language_config("rust");
        assert_eq!(extensions, vec![".rs"]);
        assert!(patterns.contains(&r"test"));
        assert!(patterns.contains(&r"_test\.rs$"));

        // Test Java configuration
        let (extensions, patterns) = get_language_config("java");
        assert_eq!(extensions, vec![".java"]);
        assert!(patterns.contains(&r"/test/"));
        assert!(patterns.contains(&r"[^/]*Test\.java$"));

        // Test TypeScript configuration
        let (extensions, patterns) = get_language_config("typescript");
        assert!(extensions.contains(&".ts"));
        assert!(extensions.contains(&".tsx"));
        assert!(patterns.contains(&r"__tests__/"));
        assert!(patterns.contains(&r"\.test\."));

        // Test unknown language fallback
        let (extensions, patterns) = get_language_config("unknown");
        assert_eq!(extensions, vec![".txt"]);
        assert_eq!(patterns, vec![r"test"]);
    }

    #[test]
    fn test_count_code_lines() {
        let sample_code = r#"// This is a comment
fn main() {
    println!("Hello, world!");
    
    // Another comment
    let x = 5;
}
"#;
        let count = count_code_lines(sample_code);
        assert_eq!(count, 4); // Should exclude comment lines and empty lines
    }

    #[test]
    fn test_count_lines_detailed_rust() {
        let sample_code = r#"// Comment
fn main() {
    println!("Hello");
    /* Block comment */
    let x = 5;
    
    let s = "string literal";
}
"#;
        let stats = count_lines_detailed(sample_code, "rust");
        assert_eq!(stats.code_lines, 5); // fn main, {, println, let x, let s, }
        assert_eq!(stats.comment_lines, 2); // // comment and /* comment */
        assert_eq!(stats.empty_lines, 1);
        assert_eq!(stats.string_lines, 2); // println and let s lines
    }

    #[test]
    fn test_count_lines_detailed_java() {
        let sample_code = r#"// Java comment
public class Test {
    public static void main(String[] args) {
        System.out.println("Hello");
        /* Multi-line
           comment */
        int x = 5;
    }
}
"#;
        let stats = count_lines_detailed(sample_code, "java");
        assert!(stats.code_lines >= 5); // class, method, println, int, closing braces
        assert!(stats.comment_lines >= 3); // Single comment + multi-line comment
    }

    #[test]
    fn test_parse_cloc_json() {
        let sample_json = r#"{
            "header": {
                "cloc_version": "1.98"
            },
            "Rust": {
                "nFiles": 1,
                "blank": 10,
                "comment": 5,
                "code": 100
            },
            "SUM": {
                "blank": 10,
                "comment": 5,
                "code": 100,
                "nFiles": 1
            }
        }"#;

        let result = parse_cloc_json(sample_json).unwrap();
        assert!(result.header.contains("cloc version 1.98"));
        assert_eq!(result.languages.len(), 1);
        
        let rust_lang = &result.languages[0];
        assert_eq!(rust_lang.language, "Rust");
        assert_eq!(rust_lang.files, 1);
        assert_eq!(rust_lang.blank_lines, 10);
        assert_eq!(rust_lang.comment_lines, 5);
        assert_eq!(rust_lang.code_lines, 100);
    }

    #[test]
    fn test_convert_cloc_to_code_stats() {
        let cloc_result = ClocResult {
            header: "test".to_string(),
            languages: vec![
                ClocLanguageResult {
                    language: "Java".to_string(),
                    files: 10,
                    blank_lines: 100,
                    comment_lines: 50,
                    code_lines: 1000,
                },
                ClocLanguageResult {
                    language: "JavaScript".to_string(),
                    files: 5,
                    blank_lines: 50,
                    comment_lines: 25,
                    code_lines: 500,
                },
            ],
        };

        let test_result = ClocTestResult {
            test_code_lines: 200,
            test_comment_lines: 20,
            test_blank_lines: 30,
        };

        // Test for Java (should only count Java lines)
        let stats = convert_cloc_to_code_stats(&cloc_result, &test_result, "Java").unwrap();
        assert_eq!(stats.production_lines, 800); // 1000 - 200
        assert_eq!(stats.test_lines, 200);
        assert_eq!(stats.comment_lines, 50); // Only Java comments
        assert_eq!(stats.empty_lines, 100); // Only Java blank lines

        // Test for JavaScript (should only count JavaScript lines)
        let stats = convert_cloc_to_code_stats(&cloc_result, &test_result, "JavaScript").unwrap();
        assert_eq!(stats.production_lines, 300); // 500 - 200
        assert_eq!(stats.test_lines, 200);
        assert_eq!(stats.comment_lines, 25); // Only JavaScript comments
        assert_eq!(stats.empty_lines, 50); // Only JavaScript blank lines
    }

    #[test]
    fn test_calculate_test_lines() {
        let total_result = ClocResult {
            header: "total".to_string(),
            languages: vec![
                ClocLanguageResult {
                    language: "Java".to_string(),
                    files: 20,
                    blank_lines: 200,
                    comment_lines: 100,
                    code_lines: 2000,
                },
            ],
        };

        let production_result = ClocResult {
            header: "production".to_string(),
            languages: vec![
                ClocLanguageResult {
                    language: "Java".to_string(),
                    files: 15,
                    blank_lines: 150,
                    comment_lines: 80,
                    code_lines: 1500,
                },
            ],
        };

        let test_result = calculate_test_lines(&total_result, &production_result, "Java").unwrap();
        assert_eq!(test_result.test_code_lines, 500); // 2000 - 1500
        assert_eq!(test_result.test_comment_lines, 20); // 100 - 80
        assert_eq!(test_result.test_blank_lines, 50); // 200 - 150
    }

    #[test]
    fn test_load_teams_config() {
        // Create a temporary teams.json file for testing
        let temp_config = r#"{
            "teams": [
                {
                    "name": "backend",
                    "organization": "myorg",
                    "repositories": ["api", "database"]
                },
                {
                    "name": "frontend",
                    "organization": "myorg",
                    "repositories": ["web", "mobile"]
                }
            ]
        }"#;

        // Write to a temporary file
        std::fs::write("/tmp/test_teams.json", temp_config).unwrap();

        let config = load_teams_config("/tmp/test_teams.json").unwrap();
        assert_eq!(config.teams.len(), 2);
        assert_eq!(config.teams[0].name, "backend");
        assert_eq!(config.teams[0].repositories.len(), 2);
        assert_eq!(config.teams[1].name, "frontend");

        // Clean up
        std::fs::remove_file("/tmp/test_teams.json").ok();
    }

    #[test]
    fn test_language_matching_case_insensitive() {
        // Test case-insensitive language matching
        let (extensions, _) = get_language_config("RUST");
        assert_eq!(extensions, vec![".rs"]);

        let (extensions, _) = get_language_config("typescript");
        assert!(extensions.contains(&".ts"));

        let (extensions, _) = get_language_config("JavaScript");
        assert!(extensions.contains(&".js"));
    }
}