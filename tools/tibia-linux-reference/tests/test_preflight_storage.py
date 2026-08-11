from __future__ import annotations

import json
import unittest
from pathlib import Path
from unittest import mock

from tibia_linux_reference.preflight import _storage_details


class StorageEncryptionTests(unittest.TestCase):
    def test_plain_lvm_device_mapper_is_not_classified_as_encrypted(self) -> None:
        findmnt = mock.Mock(
            returncode=0,
            stdout=json.dumps(
                {"filesystems": [{"fstype": "ext4", "source": "/dev/mapper/vg-data"}]}
            ),
            stderr="",
        )
        lsblk = mock.Mock(returncode=0, stdout="lvm\n", stderr="")
        with mock.patch(
            "tibia_linux_reference.preflight.subprocess.run",
            side_effect=[findmnt, lsblk],
        ):
            result = _storage_details(Path("/private/evidence"))
        self.assertEqual(result["source_class"], "device-mapper")
        self.assertEqual(result["block_device_type"], "lvm")
        self.assertIs(result["encryption_proven"], False)

    def test_crypt_block_device_is_classified_as_encrypted(self) -> None:
        findmnt = mock.Mock(
            returncode=0,
            stdout=json.dumps(
                {"filesystems": [{"fstype": "ext4", "source": "/dev/mapper/secure"}]}
            ),
            stderr="",
        )
        lsblk = mock.Mock(returncode=0, stdout="crypt\n", stderr="")
        with mock.patch(
            "tibia_linux_reference.preflight.subprocess.run",
            side_effect=[findmnt, lsblk],
        ):
            result = _storage_details(Path("/private/evidence"))
        self.assertEqual(result["block_device_type"], "crypt")
        self.assertIs(result["encryption_proven"], True)

    def test_unknown_block_device_type_fails_closed(self) -> None:
        findmnt = mock.Mock(
            returncode=0,
            stdout=json.dumps(
                {"filesystems": [{"fstype": "ext4", "source": "/dev/mapper/ambiguous"}]}
            ),
            stderr="",
        )
        lsblk = mock.Mock(returncode=1, stdout="", stderr="not found")
        with mock.patch(
            "tibia_linux_reference.preflight.subprocess.run",
            side_effect=[findmnt, lsblk],
        ):
            result = _storage_details(Path("/private/evidence"))
        self.assertEqual(result["block_device_type"], "unknown")
        self.assertIs(result["encryption_proven"], False)


if __name__ == "__main__":
    unittest.main()
