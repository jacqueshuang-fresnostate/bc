#!/bin/sh
set -eu

export PORT="${BACKEND_PORT:-8080}"
export RUST_LOG="${RUST_LOG:-info}"
BACKEND_STARTUP_TIMEOUT_SECONDS="${BACKEND_STARTUP_TIMEOUT_SECONDS:-60}"

case "${PORT}" in
    ''|*[!0-9]*)
        echo "BACKEND_PORT 必须是数字，当前值：${PORT}"
        exit 1
        ;;
esac

case "${BACKEND_STARTUP_TIMEOUT_SECONDS}" in
    ''|*[!0-9]*)
        echo "BACKEND_STARTUP_TIMEOUT_SECONDS 必须是数字，当前值：${BACKEND_STARTUP_TIMEOUT_SECONDS}"
        exit 1
        ;;
esac

sed "s/__BACKEND_PORT__/${PORT}/g" /etc/nginx/nginx.conf > /tmp/bc-nginx.conf
cp /tmp/bc-nginx.conf /etc/nginx/nginx.conf

backend_pid=""
nginx_pid=""

stop_services() {
    echo "收到停止信号，正在关闭服务"
    if [ -n "${nginx_pid:-}" ]; then
        nginx -s quit 2>/dev/null || true
        kill -TERM "${nginx_pid}" 2>/dev/null || true
        wait "${nginx_pid}" 2>/dev/null || true
    fi
    if [ -n "${backend_pid:-}" ]; then
        kill -TERM "${backend_pid}" 2>/dev/null || true
        wait "${backend_pid}" 2>/dev/null || true
    fi
}

wait_for_backend() {
    elapsed=0
    echo "等待后端健康检查通过，最长等待：${BACKEND_STARTUP_TIMEOUT_SECONDS} 秒"
    while [ "${elapsed}" -lt "${BACKEND_STARTUP_TIMEOUT_SECONDS}" ]; do
        if ! kill -0 "${backend_pid}" 2>/dev/null; then
            set +e
            wait "${backend_pid}"
            backend_status="$?"
            set -e
            echo "后端服务启动失败，退出码：${backend_status}"
            exit "${backend_status}"
        fi

        if curl -fsS "http://127.0.0.1:${PORT}/api/health" >/dev/null 2>&1; then
            echo "后端健康检查通过"
            return 0
        fi

        sleep 1
        elapsed=$((elapsed + 1))
    done

    echo "后端服务在 ${BACKEND_STARTUP_TIMEOUT_SECONDS} 秒内未通过健康检查"
    stop_services
    exit 1
}

monitor_services() {
    while :; do
        if ! kill -0 "${backend_pid}" 2>/dev/null; then
            set +e
            wait "${backend_pid}"
            backend_status="$?"
            set -e
            echo "后端服务已退出，正在关闭 Nginx，后端退出码：${backend_status}"
            nginx -s quit 2>/dev/null || true
            kill -TERM "${nginx_pid}" 2>/dev/null || true
            wait "${nginx_pid}" 2>/dev/null || true
            exit "${backend_status}"
        fi

        if ! kill -0 "${nginx_pid}" 2>/dev/null; then
            set +e
            wait "${nginx_pid}"
            nginx_status="$?"
            set -e
            echo "Nginx 前端网关已退出，正在关闭后端，Nginx 退出码：${nginx_status}"
            kill -TERM "${backend_pid}" 2>/dev/null || true
            wait "${backend_pid}" 2>/dev/null || true
            exit "${nginx_status}"
        fi

        sleep 2
    done
}

trap 'stop_services; exit 143' INT TERM

echo "启动后端服务，端口：${PORT}"
/usr/local/bin/bc-backend &
backend_pid="$!"

wait_for_backend

echo "启动 Nginx 前端网关，端口：80"
nginx -g "daemon off;" &
nginx_pid="$!"

monitor_services
