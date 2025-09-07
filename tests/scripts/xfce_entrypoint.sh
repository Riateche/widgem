#!/bin/bash

set -e

if [ $# -eq 0 ]; then
    2>&1 echo "No command specified"
    exit 1
fi

set -x
tigervncserver -geometry 1600x900 -localhost no :1
{ set +x; } 2>/dev/null

for i in {1..20}; do
    sleep 0.3
    echo Testing container status...
    xdotool click 1 || true
    if xdotool getactivewindow > /dev/null; then
       echo Container is ready
       break
    fi
    if ! pidof xfwm4; then
        echo xfwm4 is not running, starting xfwm4
        set -x
        xfwm4 &
        { set +x; } 2>/dev/null
    fi
done
if [ "$i" == "20" ]; then
    2>&1 echo "Container check failed"
    exit 1
fi

"$@"
