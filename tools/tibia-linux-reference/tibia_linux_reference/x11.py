from __future__ import annotations

import ctypes


def window_ids() -> set[int]:
    library = ctypes.cdll.LoadLibrary("libX11.so.6")
    library.XOpenDisplay.argtypes = [ctypes.c_char_p]
    library.XOpenDisplay.restype = ctypes.c_void_p
    display = library.XOpenDisplay(None)
    if not display:
        return set()
    try:
        library.XDefaultScreen.argtypes = [ctypes.c_void_p]
        library.XDefaultScreen.restype = ctypes.c_int
        library.XRootWindow.argtypes = [ctypes.c_void_p, ctypes.c_int]
        library.XRootWindow.restype = ctypes.c_ulong
        root = library.XRootWindow(display, library.XDefaultScreen(display))
        returned_root = ctypes.c_ulong()
        returned_parent = ctypes.c_ulong()
        children = ctypes.POINTER(ctypes.c_ulong)()
        count = ctypes.c_uint()
        library.XQueryTree.argtypes = [
            ctypes.c_void_p,
            ctypes.c_ulong,
            ctypes.POINTER(ctypes.c_ulong),
            ctypes.POINTER(ctypes.c_ulong),
            ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)),
            ctypes.POINTER(ctypes.c_uint),
        ]
        status = library.XQueryTree(
            display,
            root,
            ctypes.byref(returned_root),
            ctypes.byref(returned_parent),
            ctypes.byref(children),
            ctypes.byref(count),
        )
        if not status:
            return set()
        try:
            return {int(children[index]) for index in range(count.value)}
        finally:
            if children:
                library.XFree(children)
    finally:
        library.XCloseDisplay(display)
