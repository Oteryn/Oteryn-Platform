from __future__ import annotations

import ctypes
import unittest
from types import SimpleNamespace
from unittest import mock

from tibia_linux_reference.fake_client import _configure_x11 as configure_fake_x11
from tibia_linux_reference.x11 import _configure_x11 as configure_observer_x11


def fake_library(*names: str) -> SimpleNamespace:
    return SimpleNamespace(**{name: mock.Mock() for name in names})


class X11AbiTests(unittest.TestCase):
    def test_fake_client_declares_pointer_safe_x11_signatures(self) -> None:
        library = fake_library(
            "XOpenDisplay",
            "XDefaultScreen",
            "XRootWindow",
            "XCreateSimpleWindow",
            "XStoreName",
            "XMapWindow",
            "XDestroyWindow",
            "XFlush",
            "XCloseDisplay",
        )
        configure_fake_x11(library)

        self.assertEqual(library.XOpenDisplay.restype, ctypes.c_void_p)
        self.assertEqual(library.XStoreName.argtypes[0], ctypes.c_void_p)
        self.assertEqual(library.XMapWindow.argtypes[0], ctypes.c_void_p)
        self.assertEqual(library.XDestroyWindow.argtypes[0], ctypes.c_void_p)
        self.assertEqual(library.XFlush.argtypes, [ctypes.c_void_p])
        self.assertEqual(library.XCloseDisplay.argtypes, [ctypes.c_void_p])

    def test_observer_declares_pointer_safe_x11_signatures(self) -> None:
        library = fake_library(
            "XOpenDisplay",
            "XDefaultScreen",
            "XRootWindow",
            "XQueryTree",
            "XFree",
            "XCloseDisplay",
        )
        configure_observer_x11(library)

        self.assertEqual(library.XOpenDisplay.restype, ctypes.c_void_p)
        self.assertEqual(library.XQueryTree.argtypes[0], ctypes.c_void_p)
        self.assertEqual(library.XFree.argtypes, [ctypes.c_void_p])
        self.assertEqual(library.XCloseDisplay.argtypes, [ctypes.c_void_p])


if __name__ == "__main__":
    unittest.main()
