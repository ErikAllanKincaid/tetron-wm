#!/usr/bin/env python3
"""Screenshot each built-in theme in an isolated, throwaway tetron-wm session.

Usage: scripts/theme-screenshots.py OUTDIR [theme ...]   (default: the four built-ins)
Needs a built binary (TETRON_WM_BIN, default target/debug/tetron-wm), chafa and a PSF
console font (fonts-terminus). Uses a fake HOME and a short XDG_RUNTIME_DIR (a unix
socket path is limited to about 100 characters), so your own desktop is untouched.
"""
import os, pty, sys, time, fcntl, termios, struct, subprocess, threading, shutil
import tempfile
OUT = os.path.abspath(sys.argv[1]); os.makedirs(OUT, exist_ok=True)
BIN = os.environ.get('TETRON_WM_BIN', os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'target', 'debug', 'tetron-wm'))
home = tempfile.mkdtemp(prefix='twh-'); run = tempfile.mkdtemp(prefix='twr-', dir='/tmp')
os.makedirs(f'{home}/Desktop')
for n in ['notes.txt', 'todo.md', 'photo.png']: open(f'{home}/Desktop/{n}', 'w').write('x')
os.makedirs(f'{home}/Desktop/projects')
env = dict(os.environ, HOME=home, XDG_CONFIG_HOME=f'{home}/.config', XDG_RUNTIME_DIR=run,
           TERM='xterm-256color', COLORTERM='truecolor', TETRON_WM_COLORS='truecolor')
env.pop('TETRON_WM_SESSION', None)
COLS, ROWS = int(os.environ.get('COLS', 120)), int(os.environ.get('ROWS', 36))
pid, fd = pty.fork()
if pid == 0:
    os.execve(BIN, [BIN, '-s', 'shots'], env)
fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', ROWS, COLS, 0, 0))
def drain():
    while True:
        try:
            if not os.read(fd, 65536): break
        except OSError: break
threading.Thread(target=drain, daemon=True).start()
def ctl(*a):
    r = subprocess.run([BIN, '-s', 'shots', *a], env=env, capture_output=True, text=True, timeout=30)
    return (r.stdout + r.stderr).strip()
time.sleep(3)
SW = r'for i in 0 1 2 3 4 5 6 7; do printf "\033[4${i}m  \033[0m"; done; echo; for i in 0 1 2 3 4 5 6 7; do printf "\033[10${i}m  \033[0m"; done; echo'
cmds = [
 ['launch', 'bash', '-c', 'sleep 4; echo "$ ls --color"; ls --color=always /usr /etc | head -12; echo; printf "\\033[31mred \\033[32mgreen \\033[33myellow \\033[34mblue \\033[35mmagenta \\033[36mcyan\\033[0m\\n"; exec bash --norc'],
 ['launch', 'bash', '-c', f'sleep 4; echo "$ ansi palette"; {SW}; echo; echo "dim text and normal text"; exec bash --norc'],
 ['launch', 'bash', '-c', 'sleep 4; echo "third window"; exec bash --norc'],
]
for c in cmds:
    print(ctl(*c)); time.sleep(1.0)
print(ctl('tile')); time.sleep(6)
for t in sys.argv[2:] or ['midnight', 'gruvbox', 'pastoral', 'windows']:
    print(t, ctl('theme', t)); time.sleep(9)
    print(ctl('screenshot', f'{OUT}/theme-{t}.png')); time.sleep(1.5)
print(ctl('kill'))
try: os.kill(pid, 15)
except OSError: pass

shutil.rmtree(home, ignore_errors=True); shutil.rmtree(run, ignore_errors=True)
