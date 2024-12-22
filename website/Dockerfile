# ベースイメージ
FROM rust:latest

# 作業ディレクトリ
WORKDIR /app

# 必要なファイルをコピー
COPY . .

# 依存関係のダウンロードとビルドキャッシュの活用
RUN cargo fetch

# リリースビルド
RUN cargo build --release

# 実行時の軽量化のために最小限のイメージを作成
FROM debian:bullseye-slim

# 必要なパッケージをインストール
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# 作業ディレクトリ
WORKDIR /app

# Rustのバイナリをコピー
COPY --from=0 /app/target/release/website /app/

# 実行コマンド
CMD ["/app/target/release/website"]
