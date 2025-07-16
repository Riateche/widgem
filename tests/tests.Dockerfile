FROM docker.io/ubuntu:noble-20250415.1 AS xfce
ENV USER=root \
    DISPLAY=:1 \
    VNC_PASSWORD=1 \
    LC_ALL=en_US.UTF-8 \
    LANG=en_US.UTF-8 \
    LANGUAGE=en_US.UTF-8
RUN apt-get update && apt-get install -y \
    tigervnc-standalone-server xfce4 xfce4-terminal dbus-x11 \
    xdotool wmctrl libxkbcommon-x11-0 locales locales-all && \
    mkdir /root/.vnc && \
    printf '#!/bin/bash\nstartxfce4\n' > /root/.vnc/xstartup && \
    chmod +x /root/.vnc/xstartup && \
    echo "$VNC_PASSWORD" | vncpasswd -f > /root/.vnc/passwd && \
    chmod 0600 /root/.vnc/passwd && \
    touch /root/.Xauthority
COPY xfce_entrypoint.sh /entrypoint
ENTRYPOINT ["/entrypoint"]

FROM xfce AS test
ARG BUILD_MODE
ENV RUST_BACKTRACE=1 \
    WIDGEM_REPO_DIR=/app
COPY widgem_tests /usr/local/bin/
