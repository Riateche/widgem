#!/bin/bash

set -ex

if [ $# -eq 0 ]; then
    2>&1 echo "No command specified"
    exit 1
fi

tigervncserver -geometry 1600x900 -localhost no :1

for i in {1..20}; do
    sleep 0.3
    echo Testing container status
    xdotool click 1 || true
    if xdotool getactivewindow; then
       echo Container is ready
       break
    fi
    if ! pidof xfwm4; then
        echo xfwm4 is not running, starting xfwm4
        xfwm4 &
    fi
done
if [ "$i" == "20" ]; then
    2>&1 echo "Container check failed"
    exit 1
fi

"$@"
