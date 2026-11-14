"""Check an inventory produced by Hephaestus using optional private audit patterns."""
import os
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


class AnonymityTests(unittest.TestCase):
    def setUp(self):
        inventory = os.environ.get("HTON_AUDIT_INVENTORY")
        if not inventory:
            self.skipTest("HTON_AUDIT_INVENTORY must point to Hephaestus's NUL-delimited tracked file list")
        self.tracked = [Path(name) for name in Path(inventory).read_text(encoding="utf-8").split("\0") if name]
        self.assertTrue(self.tracked, "tracked inventory is empty")
        self.assertTrue(all(not path.is_absolute() and ".." not in path.parts
                            for path in self.tracked), "tracked inventory contains unsafe paths")

    def test_tracked_files_do_not_identify_the_owner(self):
        patterns = os.environ.get("HTON_AUDIT_PATTERNS")
        if not patterns:
            self.skipTest("HTON_AUDIT_PATTERNS must point to the private owner patterns file")
        pattern_path = Path(patterns).resolve()
        if pattern_path.is_relative_to(REPO) and not pattern_path.is_relative_to(REPO / ".secrets"):
            self.fail("owner patterns must be kept outside the public repository")
        tokens = [line.strip().lower() for line in pattern_path.read_bytes().splitlines() if line.strip()]
        self.assertTrue(tokens, "private owner patterns are empty")
        count = 0
        for relative in self.tracked:
            path = REPO / relative
            path_bytes = os.fsencode(relative).lower()
            content = path.read_bytes().lower() if path.is_file() and not path.is_symlink() else b""
            if any(token in path_bytes or token in content for token in tokens):
                count += 1
        self.assertEqual(count, 0, "tracked content or filenames contain private owner patterns")

    def test_secrets_tree_is_never_tracked(self):
        count = sum(path.parts[0] in {".secrets", ".deploy", ".env", "secrets"}
                    for path in self.tracked)
        self.assertEqual(count, 0, "tracked inventory contains private files")


if __name__ == "__main__":
    unittest.main()
