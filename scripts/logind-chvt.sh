#!/bin/sh
# Switch the seat to VT N through logind (no root; polkit allows an active local session).
# Usage: logind-chvt.sh N
exec busctl --system call org.freedesktop.login1 /org/freedesktop/login1/seat/seat0 org.freedesktop.login1.Seat SwitchTo u "$1"
