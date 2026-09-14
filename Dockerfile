# ---------- 前端构建 ----------
FROM node:20-alpine AS frontend-builder
WORKDIR /build
COPY package.json package-lock.json ./
RUN npm ci
COPY index.html vite.config.ts tailwind.config.js postcss.config.js tsconfig.json tsconfig.node.json ./
COPY src ./src
COPY public ./public
RUN npm run build

# ---------- 后端构建 ----------
FROM rust:1-bookworm AS backend-builder
WORKDIR /build
COPY server/Cargo.toml server/Cargo.lock ./
# 预先编译依赖以利用 Docker 层缓存
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/deps/asmr_db*
COPY server/src ./src
RUN touch src/main.rs && cargo build --release

# ---------- 运行时 ----------
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=backend-builder /build/target/release/asmr-db /app/asmr-db
COPY --from=frontend-builder /build/dist /app/dist

ENV ASMR_HOST=0.0.0.0 \
    ASMR_PORT=1421 \
    ASMR_DB_DIR=/data \
    ASMR_DIST_DIR=/app/dist

VOLUME ["/data"]
EXPOSE 1421

CMD ["/app/asmr-db"]
