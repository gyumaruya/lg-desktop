#!/bin/bash
# lg-desktop virtual desktop startup script
# Launches Xvfb + i3 + x11vnc + noVNC

set -e

RESOLUTION="${RESOLUTION:-1280x1024x24}"
DISPLAY_NUM="${DISPLAY_NUM:-1}"
VNC_PORT="${VNC_PORT:-5900}"
NOVNC_PORT="${NOVNC_PORT:-3000}"

export DISPLAY=":${DISPLAY_NUM}"

echo "[lg-desktop] Starting virtual display ${DISPLAY} at ${RESOLUTION}"
Xvfb "${DISPLAY}" -screen 0 "${RESOLUTION}" -ac +extension GLX +render -noreset &
XVFB_PID=$!
sleep 1

if ! kill -0 "$XVFB_PID" 2>/dev/null; then
    echo "[lg-desktop] ERROR: Xvfb failed to start. Check if display ${DISPLAY} is already in use."
    exit 1
fi

echo "[lg-desktop] Starting i3 window manager"
i3 &
I3_PID=$!
sleep 0.5

if ! kill -0 "$I3_PID" 2>/dev/null; then
    echo "[lg-desktop] ERROR: i3 window manager failed to start."
    kill "$XVFB_PID" 2>/dev/null || true
    exit 1
fi

echo "[lg-desktop] Starting VNC server on port ${VNC_PORT}"
if [ -n "${VNC_PASSWORD}" ]; then
    x11vnc -display "${DISPLAY}" -forever -shared -passwd "${VNC_PASSWORD}" -rfbport "${VNC_PORT}" -quiet &
else
    x11vnc -display "${DISPLAY}" -forever -shared -nopw -rfbport "${VNC_PORT}" -quiet &
fi
VNC_PID=$!

echo "[lg-desktop] Starting noVNC on port ${NOVNC_PORT}"
/usr/share/novnc/utils/novnc_proxy \
    --vnc localhost:"${VNC_PORT}" \
    --listen "${NOVNC_PORT}" &
NOVNC_PID=$!

sleep 1

# Verify all processes are still running
ALL_OK=true
for name_pid in "Xvfb:$XVFB_PID" "i3:$I3_PID" "x11vnc:$VNC_PID" "noVNC:$NOVNC_PID"; do
    name="${name_pid%%:*}"
    pid="${name_pid##*:}"
    if ! kill -0 "$pid" 2>/dev/null; then
        echo "[lg-desktop] ERROR: $name (PID $pid) failed to start"
        ALL_OK=false
    fi
done

if [ "$ALL_OK" = "false" ]; then
    echo "[lg-desktop] ERROR: One or more services failed. Shutting down."
    kill "$XVFB_PID" "$I3_PID" "$VNC_PID" "$NOVNC_PID" 2>/dev/null || true
    exit 1
fi

echo "[lg-desktop] Desktop ready at http://localhost:${NOVNC_PORT}/vnc.html"

# Wait for any process to exit, then identify which one and clean up
wait -n "${XVFB_PID}" "${I3_PID}" "${VNC_PID}" "${NOVNC_PID}" 2>/dev/null
EXIT_CODE=$?

echo "[lg-desktop] Process exited with code ${EXIT_CODE}"
for name_pid in "Xvfb:$XVFB_PID" "i3:$I3_PID" "x11vnc:$VNC_PID" "noVNC:$NOVNC_PID"; do
    name="${name_pid%%:*}"
    pid="${name_pid##*:}"
    if ! kill -0 "$pid" 2>/dev/null; then
        echo "[lg-desktop] $name (PID $pid) has exited"
    fi
done

echo "[lg-desktop] Shutting down remaining processes"
kill "${XVFB_PID}" "${I3_PID}" "${VNC_PID}" "${NOVNC_PID}" 2>/dev/null || true
wait
exit "${EXIT_CODE}"
