from __future__ import annotations

import argparse
import ctypes
import json
import os
import signal
import socket
import sys
import time
from typing import BinaryIO


def emit(event: str, **fields: object) -> None:
    print(json.dumps({"event": event, **fields}, sort_keys=True), flush=True)


def read_secret_channel(stream: BinaryIO) -> None:
    payload = stream.read(16 * 1024)
    values = payload.splitlines()
    if len(values) != 4 or any(len(value) < 16 for value in values):
        raise RuntimeError("synthetic credential channel did not contain the expected corpus")
    emit("credential_channel_consumed", mechanism="anonymous-pipe", value_count=4)


def _blocked_tcp(family: int, address: str) -> tuple[bool, str]:
    try:
        probe = socket.socket(family, socket.SOCK_STREAM)
    except OSError as error:
        return True, error.__class__.__name__
    probe.settimeout(0.5)
    try:
        probe.connect((address, 443))
    except OSError as error:
        return True, error.__class__.__name__
    finally:
        probe.close()
    return False, "none"


def _blocked_dns() -> tuple[bool, str]:
    class ResolutionTimeout(RuntimeError):
        pass

    def timeout_handler(signum: int, frame: object) -> None:
        del signum, frame
        raise ResolutionTimeout()

    previous = signal.signal(signal.SIGALRM, timeout_handler)
    signal.setitimer(signal.ITIMER_REAL, 1.0)
    try:
        socket.getaddrinfo(
            "oteryn-network-denial-probe.invalid",
            443,
            type=socket.SOCK_STREAM,
        )
    except (socket.gaierror, OSError, ResolutionTimeout) as error:
        return True, error.__class__.__name__
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)
    return False, "none"


def prove_network_denial() -> None:
    interfaces = sorted(name for _, name in socket.if_nameindex())
    namespace = os.readlink("/proc/self/ns/net")
    ipv4_blocked, ipv4_error = _blocked_tcp(socket.AF_INET, "198.51.100.1")
    ipv6_blocked, ipv6_error = _blocked_tcp(socket.AF_INET6, "2001:db8::1")
    dns_blocked, dns_error = _blocked_dns()
    blocked = (
        ipv4_blocked
        and ipv6_blocked
        and dns_blocked
        and all(name == "lo" for name in interfaces)
    )
    error_name = "/".join(sorted({ipv4_error, ipv6_error, dns_error} - {"none"})) or "none"
    emit(
        "network_denial",
        denied=blocked,
        endpoint_classification="RFC5737-TEST-NET-2+RFC3849+RFC2606.invalid",
        error_class=error_name,
        interfaces=interfaces,
        namespace=namespace,
    )
    if not blocked:
        raise RuntimeError("dry-run network namespace is not isolated")


def _configure_x11(library: object) -> None:
    """Declare the libX11 ABI before passing 64-bit Display pointers through ctypes."""
    library.XOpenDisplay.argtypes = [ctypes.c_char_p]
    library.XOpenDisplay.restype = ctypes.c_void_p
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
    library.XStoreName.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_char_p]
    library.XStoreName.restype = ctypes.c_int
    library.XMapWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
    library.XMapWindow.restype = ctypes.c_int
    library.XDestroyWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
    library.XDestroyWindow.restype = ctypes.c_int
    library.XFlush.argtypes = [ctypes.c_void_p]
    library.XFlush.restype = ctypes.c_int
    library.XCloseDisplay.argtypes = [ctypes.c_void_p]
    library.XCloseDisplay.restype = ctypes.c_int


def open_x11_window(duration: float) -> None:
    library = ctypes.cdll.LoadLibrary("libX11.so.6")
    _configure_x11(library)
    display = library.XOpenDisplay(None)
    if not display:
        raise RuntimeError("cannot open the configured X11 display")
    try:
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
    parser.add_argument("--window-seconds", type=float, default=0.25)
    arguments = parser.parse_args()
    try:
        emit("process_state", state="started")
        read_secret_channel(sys.stdin.buffer)
        prove_network_denial()
        open_x11_window(arguments.window_seconds)
        emit("process_state", state="exiting")
        return 0
    except Exception as error:  # the parent retains only the error class
        emit("fake_client_failure", error_class=error.__class__.__name__)
        return 70


if __name__ == "__main__":
    sys.exit(main())
