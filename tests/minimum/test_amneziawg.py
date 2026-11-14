"""Acceptance for the AmneziaWG2 minimum-profile module.

These tests are Linux-only in the same sense as the installer: they use the
descriptor-relative Store and the product's host cryptography dependency.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "python/minimum"))

import access_links
import protocols
from storage import InstallError, Store


NATIVE = {"inbounds": [{"settings": {"clients": [
    {"id": "11111111-1111-1111-1111-111111111111", "email": "alice"},
    {"id": "22222222-2222-2222-2222-222222222222", "email": "bob"},
]}}]}


class AmneziaWGTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.work = Store(Path(self.directory.name))

    def tearDown(self):
        self.work.close()
        self.directory.cleanup()

    def test_alias_selection_is_bounded(self):
        self.assertEqual(protocols.parse_selection("awg2,mtg", "--with"),
            {"amneziawg", "mtproto"})
        self.assertEqual(protocols.parse_selection("amneziawg", "--with"),
            {"amneziawg"})
        with self.assertRaises(InstallError):
            protocols.parse_selection("awg2,amneziawg", "--with")

    def test_without_awg_retains_other_first_install_defaults(self):
        self.assertEqual(protocols.selected({}, None, "amneziawg"),
            set(protocols.PROTOCOLS) - {"amneziawg"})

    def test_peer_material_is_stable_and_addresses_are_unique(self):
        names = protocols._awg_names(NATIVE)
        first = protocols._awg_material(self.work, names)
        second = protocols._awg_material(self.work, names)
        self.assertEqual(first, second)
        self.assertEqual([peer["name"] for peer in first["peers"]], ["alice", "bob"])
        self.assertEqual([peer["address"] for peer in first["peers"]],
            ["10.66.66.2", "10.66.66.3"])
        self.assertNotEqual(first["peers"][0]["privateKey"],
            first["peers"][1]["privateKey"])

    def test_server_and_client_config_are_fenced(self):
        material = protocols._awg_material(self.work, ["alice"])
        server = protocols.awg_server_config(material)
        client = protocols.awg_client_config(material, material["peers"][0],
            "example.com", 5601)
        self.assertIn(b"ListenPort = 51820", server)
        self.assertIn(b"AllowedIPs = 10.66.66.2/32", server)
        self.assertNotIn(b"Endpoint =", server)
        self.assertIn(b"Endpoint = example.com:5601", client)
        self.assertIn(b"AllowedIPs = 0.0.0.0/0", client)
        self.assertNotIn(b"ListenPort", client)
        self.assertEqual(protocols.awg_client_text(client), client)
        with self.assertRaises(InstallError):
            protocols.awg_client_text(server)

    def test_client_export_rejects_hooks_and_foreign_fields(self):
        material = protocols._awg_material(self.work, ["alice"])
        client = protocols.awg_client_config(material, material["peers"][0],
            "example.com", 51820)
        for field in (b"SaveConfig = true", b"PostUp = echo x",
                b"ListenPort = 1"):
            with self.assertRaises(InstallError):
                protocols.awg_client_text(client.replace(b"[Peer]",
                    field + b"\n[Peer]"))

    def test_plan_exports_one_client_per_native_user(self):
        args = argparse.Namespace(json=True, mtproto_camouflage=None,
            hysteria_port=8443, outline_binary=None)
        with patch("protocols._awg_port", return_value=5601), \
                patch("protocols._write_artifact",
                    side_effect=lambda work, directory, filename, content:
                        "protocols/amneziawg-test/" + filename):
            value = protocols.plan(None, self.work, {}, {
                "amneziawg": "img@sha256:" + "a" * 64},
                "example.com", ["example.com"], {"amneziawg"}, args, {},
                ask=lambda label, default="": default,
                confirm=lambda label: True, native_server=NATIVE)
        self.assertEqual(value["enabled"], ["amneziawg"])
        self.assertEqual(value["amneziawg"]["port"], 5601)
        self.assertEqual(value["amneziawg"]["files"], ["protocols/amneziawg-test/hton0.conf"])
        self.assertIn(b"ListenPort = 51820", value["amneziawg"]["server"])
        self.assertEqual(set(value["amneziawg"]["clients"]), {"alice", "bob"})
        self.assertEqual(value["documents"]["amneziawg"]["settings"]["backend"],
            "amneziawg-go")
        shares = protocols.save_shares(self.work, value, "example.com")
        self.assertEqual(set(shares), {"AmneziaWG alice", "AmneziaWG bob"})
        for path in shares.values():
            self.assertTrue(Path(path).is_file())
            self.assertIn(b"[Interface]", Path(path).read_bytes())

    def test_record_state_persists_the_selected_udp_port(self):
        state = {}
        protocols.record_state(state, {"facts": {}, "mtproto": None,
            "hysteria": None, "outline": None, "amneziawg": {"port": 5601}})
        self.assertEqual(state["amneziawgPort"], 5601)

    def test_client_artifact_is_encodable_without_leaking_server_fields(self):
        material = protocols._awg_material(self.work, ["alice"])
        client = protocols.awg_client_config(material, material["peers"][0],
            "example.com", 51820)
        artifacts = access_links.encode({"kind": "guest", "protocol": "amneziawg",
            "credentialId": "amneziawg-alice", "displayName": "alice",
            "host": "example.com", "payload": client})
        self.assertEqual(artifacts[0]["name"], "share/amneziawg-alice.conf")
        self.assertEqual(artifacts[0]["content"], client)

    def test_malformed_saved_material_blocks_instead_of_rotating_keys(self):
        self.work.write(protocols.AMNEZIAWG_CLIENT_FILE, b"{}")
        with self.assertRaises(InstallError):
            protocols._awg_material(self.work, ["alice"])

    def test_duplicate_peer_addresses_are_rejected(self):
        material = protocols._awg_material(self.work, ["alice", "bob"])
        material["peers"][1]["address"] = material["peers"][0]["address"]
        self.work.write(protocols.AMNEZIAWG_CLIENT_FILE,
            (json.dumps(material) + "\n").encode())
        with self.assertRaises(InstallError):
            protocols._awg_material(self.work, ["alice", "bob"])


if __name__ == "__main__":
    unittest.main()
