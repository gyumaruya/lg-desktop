#!/bin/bash
# Check lg-desktop health

STATUS=$(docker inspect --format '{{.State.Status}}' lg-desktop 2>/dev/null || echo "not_found")

if [ "$STATUS" = "running" ]; then
    # Verify internal services are functional
    VNC_OK=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/ 2>/dev/null || echo "000")
    X11_OK=$(docker exec -e DISPLAY=:1 lg-desktop xdpyinfo > /dev/null 2>&1 && echo "ok" || echo "fail")

    if [ "$VNC_OK" != "200" ] || [ "$X11_OK" != "ok" ]; then
        echo "{\"status\":\"degraded\",\"container\":\"running\",\"vnc\":\"$VNC_OK\",\"x11\":\"$X11_OK\"}"
        exit 1
    fi

    echo '{"status":"healthy","url":"http://localhost:3000","container":"lg-desktop"}'
    exit 0
elif [ "$STATUS" = "not_found" ]; then
    echo '{"status":"not_found","message":"Container lg-desktop does not exist. Run setup.sh first."}'
    exit 1
else
    echo "{\"status\":\"$STATUS\",\"container\":\"lg-desktop\"}"
    exit 1
fi
