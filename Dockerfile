# syntax=docker/dockerfile:1

FROM node:22-bookworm-slim AS admin-build
WORKDIR /workspace/admin

COPY admin/package*.json ./
RUN npm ci

COPY admin/ ./
RUN npm run build

FROM rust:1.92-slim-bookworm AS backend-build
WORKDIR /workspace/backend

COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/migrations ./migrations
COPY backend/src ./src
RUN cargo build --release

FROM nginx:1.27-bookworm AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

COPY docker/nginx.conf /etc/nginx/nginx.conf
COPY docker/entrypoint.sh /usr/local/bin/bc-entrypoint.sh
RUN chmod +x /usr/local/bin/bc-entrypoint.sh

COPY --from=admin-build /workspace/admin/dist /usr/share/nginx/html
COPY --from=backend-build /workspace/backend/target/release/bc-backend /usr/local/bin/bc-backend

ENV BACKEND_PORT=8080
ENV RUST_LOG=info

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -fsS http://127.0.0.1/api/health >/dev/null || exit 1

ENTRYPOINT ["/usr/local/bin/bc-entrypoint.sh"]
