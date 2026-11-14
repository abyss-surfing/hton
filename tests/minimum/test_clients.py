"""Unit acceptance for native Amnezia client add/remove/list."""
import io
import json
import sys
import tempfile
import unittest
from importlib.machinery import SourceFileLoader
from pathlib import Path
from unittest import mock
from urllib.parse import quote

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "python/minimum"))

import clients  # noqa: E402


def native():
    return {
        "inbounds": [{
            "port": 443,
            "protocol": "vless",
            "settings": {"clients": [{
                "id": "11111111-1111-1111-1111-111111111111",
                "flow": "xtls-rprx-vision",
                "email": "guest",
            }]},
            "streamSettings": {
                "security": "reality",
                "realitySettings": {
                    "privateKey": "private-fixture",
                    "serverNames": ["static.example.com"],
                    "shortIds": ["abcd1234"],
                },
            },
        }],
    }


class ClientDocumentTests(unittest.TestCase):
    def test_add_preserves_guest_and_existing_settings(self):
        document = clients.document(native())
        identifier = clients.add(document, "alice")
        self.assertEqual(clients.names(document), ["guest", "alice"])
        entry = document["inbounds"][0]["settings"]["clients"][1]
        self.assertEqual(entry["id"], identifier)
        self.assertEqual(entry["flow"], "xtls-rprx-vision")
        self.assertEqual(entry["email"], "alice")
        reality = document["inbounds"][0]["streamSettings"]["realitySettings"]
        self.assertEqual(reality["privateKey"], "private-fixture")
        self.assertEqual(reality["shortIds"], ["abcd1234"])

    def test_add_rejects_duplicate(self):
        document = clients.document(native())
        with self.assertRaisesRegex(clients.ClientError, "already exists"):
            clients.add(document, "guest")

    def test_names_are_bounded_and_validated(self):
        for bad in ("", "a b", "-x", "x" * 33, "a/b", "имя!", "ё" * 33):
            with self.assertRaises(clients.ClientError):
                clients.validate(bad)
        for good in ("bob-1", "Alice", "ГОСТЬ", "Иван", "ёжик", "9lives"):
            self.assertEqual(clients.validate(good), good)

    def test_remove_keeps_guest_protected(self):
        document = clients.document(native())
        with self.assertRaisesRegex(clients.ClientError, "protected"):
            clients.remove(document, "guest")

    def test_remove_unknown(self):
        document = clients.document(native())
        with self.assertRaisesRegex(clients.ClientError, "not found"):
            clients.remove(document, "alice")

    def test_remove_last_client_is_refused(self):
        document = clients.document({
            "inbounds": [{
                "settings": {"clients": [{"id": "x", "email": "only"}]},
                "streamSettings": {"realitySettings": {}},
            }],
        })
        with self.assertRaisesRegex(clients.ClientError, "last client"):
            clients.remove(document, "only")

    def test_remove_drops_only_target(self):
        document = clients.document(native())
        clients.add(document, "alice")
        clients.add(document, "bob")
        clients.remove(document, "alice")
        self.assertEqual(clients.names(document), ["guest", "bob"])

    def test_document_is_a_copy(self):
        source = native()
        document = clients.document(source)
        clients.add(document, "alice")
        self.assertEqual(clients.names(source), ["guest"])

    def test_encode_roundtrip(self):
        document = clients.document(native())
        clients.add(document, "alice")
        self.assertEqual(json.loads(clients.encode(document)), document)

    def test_invalid_documents(self):
        for payload in (b"not json", {}, {"inbounds": []},
                        {"inbounds": [{"settings": {"clients": []}}]}):
            with self.assertRaises(clients.ClientError):
                clients.document(payload)

    def test_reality_fields_and_markdown(self):
        document = clients.document(native())
        self.assertEqual(clients.reality_fields(document),
                         ("private-fixture", "static.example.com", "abcd1234"))
        text = clients.markdown("alice", "vless://x")
        self.assertTrue(text.startswith("# Amnezia — alice"))
        self.assertIn("```\nvless://x\n```", text)

    def test_connection_label_reads_instance_label_and_per_client_override(self):
        script = SourceFileLoader("clients_script", str(
            Path(__file__).resolve().parents[2] / "scripts/clients")).load_module()
        loaded = script.configuration.Loaded(root={
            "instance": {"label": "🇱🇻 Латвия"},
            "core": {"clientAccessPage": {"names": {"d6": "Свой сервер"}}},
        })
        with mock.patch.object(script.configuration, "load", return_value=loaded):
            self.assertEqual(script.connection_label("d6"), "Свой сервер")
            self.assertEqual(script.connection_label("Иван"), "🇱🇻 Латвия")

    def test_write_bundle_is_one_per_user_and_preserves_other_sections(self):
        script = SourceFileLoader("clients_script", str(
            Path(__file__).resolve().parents[2] / "scripts/clients")).load_module()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "share").mkdir()
            (root / "share" / "d6.md").write_text(
                "# d6\n\n## Outline\n\n```yaml\nkey: old\n```\n\n")
            (root / "share" / "amnezia-d6.md").write_text("# Amnezia — d6\n\nlegacy\n")
            (root / "share" / "client-page.md").write_text(
                "# Client access page\n\nhttps://example.com/secret\n")
            with mock.patch.object(script, "share_roots", return_value=[root]), \
                    mock.patch.object(script, "_shared_sections", return_value=[
                        ("MTProto", "tg://proxy?x"), ("Hysteria", "hysteria2://y")]), \
                    mock.patch.object(script.os, "fchown"), \
                    mock.patch.object(script.os, "chown"):
                path = script.write_bundle("d6", "vless://link#d6")
            text = path.read_text()
            self.assertEqual(path.name, "d6.md")
            self.assertEqual(path.stat().st_mode & 0o777, 0o640)
            self.assertIn("vless://link#d6", text)
            self.assertIn("## Outline", text)
            self.assertIn("key: old", text)
            self.assertIn("## MTProto", text)
            self.assertIn("## Hysteria", text)
            self.assertIn("## Client access page", text)
            self.assertIn("https://example.com/secret", text)
            self.assertFalse((root / "share" / "amnezia-d6.md").exists())

    def test_link_fragment_uses_configured_connection_label(self):
        script = SourceFileLoader("clients_script", str(
            Path(__file__).resolve().parents[2] / "scripts/clients")).load_module()
        document = clients.document(native())
        with mock.patch.object(script, "connection_label", return_value="🇷🇺 Россия"), \
                mock.patch.object(script.managed_amnezia, "_public_key", return_value="pub"):
            link = script.link_for(document, "d6", "11111111-1111-1111-1111-111111111111")
        self.assertTrue(link.endswith("#" + quote("🇷🇺 Россия XRay")))

    def test_identifier_returns_existing_uuid(self):
        document = clients.document(native())
        found = clients.add(document, "alice")
        self.assertEqual(clients.identifier(document, "alice"), found)
        with self.assertRaisesRegex(clients.ClientError, "not found"):
            clients.identifier(document, "bob")

    def test_write_config_sets_readable_mode_and_bounded_timeout(self):
        script = SourceFileLoader("clients_script", str(
            Path(__file__).resolve().parents[2] / "scripts/clients")).load_module()
        seen = {}

        def fake(*args, **kwargs):
            seen["path"] = args[2]
            seen["mode"] = Path(args[2]).stat().st_mode & 0o777
            seen["directory_mode"] = Path(args[2]).parent.stat().st_mode & 0o777
            seen["timeout"] = kwargs.get("timeout")
            return b""

        with mock.patch.object(script.docker, "command", side_effect=fake):
            script.write_config("a" * 64, b'{"inbounds": []}')
        self.assertEqual(seen["mode"], 0o644)
        self.assertEqual(seen["directory_mode"], 0o700)
        self.assertFalse(Path(seen["path"]).parent.exists())
        self.assertEqual(seen["timeout"], 120)

    def test_refresh_rewrites_bundles_without_restart(self):
        script = SourceFileLoader("clients_script", str(
            Path(__file__).resolve().parents[2] / "scripts/clients")).load_module()
        document = clients.document(native())
        clients.add(document, "alice")
        written = []
        with mock.patch.object(script, "find_container", return_value="a" * 64), \
                mock.patch.object(script, "read_config",
                                  return_value=clients.encode(document)), \
                mock.patch.object(script, "link_for",
                                  return_value="vless://x#Латвия XRay"), \
                mock.patch.object(script, "write_bundle",
                                  side_effect=lambda name, link: written.append(
                                      (name, link)) or Path("/tmp/x.md")), \
                mock.patch.object(script, "restart") as restart, \
                mock.patch("sys.stdout", new_callable=io.StringIO) as output:
            script.command_refresh(mock.Mock(names=["alice"], json=False,
                                             container=None))
        restart.assert_not_called()
        self.assertEqual(written, [("alice", "vless://x#Латвия XRay")])
        self.assertIn("refreshed: alice", output.getvalue())

    def test_add_accepts_multiple_names_in_one_transaction(self):
        script = SourceFileLoader("clients_script", str(
            Path(__file__).resolve().parents[2] / "scripts/clients")).load_module()
        document = clients.document(native())
        written = {}
        bundles = []
        with mock.patch.object(script, "find_container", return_value="a" * 64), \
                mock.patch.object(script, "read_config",
                                  return_value=clients.encode(document)), \
                mock.patch.object(script, "write_config",
                                  side_effect=lambda container, data:
                                  written.setdefault("data", data)), \
                mock.patch.object(script, "restart") as restart, \
                mock.patch.object(script, "link_for",
                                  return_value="vless://x#Латвия XRay"), \
                mock.patch.object(script, "write_bundle",
                                  side_effect=lambda name, link: bundles.append(name)
                                  or Path("/tmp/x.md")), \
                mock.patch("sys.stdout", new_callable=io.StringIO) as output:
            script.command_add(mock.Mock(names=["alice", "bob"], json=True,
                                         container=None))
        restart.assert_called_once()
        self.assertEqual(clients.names(clients.document(written["data"])),
                         ["guest", "alice", "bob"])
        self.assertEqual(bundles, ["alice", "bob"])
        self.assertEqual(len(json.loads(output.getvalue())["clients"]), 2)

    def test_add_duplicate_in_list_writes_nothing(self):
        script = SourceFileLoader("clients_script", str(
            Path(__file__).resolve().parents[2] / "scripts/clients")).load_module()
        document = clients.document(native())
        with mock.patch.object(script, "find_container", return_value="a" * 64), \
                mock.patch.object(script, "read_config",
                                  return_value=clients.encode(document)), \
                mock.patch.object(script, "write_config") as write_config, \
                mock.patch.object(script, "restart") as restart:
            with self.assertRaisesRegex(clients.ClientError, "already exists"):
                script.command_add(mock.Mock(names=["alice", "alice"], json=False,
                                             container=None))
        write_config.assert_not_called()
        restart.assert_not_called()

    def test_slug(self):
        self.assertEqual(clients.slug("Alice Smith"), "alice-smith")
        self.assertEqual(clients.slug("Иван Петров"), "иван-петров")
        self.assertEqual(clients.slug("???"), "client")


if __name__ == "__main__":
    unittest.main()
