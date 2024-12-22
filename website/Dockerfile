# ベースイメージ
FROM rust:latest as builder

# 作業ディレクトリ
WORKDIR /website

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

# ライブラリキャッシュを更新 (追加)
RUN ldconfig

# 作業ディレクトリ
WORKDIR /website

# Rustのバイナリをコピー (ここを修正)
COPY --from=builder /website/target/release/website /website/

# 実行権限を付与
RUN chmod +x ./website

# 実行コマンド
CMD ["./website"]  # ここも修正。websiteだけで良い