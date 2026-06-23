#!/bin/sh
set -eu

export PORT="${BACKEND_PORT:-8080}"
export RUST_LOG="${RUST_LOG:-info}"
BACKEND_STARTUP_TIMEOUT_SECONDS="${BACKEND_STARTUP_TIMEOUT_SECONDS:-60}"
BACKEND_STARTUP_LOG_INTERVAL_SECONDS="${BACKEND_STARTUP_LOG_INTERVAL_SECONDS:-2}"

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

case "${BACKEND_STARTUP_LOG_INTERVAL_SECONDS}" in
    ''|*[!0-9]*)
        echo "BACKEND_STARTUP_LOG_INTERVAL_SECONDS 必须是数字，当前值：${BACKEND_STARTUP_LOG_INTERVAL_SECONDS}"
        exit 1
        ;;
esac

if [ "${BACKEND_STARTUP_LOG_INTERVAL_SECONDS}" -eq 0 ]; then
    echo "BACKEND_STARTUP_LOG_INTERVAL_SECONDS 必须大于 0"
    exit 1
fi

sed "s/__BACKEND_PORT__/${PORT}/g" /etc/nginx/nginx.conf > /tmp/bc-nginx.conf
cp /tmp/bc-nginx.conf /etc/nginx/nginx.conf

backend_pid=""
nginx_pid=""
last_health_body_file="/tmp/bc-backend-health-body.txt"
last_health_error_file="/tmp/bc-backend-health-error.txt"

log_runtime_config() {
    if [ -n "${DATABASE_URL:-}" ]; then
        database_state="已配置"
    else
        database_state="未配置"
    fi
    if [ -n "${REDIS_URL:-}" ]; then
        redis_state="已配置"
    else
        redis_state="未配置"
    fi

    echo "容器启动配置：后端端口=${PORT}，健康检查等待=${BACKEND_STARTUP_TIMEOUT_SECONDS}秒，等待日志间隔=${BACKEND_STARTUP_LOG_INTERVAL_SECONDS}秒，RUST_LOG=${RUST_LOG}，DATABASE_URL=${database_state}，REDIS_URL=${redis_state}"
    echo "Nginx 配置已渲染，/api/ 将转发到 127.0.0.1:${PORT}"
}

backend_process_state() {
    if [ -n "${backend_pid:-}" ] && [ -r "/proc/${backend_pid}/status" ]; then
        state_line="$(grep '^State:' "/proc/${backend_pid}/status" 2>/dev/null || true)"
        rss_line="$(grep '^VmRSS:' "/proc/${backend_pid}/status" 2>/dev/null || true)"
        echo "后端进程状态：pid=${backend_pid} ${state_line:-State: 未知} ${rss_line:-VmRSS: 未知}"
    else
        echo "后端进程状态：pid=${backend_pid:-未启动}，/proc 状态不可读"
    fi
}

log_health_wait_progress() {
    elapsed="$1"
    curl_status="$2"
    http_status="$3"
    remaining=$((BACKEND_STARTUP_TIMEOUT_SECONDS - elapsed))
    error_message=""
    response_message=""

    if [ -s "${last_health_error_file}" ]; then
        error_message="$(tr '\n' ' ' < "${last_health_error_file}" | sed 's/[[:space:]]\{1,\}/ /g')"
    fi
    if [ -s "${last_health_body_file}" ]; then
        response_message="$(head -c 240 "${last_health_body_file}" | tr '\n' ' ' | sed 's/[[:space:]]\{1,\}/ /g')"
    fi

    echo "等待后端健康检查中：已等待 ${elapsed}/${BACKEND_STARTUP_TIMEOUT_SECONDS} 秒，剩余 ${remaining} 秒，curl退出码=${curl_status}，HTTP状态=${http_status}"
    if [ -n "${error_message}" ]; then
        echo "最近一次健康检查错误：${error_message}"
    fi
    if [ -n "${response_message}" ]; then
        echo "最近一次健康检查响应：${response_message}"
    fi
    backend_process_state
}

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
            backend_process_state
            exit "${backend_status}"
        fi

        set +e
        http_status="$(curl -sS -m 2 -o "${last_health_body_file}" -w "%{http_code}" "http://127.0.0.1:${PORT}/api/health" 2>"${last_health_error_file}")"
        curl_status="$?"
        set -e
        if [ "${curl_status}" -eq 0 ] && [ "${http_status}" = "200" ]; then
            echo "后端健康检查通过"
            return 0
        fi

        if [ "$((elapsed % BACKEND_STARTUP_LOG_INTERVAL_SECONDS))" -eq 0 ]; then
            log_health_wait_progress "${elapsed}" "${curl_status}" "${http_status}"
        fi

        sleep 1
        elapsed=$((elapsed + 1))
    done

    echo "后端服务在 ${BACKEND_STARTUP_TIMEOUT_SECONDS} 秒内未通过健康检查"
    log_health_wait_progress "${elapsed}" "${curl_status:-未执行}" "${http_status:-000}"
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

log_runtime_config
echo "启动后端服务，端口：${PORT}"
/usr/local/bin/bc-backend &
backend_pid="$!"
echo "后端进程已启动，pid=${backend_pid}"

wait_for_backend

echo "启动 Nginx 前端网关，端口：80"
nginx -g "daemon off;" &
nginx_pid="$!"

monitor_services
