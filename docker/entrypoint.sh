#!/bin/sh
set -eu

export PORT="${BACKEND_PORT:-8080}"
export RUST_LOG="${RUST_LOG:-info}"

case "${PORT}" in
    ''|*[!0-9]*)
        echo "BACKEND_PORT 必须是数字，当前值：${PORT}"
        exit 1
        ;;
esac

sed "s/__BACKEND_PORT__/${PORT}/g" /etc/nginx/nginx.conf > /tmp/bc-nginx.conf
cp /tmp/bc-nginx.conf /etc/nginx/nginx.conf

echo "启动后端服务，端口：${PORT}"
/usr/local/bin/bc-backend &
backend_pid="$!"

stop_services() {
    echo "收到停止信号，正在关闭服务"
    nginx -s quit 2>/dev/null || true
    kill -TERM "${backend_pid}" 2>/dev/null || true
    wait "${backend_pid}" 2>/dev/null || true
}

trap stop_services INT TERM

echo "启动 Nginx 前端网关，端口：80"
nginx -g "daemon off;"
nginx_status="$?"

kill -TERM "${backend_pid}" 2>/dev/null || true
wait "${backend_pid}" 2>/dev/null || true

exit "${nginx_status}"
