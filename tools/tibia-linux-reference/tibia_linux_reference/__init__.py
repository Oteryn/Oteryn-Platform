"""Safe local harness for official Tibia Linux client interoperability research."""

__all__ = ["HarnessError"]


class HarnessError(RuntimeError):
    """A fail-closed harness validation error."""
