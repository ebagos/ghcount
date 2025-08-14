# GitHub Code Counter (ghcount)

GitHubリポジトリのプロダクションコードとテストコードの行数を分析し、チーム・組織レベルで統計を提供するコマンドラインツールです。

## 🚀 機能

- **プロダクション vs テストコード分析**: テストファイルを自動検出し、プロダクションコードとテストコードを分離して統計を表示
- **複数プログラミング言語対応**: Rust、Java、TypeScript/JavaScript、Python、Go、C/C++をサポート
- **チーム・組織レベル集計**: 個別リポジトリ、チーム、組織全体での統計を表示
- **clocとの統合**: オプションでclocを使用したより詳細な分析が可能
- **言語フィルタリング**: 特定のプログラミング言語のみを対象とした分析
- **柔軟な設定**: 環境変数とコマンドライン引数の両方をサポート

## 📦 インストール

### 前提条件

- Rust 1.24以降
- Git
- GitHub Personal Access Token
- cloc（オプション、より詳細な分析のため）

### インストール手順

1. リポジトリをクローン:
```bash
git clone https://github.com/your-org/ghcount.git
cd ghcount
```

2. ビルド:
```bash
cargo build --release
```

3. （オプション）clocをインストール:
```bash
# Ubuntu/Debian
sudo apt-get install cloc

# macOS
brew install cloc

# その他の方法については https://github.com/AlDanial/cloc を参照
```

## 🔧 設定

### 1. GitHub Personal Access Token の取得

1. [GitHub Settings > Developer settings > Personal access tokens](https://github.com/settings/tokens)にアクセス
2. "Generate new token (classic)"をクリック
3. 必要な権限を選択:
   - パブリックリポジトリのみ: `public_repo`
   - プライベートリポジトリも含む: `repo`
4. トークンを生成し、安全な場所に保存

### 2. 環境変数の設定

`.env.sample`を`.env`にコピーして設定:

```bash
cp .env.sample .env
```

`.env`ファイルを編集:

```bash
# 必須: GitHub Personal Access Token
GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

# オプション: チーム設定ファイル
TEAMS_CONFIG=teams.json

# オプション: デバッグモード（詳細な行数情報を表示）
DEBUG_MODE=false

# オプション: clocを使用（より詳細な分析）
USE_CLOC=false

# オプション: 分析対象言語の絞り込み（カンマ区切り）
LANGUAGES=Java,TypeScript,Python
```

### 3. チーム設定ファイル（teams.json）

```json
{
  "teams": [
    {
      "name": "backend",
      "organization": "your-org",
      "repositories": ["api-server", "database-service", "auth-service"]
    },
    {
      "name": "frontend",
      "organization": "your-org", 
      "repositories": ["web-app", "mobile-app", "admin-dashboard"]
    },
    {
      "name": "infrastructure",
      "organization": "your-org",
      "repositories": ["terraform-configs", "ci-cd-pipelines"]
    }
  ]
}
```

## 🎯 使用方法

### 基本的な使用法

```bash
# 環境変数を使用
cargo run

# コマンドライン引数を使用
cargo run -- --token ghp_xxx --teams-config teams.json
```

### 高度な使用法

```bash
# clocを使用した詳細分析
cargo run -- --use-cloc

# 特定の言語のみを分析
cargo run -- --languages Java,TypeScript

# デバッグモードで詳細情報を表示
cargo run -- --debug

# 全オプションを組み合わせ
cargo run -- --token ghp_xxx --use-cloc --debug --languages Rust,Python
```

### 環境変数での設定

```bash
# 環境変数で一括設定
export GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
export USE_CLOC=true
export DEBUG_MODE=true
export LANGUAGES=Java,TypeScript,Python

cargo run
```

## 📊 出力例

### 標準出力

```
=== Repository Statistics ===

Repository: your-org/api-server
  Java - Production: 15420, Test: 8750

Repository: your-org/web-app  
  TypeScript - Production: 12300, Test: 6890

=== Team Statistics ===

Team: backend
  Java - Production: 28450, Test: 15200

Team: frontend
  TypeScript - Production: 25600, Test: 14300

=== Organization Statistics ===
Java - Production: 28450, Test: 15200
TypeScript - Production: 25600, Test: 14300
```

### cloc使用時の詳細出力

```
=== Detailed Cloc Analysis ===

--- Repository: your-org/api-server ---

=== Cloc Analysis Results ===
cloc version 1.98

Language                Files        Blank      Comment         Code
======================================================================
Java                      206         4812         8572        18479
XML                        14           28          515         3927
Properties                  4           21           29           66
----------------------------------------------------------------------
SUM                       224         4861         9116        22472

=== Production vs Test Code Breakdown ===
Category                    Lines
===================================
Production Code             13729
Test Code                    4750
-----------------------------------
Total Code                  18479

=== Code Distribution ===
Production: 74.3% (13729 lines)
Test:       25.7% (4750 lines)
```

## 🔍 サポートされる言語とテストパターン

| 言語 | ファイル拡張子 | テストパターン |
|------|----------------|----------------|
| Rust | `.rs` | `test/`, `tests/`, `*_test.rs`, `test_*.rs` |
| Java | `.java` | `/test/`, `/tests/`, `*Test.java`, `*Tests.java` |
| TypeScript/JavaScript | `.ts`, `.tsx`, `.js`, `.jsx` | `__tests__/`, `*.test.*`, `*.spec.*`, `test/`, `spec/` |
| Python | `.py` | `test/`, `tests/`, `test_*.py`, `*_test.py` |
| Go | `.go` | `*_test.go` |
| C/C++ | `.c`, `.cpp`, `.h`, `.hpp` | `test/`, `tests/` |

## ⚙️ コマンドラインオプション

```
Usage: ghcount [OPTIONS] --token <TOKEN>

Options:
  -t, --token <TOKEN>                GitHub API token [env: GITHUB_TOKEN]
  -c, --teams-config <TEAMS_CONFIG>  Team configuration file (JSON) [env: TEAMS_CONFIG] [default: teams.json]
  -d, --debug                        Enable debug mode [env: DEBUG_MODE]
      --use-cloc                     Use cloc for counting [env: USE_CLOC]
      --languages <LANGUAGES>        Filter repositories by programming languages [env: LANGUAGES]
  -h, --help                         Print help
  -V, --version                      Print version
```

## 🧪 テスト

### ユニットテストの実行

```bash
cargo test --bin ghcount
```

### 統合テストの実行

```bash
cargo test --test integration_tests
```

### 全テストの実行

```bash
cargo test
```

## 🔧 開発

### 開発環境の構築

1. リポジトリをクローン
2. `.env.sample`を`.env`にコピーして設定
3. `teams.json`を作成
4. テストを実行して動作確認

### コードの構造

```
src/
├── main.rs                 # メインアプリケーション
├── lib.rs                  # ライブラリ関数（今後追加予定）
tests/
├── integration_tests.rs    # 統合テスト
```

### 主要な機能モジュール

- **GitHub API統合**: リポジトリ情報の取得
- **コード行数分析**: 言語別の詳細分析
- **cloc統合**: 外部ツールとの連携
- **設定管理**: 環境変数とファイルベース設定
- **レポート生成**: 統計結果の整形と表示

## 🚨 トラブルシューティング

### よくある問題

1. **認証エラー**
   ```
   認証エラー: GitHubトークンが無効です
   ```
   → GitHub Personal Access Tokenが正しく設定されているか確認

2. **リポジトリアクセスエラー**
   ```
   アクセス拒否: リポジトリにアクセスする権限がありません
   ```
   → トークンに適切な権限があるか確認

3. **clocエラー**
   ```
   clocがインストールされていません
   ```
   → clocをインストールするか、`--use-cloc`オプションを外して実行

4. **設定ファイルエラー**
   ```
   No such file or directory
   ```
   → `teams.json`ファイルが正しいパスに存在するか確認

### デバッグ方法

```bash
# デバッグモードで詳細情報を表示
cargo run -- --debug

# 環境変数の確認
echo $GITHUB_TOKEN
echo $TEAMS_CONFIG

# 設定ファイルの確認
cat teams.json
```

## 🤝 コントリビューション

1. フォークを作成
2. フィーチャーブランチを作成: `git checkout -b feature/new-feature`
3. 変更をコミット: `git commit -am 'Add new feature'`
4. ブランチにプッシュ: `git push origin feature/new-feature`
5. Pull Requestを作成

### 開発ガイドライン

- 新機能には必ずテストを追加
- コードスタイルは`cargo fmt`で統一
- リンターは`cargo clippy`でチェック
- 日本語コメントと英語コメントの併用OK

## 📄 ライセンス

このプロジェクトはMITライセンスの下で公開されています。詳細は[LICENSE](LICENSE)ファイルを参照してください。

## 🙏 謝辞

- [cloc](https://github.com/AlDanial/cloc) - 高度なコード分析ツール
- [tokio](https://tokio.rs/) - 非同期ランタイム
- [clap](https://clap.rs/) - コマンドライン引数パーサー
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTPクライアント