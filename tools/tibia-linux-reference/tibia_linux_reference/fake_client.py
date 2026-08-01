from __future__ import annotations

import argparse
import ctypes
import json
import os
import socket
import sys
import time


def emit(event: str, **fields: object) -> None:
    print(json.dumps({"event": event, **fields}, sort_keys=True), flush=True)


def read_secret_channel(descriptor: int) -> None:
    with os.fdopen(descriptor, "rb", closefd=True) as handle:
        payload = handle.read(16 * 1024)
    values = payload.splitlines()
    if len(values) != 4 or any(len(value) < 16 for value in values):
        raise RuntimeError("synthetic credential channel did not contain the expected corpus")
    emit("credential_channel_consumed", mechanism="anonymous-pipe", value_count=4)


def prove_network_denial() -> None:
    interfaces = sorted(name for _, name in socket.if_nameindex())
    namespace = os.readlink("/proc/self/ns/net")
    blocked = False
    error_name = "none"
    probe = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    probe.settimeout(0.5)
    try:
        probe.connect(("198.51.100.1", 443))
    except OSError as error:
        blocked = True
        error_name = error.__class__.__name__
    finally:
        probe.close()
    emit(
        "network_denial",
        denied=blocked,
        endpoint_classification="TEST-NET-2:443",
        error_class=error_name,
        interfaces=interfaces,
        namespace=namespace,
    )
    if not blocked or any(name != "lo" for name in interfaces):
        raise RuntimeError("dry-run network namespace is not isolated")


def open_x11_window(duration: float) -> None:
    library = ctypes.cdll.LoadLibrary("libX11.so.6")
    library.XOpenDisplay.argtypes = [ctypes.c_char_p]
    library.XOpenDisplay.restype = ctypes.c_void_p
    display = library.XOpenDisplay(None)
    if not display:
        raise RuntimeError("cannot open the configured X11 display")
    try:
        library.XDefaultScreen.argtypes = [ctypes.c_void_p]
        library.XDefaultScreen.restype = ctypes.c_int
        library.XRootWindow.argtypes = [ctypes.c_void_p, ctypes.c_int]
        library.XRootWindow.restype = ctypes.c_ulong
        library.XCreateSimpleWindow.argtypes = [
            ctypes.c_void_p,
            ctypes.c_ulong,
            ctypes.c_int,
            ctypes.c_int,
            ctypes.c_uint,
            ctypes.c_uint,
            ctypes.c_uint,
            ctypes.c_ulong,
            ctypes.c_ulong,
        ]
        library.XCreateSimpleWindow.restype = ctypes.c_ulong
        screen = library.XDefaultScreen(display)
        root = library.XRootWindow(display, screen)
        window = library.XCreateSimpleWindow(display, root, 20, 20, 320, 120, 1, 0, 0x202020)
        library.XStoreName(display, window, b"Oteryn synthetic no-network client")
        library.XMapWindow(display, window)
        library.XFlush(display)
        emit("window_state", backend="x11", state="mapped")
        time.sleep(duration)
        library.XDestroyWindow(display, window)
        library.XFlush(display)
        emit("window_state", backend="x11", state="destroyed")
    finally:
        library.XCloseDisplay(display)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--secret-fd", type=int, required=True)
    parser.add_argument("--window-seconds", type=float, default=0.25)
    arguments = parser.parse_args()
    try:
        emit("process_state", state="started")
        read_secret_channel(arguments.secret_fd)
        prove_network_denial()
        open_x11_window(arguments.window_seconds)
        emit("process_state", state="exiting")
        return 0
    except Exception as error:  # the parent retains only the error class
        emit("fake_client_failure", error_class=error.__class__.__name__)
        return 70


if __name__ == "__main__":
    sys.exit(main())
