# NextRouter - Improved Architecture

デュアルWANルーター設定スクリプトの改良版です。モジュラー設計により、保守性と拡張性を大幅に向上させました。

## アーキテクチャの改善点

### 🏗️ モジュラー設計
- **単一責任の原則**: 各モジュールが明確な役割を持つ
- **疎結合**: モジュール間の依存関係を最小限に抑制
- **再利用性**: 関数やユーティリティの再利用が容易

### 🛡️ エラーハンドリング
- **Strict Mode**: `set -euo pipefail` による厳格なエラー検出
- **バックアップ機能**: 設定変更前の自動バックアップ
- **復旧機能**: エラー時の自動復旧オプション
- **詳細ログ**: カラー付きログによる状況把握

### 🧪 テスト可能性
- **Dry Run モード**: 実際の変更なしでのテスト実行
- **単体テスト**: 各モジュールの独立したテスト
- **モック機能**: 外部コマンドのモック化

## ファイル構成

```
improved/
├── nextrouter.sh      # メインスクリプト（エントリーポイント）
├── config.sh          # 設定管理モジュール
├── utils.sh           # ユーティリティ関数群
├── args.sh            # 引数解析モジュール
├── network.sh         # ネットワーク設定モジュール
├── test.sh            # テストスイート
└── README.md          # このファイル
```

## 使用方法

### 基本的な使用方法
```bash
# 通常の実行
sudo ./nextrouter.sh --wan0=eth0 --wan1=eth1 --lan0=eth2 --local-ip=192.168.1.1/24

# Dry Run（実際の変更なし）
sudo ./nextrouter.sh --wan0=eth0 --wan1=eth1 --lan0=eth2 --local-ip=192.168.1.1/24 --dry-run

# ネットワーク設定のみ
sudo ./nextrouter.sh --mode=network --wan0=eth0 --wan1=eth1 --lan0=eth2 --local-ip=192.168.1.1/24

# 監視設定のみ
sudo ./nextrouter.sh --mode=monitoring
```

### 運用モード
- **full**: 完全セットアップ（ネットワーク + 監視）
- **network**: ネットワーク設定のみ
- **monitoring**: 監視設定のみ
- **status**: 現在の状態確認
- **restore**: バックアップからの復旧

### オプション
- `--dry-run`: 実際の変更を行わずテスト実行
- `--verbose`: 詳細ログ出力
- `--backup-dir=DIR`: バックアップディレクトリの指定
- `--force`: 確認プロンプトをスキップ

## テスト実行

```bash
# テストスイートの実行
./test.sh

# 個別テストの実行例（手動）
source config.sh
source utils.sh
validate_config
```

## 設定管理

### 設定ファイル
設定は `config.sh` 内の連想配列で管理：

```bash
# ネットワーク設定
declare -A INTERFACES=(
    [WAN0]="eth0"
    [WAN1]="eth1"
    [LAN0]="eth2"
    [LAN_IP]="192.168.1.1/24"
)

# システム設定  
declare -A CONFIG=(
    [DRY_RUN]="false"
    [VERBOSE]="false"
    [BACKUP_DIR]="/opt/nextrouter/backups"
    [LOG_LEVEL]="INFO"
)
```

### 設定の永続化
```bash
# 設定の保存
save_config "/etc/nextrouter/config"

# 設定の読み込み
load_config "/etc/nextrouter/config"
```

## エラーハンドリング

### バックアップ機能
```bash
# 自動バックアップ
backup_dir=$(create_backup)
log_info "Configuration backed up to: $backup_dir"

# 手動復旧
restore_backup "$backup_dir"
```

### エラー復旧
```bash
# エラー時の自動復旧
trap 'handle_error $? $LINENO' ERR

handle_error() {
    local exit_code=$1
    local line_number=$2
    log_error "Error occurred at line $line_number (exit code: $exit_code)"
    
    if [[ "${CONFIG[AUTO_RESTORE]}" == "true" ]] && [[ -n "$BACKUP_DIR" ]]; then
        log_info "Attempting automatic restore..."
        restore_backup "$BACKUP_DIR"
    fi
}
```

## ユーティリティ関数

### ログ機能
```bash
log_info "情報メッセージ"
log_warn "警告メッセージ"  
log_error "エラーメッセージ"
log_debug "デバッグメッセージ"
```

### コマンド実行
```bash
# Dry Run 対応のコマンド実行
execute "systemctl restart dhcpd"

# 出力付きでのコマンド実行
result=$(execute_with_output "ip route show")
```

### サービス管理
```bash
# サービス状態確認
if ! manage_service "status" "dhcpd"; then
    log_error "DHCP service is not running"
fi

# サービス再起動
manage_service "restart" "dhcpd"
```

## 拡張ポイント

### 新しいモジュールの追加
1. `modules/` ディレクトリに新しいファイルを作成
2. `config.sh` に設定項目を追加
3. `nextrouter.sh` にモジュール読み込みを追加

### カスタム検証の追加
```bash
# config.sh の validate_config 関数に追加
validate_custom_setting() {
    if [[ ! -f "${CONFIG[CUSTOM_CONFIG_FILE]}" ]]; then
        log_error "Custom config file not found"
        return 1
    fi
}
```

### 新しい運用モードの追加
```bash
# args.sh の parse_arguments 関数に追加
"custom")
    CONFIG[MODE]="custom"
    ;;
```

## 移行ガイド

元のモノリシックスクリプトからの移行手順：

1. **段階的移行**: 一度に全てを置き換えるのではなく、モジュールごとに移行
2. **テスト実行**: `--dry-run` モードで動作確認
3. **バックアップ**: 既存設定のバックアップを必ず作成
4. **検証**: テストスイートでの動作確認

## パフォーマンス

### 改善点
- **並列処理**: 独立したタスクの並列実行
- **キャッシュ**: 計算結果のキャッシュ
- **最適化**: 不要なコマンド実行の削減

### ベンチマーク例
```bash
# オリジナル版
time ./NextRouter.sh -w0=eth0 -w1=eth1 -l0=eth2 -lip=192.168.1.1/24
# real    2m30.123s

# 改良版
time ./improved/nextrouter.sh --wan0=eth0 --wan1=eth1 --lan0=eth2 --local-ip=192.168.1.1/24
# real    1m45.456s
```

## トラブルシューティング

### よくある問題

1. **権限エラー**
   ```bash
   sudo ./nextrouter.sh [options]
   ```

2. **設定検証エラー**
   ```bash
   ./nextrouter.sh --validate-only
   ```

3. **ネットワークインターフェース未検出**
   ```bash
   ip link show  # インターフェース名を確認
   ```

### ログファイル
- システムログ: `/var/log/nextrouter.log`
- デバッグログ: `/tmp/nextrouter-debug.log`

## 今後の改善予定

- [ ] Ansible Playbook 対応
- [ ] Docker コンテナ化
- [ ] Web UI インターフェース
- [ ] SNMP 監視対応
- [ ] クラウド連携機能
