# Give the blender MCP server a Blender to talk to, from a Blender with no
# window: run this as `blender -b --python tools/hammer/serve.py`, and leave it
# running for as long as the session is modelling.
#
# The MCP server is `uvx blender-mcp` — the repository's .mcp.json spawns it, so
# a session here has the tools without anybody starting anything. What it needs
# at the other end is the addon the same package bundles, listening on a socket.
# This script is that end.
#
# It exists because the addon will not do it itself. Its own start() refuses
# under `blender -b` and says to use a GUI or a virtual display, and it is right
# to: it runs every command on Blender's main thread from a bpy.app.timers
# callback, and a background Blender runs no timers, so a server started there
# would accept a command and never execute it. The advice does not help here —
# there is no display in a Verkstead Sandbox, and Blender under xvfb-run dies
# for want of a GL context. So this script supplies the missing half itself: it
# starts the server with the background check stepped around, and then pumps the
# queue in a loop on the main thread, which is the timer that background mode
# does not run.
#
# The addon comes from the installed package rather than from a Blender profile,
# asked for by path through `uvx --from blender-mcp python`, so the socket end
# and the MCP end are always one version of one package. Nothing is installed
# into Blender and nothing is left behind.
#
# Stop it with a signal — Ctrl-C, or `kill` on the pid — and it closes the
# socket and unregisters the addon on the way out.
#
#   $ blender -b --python tools/hammer/serve.py            # the addon's default port, 9876
#   $ blender -b --python tools/hammer/serve.py -- --port 9877
#
import argparse
import importlib.util
import os
import signal
import subprocess
import sys
import time
from contextlib import contextmanager

# What the MCP server dials unless .mcp.json's environment says otherwise, and
# the addon's own default: one number in two places, so neither has to be told.
DEFAULT_PORT = 9876

# How long the main thread sleeps between drains. Commands arrive on a socket
# thread and are executed here, so this is the latency the MCP sees on every
# call; 20ms is far below what a round trip through the model costs anyway.
POLL_SECONDS = 0.02


def bundled_addon_path():
    """Ask the blender-mcp package where its bundled addon.py is.

    Through uvx rather than an import, because Blender's Python is not the
    environment the package is installed in — and this is the same uvx
    invocation that .mcp.json's server runs under, so it is the same version of
    the same package on both ends of the socket.

    PYTHONPATH comes out of the environment first. Blender is launched by a
    wrapper that puts its own 3.13 site-packages on it, and a 3.14 interpreter
    that inherits them imports the wrong typing_extensions and dies on the way
    in to the package.
    """
    env = dict(os.environ)
    env.pop("PYTHONPATH", None)
    env.pop("PYTHONHOME", None)
    out = subprocess.run(
        [
            "uvx",
            "--from",
            "blender-mcp",
            "python",
            "-c",
            "from blender_mcp.addon_manager import get_bundled_addon_path as p; print(p())",
        ],
        capture_output=True,
        text=True,
        check=True,
        env=env,
    )
    return out.stdout.strip()


def load_addon(path):
    """Import addon.py from a path, without installing it into Blender."""
    spec = importlib.util.spec_from_file_location("blendermcp_addon", path)
    module = importlib.util.module_from_spec(spec)
    # Under its own name in sys.modules before it executes: the addon's classes
    # are registered with Blender, which keeps the module alive by name.
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class _ForegroundApp:
    """`bpy.app`, reading as though Blender had a window."""

    background = False

    def __init__(self, app):
        self._app = app

    def __getattr__(self, name):
        return getattr(self._app, name)


class _ForegroundBpy:
    """`bpy`, with that one attribute swapped and everything else itself."""

    def __init__(self, module):
        self._module = module
        self.app = _ForegroundApp(module.app)

    def __getattr__(self, name):
        return getattr(self._module, name)


@contextmanager
def foreground(addon):
    """Lie to the addon about background mode, for the length of one call.

    Narrowly, and only around start(): `bpy.app.background` is read-only, so the
    check cannot be answered any other way, and a shim left in place for the
    whole run would be a second bpy for every command the addon executes. What
    the check guards against is a queue nothing drains, which is what the loop
    below is for.
    """
    real = addon.bpy
    addon.bpy = _ForegroundBpy(real)
    try:
        yield
    finally:
        addon.bpy = real


def script_args():
    """Blender's own arguments stop at `--`; ours are what follows it."""
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(prog="serve.py")
    parser.add_argument("--host", default="localhost")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    return parser.parse_args(argv)


def main():
    args = script_args()

    addon = load_addon(bundled_addon_path())
    addon.register()

    server = addon.BlenderMCPServer(host=args.host, port=args.port)
    with foreground(addon):
        server.start()
    if not server.running:
        # start() prints why and swallows the exception; a port already listened
        # on is the usual reason.
        addon.unregister()
        return 1

    # start() also handed this drain to bpy.app.timers, where it will never fire,
    # so the loop below is the only thing running queued commands — and, being
    # the only caller, it cannot run one twice.
    print(
        f"serve.py: draining on the main thread; {args.host}:{args.port} is open",
        flush=True,
    )

    stopping = []
    signal.signal(signal.SIGTERM, lambda *_: stopping.append(True))
    try:
        while not stopping:
            server._drain_command_queue()
            time.sleep(POLL_SECONDS)
    except KeyboardInterrupt:
        pass
    finally:
        server.stop()
        addon.unregister()
    return 0


if __name__ == "__main__":
    sys.exit(main())
