"""Unit tests for verify_dispatch helpers."""

from __future__ import annotations

import unittest

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from verify_dispatch import require_int, require_sha


class VerifyDispatchTest(unittest.TestCase):
    def test_require_int_from_string(self) -> None:
        self.assertEqual(require_int("42", "x"), 42)

    def test_require_sha_valid(self) -> None:
        self.assertEqual(require_sha("b953161761ab", "sha"), "b953161761ab")

    def test_require_sha_rejects_invalid(self) -> None:
        with self.assertRaises(SystemExit):
            require_sha("not-hex!", "sha")


if __name__ == "__main__":
    unittest.main()
