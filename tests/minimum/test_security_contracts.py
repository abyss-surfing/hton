"""U1 security contracts for the M1 configuration, DNS, page and access owners."""
from __future__ import annotations

import argparse
import copy
import io
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "python/minimum"))
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "lib"))

import access_links
import client_page
import docker
import configuration
import host_access
import install
import managed_amnezia
import secrets_store
import storage
from storage import InstallError, Store, digest

ROOT_YAML = b"""schemaVersion: 1
instance:
  name: hton
core:
  edge:
    listen:
      transport: tcp
      port: 443
      addressFamilies: [ipv4]
    domains: [example.com]
    hostPolicy:
      wildcardZone: example.com
      redirectTarget: https://example.com/
      unknown: reject
  certificates:
    ca: letsencrypt
    challenge: dns-01
    provider: internetbs
    email: operator@example.com
    wildcardDomain: example.com
    credentials:
      apiKey: {secretRef: dns/api-key}
      apiPassword: {secretRef: dns/api-password}
  dns:
    upstreamMode: tls-forward
    upstreams:
      primary:
        addresses: [1.1.1.1, 1.0.0.1]
        port: 853
        tlsName: one.one.one.one
    upstreamOrder: [primary]
    fallback: fail-closed
  clientAccessPage:
    path: ""
    names: {}
    refreshSeconds: 60
features:
  mtproto: {enabled: true}
"""


def build_tree(base: Path) -> Path:
    tree = base / "config"
    store = Store(tree, create=True)
    try:
        store.write("hton.yaml", ROOT_YAML)
        (tree / "modules").mkdir(mode=0o700, exist_ok=True)
        store.write("modules/foreign-amnezia.yaml", b"""schemaVersion: 1
moduleId: foreign-amnezia
settings:
  mode: provision
  container: amnezia-xray
  serverNames: [static.example.com]
  compatibility:
    legacyServerNames: []
  hostPorts: {tcp: 5714, udp: 5714}
  provision:
    clientName: guest
  admin: {access: generate, user: hton-amnezia, sshPort: 22}
""")
    finally:
        store.close()
    return tree


class Base(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self):
        self.temp.cleanup()

    def loaded(self):
        return configuration.load(build_tree(self.root))

    def valid(self):
        result = configuration.validate(self.loaded())
        self.assertTrue(result.ok, result.diagnostics)
        return result

    def broken(self, mutate):
        loaded = self.loaded()
        mutate(loaded.root)
        return configuration.validate(loaded)

    def make_work(self):
        (self.root / "work").mkdir(mode=0o700, exist_ok=True)
        return Store(self.root / "work/minimum", create=True)


class ConfigurationTests(Base):
    def test_example_tree_validates_and_derives_inventory(self):
        result = self.valid()
        names = {item["name"]: item["kind"] for item in configuration.resolved_names(result)}
        self.assertEqual(names["example.com"], "site")
        self.assertEqual(names["*.example.com"], "wildcard")
        self.assertEqual(names["static.example.com"], "reality")
        self.assertEqual(result.modules["foreign-amnezia"].mode, "provision")

    def test_provision_client_name_accepts_cyrillic_and_uppercase(self):
        for name in ("Гость", "Alice", "Иван_1.гость", "9жизней"):
            loaded = self.loaded()
            loaded.modules["foreign-amnezia"]["settings"]["provision"]["clientName"] = name
            result = configuration.validate(loaded)
            self.assertTrue(result.ok, result.diagnostics)
            self.assertEqual(result.modules["foreign-amnezia"].client_name, name)
        for name in ("a b", "-x", "x" * 33, "гость!"):
            loaded = self.loaded()
            loaded.modules["foreign-amnezia"]["settings"]["provision"]["clientName"] = name
            result = configuration.validate(loaded)
            self.assertFalse(result.ok)
            self.assertIn("module.clientName", [d["code"] for d in result.diagnostics])

    def test_instance_connection_label_is_validated(self):
        loaded = self.loaded()
        loaded.root["instance"]["label"] = "🇷🇺 Россия"
        result = configuration.validate(loaded)
        self.assertTrue(result.ok, result.diagnostics)
        self.assertEqual(result.desired.connection_label, "🇷🇺 Россия")
        for bad in ("", "  ", "x" * 65, "a\nb"):
            loaded = self.loaded()
            loaded.root["instance"]["label"] = bad
            result = configuration.validate(loaded)
            self.assertFalse(result.ok)
            self.assertIn("instance.label", [d["code"] for d in result.diagnostics])

    def test_test_tunnel_is_separate_from_site_names(self):
        loaded = self.loaded()
        loaded.root["core"]["edge"]["testTunnel"] = {
            "sni": "probe.example.com", "port": 19090,
        }
        result = configuration.validate(loaded)
        self.assertTrue(result.ok, result.diagnostics)
        self.assertEqual(result.desired.edge.test_tunnel.sni, "probe.example.com")
        names = {item["name"]: item["kind"] for item in configuration.resolved_names(result)}
        self.assertEqual(names["probe.example.com"], "test-tunnel")

        for mutation in (
            {"sni": "example.com", "port": 19090},
            {"sni": "www.example.com", "port": 19090},
            {"sni": "probe.example.com", "port": 80},
            {"sni": "probe.example.com", "port": 65536},
            {"sni": "not a name", "port": 19090},
        ):
            loaded = self.loaded()
            loaded.root["core"]["edge"]["testTunnel"] = mutation
            result = configuration.validate(loaded)
            self.assertFalse(result.ok, mutation)

        loaded = self.loaded()
        loaded.root["core"]["edge"]["testTunnel"] = {
            "enabled": False, "sni": "probe.example.com", "port": 19090,
        }
        result = configuration.validate(loaded)
        self.assertTrue(result.ok, result.diagnostics)
        self.assertIsNone(result.desired.edge.test_tunnel)

    def test_install_test_tunnel_options_are_explicit(self):
        root = {"core": {"edge": {}}}
        args = argparse.Namespace(test_sni="probe.example.com",
            test_tunnel_port=19090, no_test_tunnel=False)
        value = install.configure_test_tunnel(root, args, "example.com")
        self.assertEqual(value, {"sni": "probe.example.com", "port": 19090})
        self.assertEqual(root["core"]["edge"]["testTunnel"], value)
        install.validate_test_tunnel(value, ["example.com", "www.example.com"])
        with self.assertRaises(InstallError):
            install.validate_test_tunnel(value, ["probe.example.com"])

        saved = argparse.Namespace(test_sni=None, test_tunnel_port=None,
            no_test_tunnel=False)
        self.assertEqual(install.configure_test_tunnel(root, saved, "example.com"), value)
        disabled = argparse.Namespace(test_sni=None, test_tunnel_port=None,
            no_test_tunnel=True)
        self.assertIsNone(install.configure_test_tunnel(root, disabled, "example.com"))
        self.assertNotIn("testTunnel", root["core"]["edge"])

        for bad in (
            argparse.Namespace(test_sni="probe.example.com", test_tunnel_port=None,
                no_test_tunnel=False),
            argparse.Namespace(test_sni="example.com", test_tunnel_port=19090,
                no_test_tunnel=False),
            argparse.Namespace(test_sni="probe.example.com", test_tunnel_port=80,
                no_test_tunnel=False),
        ):
            with self.assertRaises(InstallError):
                install.configure_test_tunnel({"core": {"edge": {}}}, bad, "example.com")

    def test_redirect_target_must_equal_apex(self):
        result = self.broken(lambda root: root["core"]["edge"]["hostPolicy"].update(
            redirectTarget="https://wrong.example/"))
        self.assertFalse(result.ok)
        self.assertIn("edge.hostPolicy.redirect-mismatch", [d["code"] for d in result.diagnostics])

    def test_wildcard_zone_requires_dns_01(self):
        def mutate(root):
            root["core"]["certificates"]["challenge"] = "http-01"
        result = self.broken(mutate)
        self.assertFalse(result.ok)
        self.assertIn("config.wildcard-challenge", [d["code"] for d in result.diagnostics])

    def test_non_ipv4_bootstrap_is_rejected(self):
        def mutate(root):
            root["core"]["dns"]["upstreams"]["primary"]["addresses"] = ["not-an-ip"]
        result = self.broken(mutate)
        self.assertFalse(result.ok)
        self.assertIn("dns.upstream.invalid", [d["code"] for d in result.diagnostics])

    def test_dns_requires_ordered_upstreams_without_fallback(self):
        for mutate in (
            lambda root: root["core"]["dns"].update(upstreamOrder=[]),
            lambda root: root["core"]["dns"].update(fallback="os"),
        ):
            result = self.broken(mutate)
            self.assertFalse(result.ok)

    def test_client_page_path_rejects_slash(self):
        def mutate(root):
            root["core"]["clientAccessPage"]["path"] = "bad/path"
        result = self.broken(mutate)
        self.assertFalse(result.ok)
        self.assertIn("clientPage.path.invalid", [d["code"] for d in result.diagnostics])

    def test_admin_username_and_collision_are_rejected(self):
        with self.assertRaises(host_access.HostAccessError):
            host_access.plan({"access": "generate", "user": "root", "sshPort": 22})
        with self.assertRaises(host_access.HostAccessError):
            host_access.plan({"access": "generate", "user": "bob", "sshPort": 22},
                {"users": ["bob"]})

    def test_migrations_plan_is_read_only(self):
        work = Store(self.root / "work", create=True)
        try:
            from storage import document  # noqa: F401
            import migrations
            plan = migrations.plan(work, None, None)
            self.assertTrue(plan and all(item["pending"] for item in plan))
            self.assertIsNone(work.read("migrations.json"))
        finally:
            work.close()


class ClientPageTests(Base):
    def settings(self, path="secret-path"):
        return configuration.ClientPage(path, {}, 60)

    def inventory(self):
        return [
            {"credentialId": "a", "protocol": "outline-wss", "displayName": "A <x>",
             "status": "active", "payload": "ss://key", "sourceRevision": "1"},
            {"credentialId": "b", "protocol": "amnezia", "displayName": "B",
             "status": "unavailable", "payload": None, "sourceRevision": "2"},
        ]

    def test_render_escapes_and_sets_privacy_headers(self):
        plan_value = client_page.plan(self.settings(), self.inventory())
        self.assertEqual(plan_value.coverage, "partial")
        artifact = client_page.render(plan_value, self.inventory())
        self.assertIn(b"&lt;x&gt;", artifact.html)
        self.assertEqual(artifact.digest, digest(artifact.html))
        self.assertEqual(artifact.headers["Cache-Control"], "no-store")
        self.assertIn("default-src 'none'", artifact.headers["Content-Security-Policy"])

    def test_empty_path_withdraws(self):
        work = Store(self.root / "work", create=True)
        try:
            receipt = client_page.refresh(self.settings(), self.inventory(), work)
            self.assertTrue(receipt["enabled"])
            artifact = work.read(receipt["artifact"])
            self.assertTrue(artifact)
            withdrawn = client_page.refresh(self.settings(""), self.inventory(), work)
            self.assertTrue(withdrawn["withdrawn"])
            self.assertIsNone(work.read(receipt["artifact"]))
        finally:
            work.close()

    def test_invalid_path_and_reserved_collision(self):
        with self.assertRaises(client_page.ClientPageError):
            client_page.plan(self.settings("bad/path"), self.inventory())
        with self.assertRaises(client_page.ClientPageError):
            client_page.plan(self.settings("admin"), self.inventory())

    def test_generated_path_is_safe_secret_and_unique(self):
        generated = {client_page.generate_path() for _ in range(32)}
        self.assertEqual(len(generated), 32)
        for value in generated:
            self.assertGreaterEqual(len(value), 22)
            self.assertTrue(client_page._valid_path(value), value)
            self.assertNotIn(value, client_page.RESERVED_PATHS)

    def test_persisted_receipt_does_not_contain_the_bearer_path(self):
        work = Store(self.root / "work", create=True)
        try:
            receipt = client_page.refresh(self.settings("secret-path"), self.inventory(), work)
            self.assertEqual(receipt["path"], "secret-path")
            persisted = work.read("client-page/receipt.json")
            self.assertNotIn(b"secret-path", persisted)
            self.assertNotIn("path", work.json("client-page/receipt.json"))
        finally:
            work.close()

    def test_admin_payload_is_forbidden(self):
        with self.assertRaises(client_page.ClientPageError):
            client_page.plan(self.settings(), [
                {"credentialId": "z", "kind": "admin", "status": "active", "payload": "x"}])
        with self.assertRaises(client_page.ClientPageError):
            client_page.plan(self.settings(), [
                {"credentialId": "z", "status": "active", "payload": "x",
                 "fields": ["privateKey"]}])

    def test_page_refresh_survives_unrelated_blocking_diagnostics(self):
        tree = build_tree(self.root)
        store = Store(tree)
        try:
            text = store.read("hton.yaml").decode()
            store.write("hton.yaml", text.replace(
                "challenge: dns-01", "challenge: http-01").replace(
                'path: ""', 'path: "secret-path"').encode())
        finally:
            store.close()
        loaded = configuration.load(tree)
        self.assertTrue(configuration.validate(loaded).blocking)
        config = Store(tree)
        work = Store(self.root / "work", create=True)
        try:
            with patch.object(install.managed_amnezia, "client_inventory",
                              return_value=[]):
                receipt = install.refresh_client_page(
                    config, work, {"container": "a" * 64})
            self.assertTrue(receipt["enabled"])
            self.assertIsNotNone(work.read("client-page/index.html"))
        finally:
            config.close()
            work.close()


    def test_refresh_timer_reads_the_managed_container(self):
        tree = build_tree(self.root)
        store = Store(tree)
        try:
            text = store.read("hton.yaml").decode()
            store.write("hton.yaml", text.replace('path: ""', 'path: "secret-path"').encode())
        finally:
            store.close()
        work = self.make_work()
        work.save("managed.json", {"schemaVersion": 1, "kind": "managed",
            "instance": "a" * 32, "operation": "b" * 32, "phase": "committed",
            "port": 5714, "container": "c" * 64})
        material = managed_amnezia.generate_key_material()
        native = {"inbounds": [{"settings": {"clients": [
            {"email": "guest", "id": material["uuid"]}]},
            "streamSettings": {"realitySettings": {
                "privateKey": material["privateKey"],
                "serverNames": ["static.example.com"],
                "shortIds": [material["shortId"]]}}}]}
        config = Store(tree)
        try:
            with patch("install.docker.native_config",
                       return_value=json.dumps(native).encode()):
                receipt = install.refresh_client_page(config, work, {})
            self.assertTrue(receipt["enabled"])
            self.assertIn("vless://", work.read("client-page/index.html").decode())
        finally:
            config.close()
            work.close()


class AccessLinkTests(unittest.TestCase):
    def test_guest_links(self):
        hy = access_links.encode({"kind": "guest", "protocol": "hysteria", "credentialId": "h",
            "displayName": "H", "host": "example.com", "secret": "abc", "port": 443})
        self.assertTrue(hy[0]["content"].startswith(b"hysteria2://abc@example.com:443"))
        tg = access_links.encode({"kind": "guest", "protocol": "mtproto", "credentialId": "m",
            "host": "example.com", "secret": "ee00", "port": 443})
        self.assertTrue(tg[0]["content"].startswith(b"tg://proxy?server=example.com"))

    def test_admin_and_secret_payloads_are_rejected(self):
        with self.assertRaises(access_links.IncompatibleExport):
            access_links.encode({"kind": "admin", "protocol": "amnezia", "credentialId": "x",
                "host": "example.com", "payload": b"x"})
        with self.assertRaises(access_links.IncompatibleExport):
            access_links.encode({"kind": "guest", "protocol": "amnezia", "credentialId": "x",
                "host": "example.com", "payload": b"-----BEGIN OPENSSH PRIVATE KEY-----"})

    def test_invalid_host_is_rejected(self):
        with self.assertRaises(access_links.IncompatibleExport):
            access_links.encode({"kind": "guest", "protocol": "mtproto", "credentialId": "m",
                "host": "https://example.com/", "secret": "ee00"})


class HostAccessTests(Base):
    def test_plan_and_disabled_mode(self):
        plan_value = host_access.plan({"access": "generate", "user": "hton-amnezia", "sshPort": 22})
        self.assertTrue(plan_value.enabled)
        self.assertEqual(plan_value.username, "hton-amnezia")
        self.assertTrue(host_access.plan({"access": "none"}).disabled)

    def test_purge_requires_consent_and_stays_in_tree(self):
        with self.assertRaises(host_access.HostAccessError):
            host_access.purge({"receiptId": "a" * 16}, explicit_consent=False)
        rid = "b" * 16
        home = self.root / "host-access" / rid / "home"
        home.mkdir(parents=True)
        (home / "f").write_text("x")
        receipt = host_access.purge({"receiptId": rid}, True, root=self.root)
        self.assertTrue(receipt["purged"])
        self.assertFalse((self.root / "host-access" / rid).exists())
        with self.assertRaises(host_access.HostAccessError):
            host_access.purge({"receiptId": "../../etc"}, True, root=self.root)


class IngressRouteTests(unittest.TestCase):
    def test_client_page_route_precedes_site(self):
        import ingress
        import json
        _ha, caddy = ingress.configs("example.com", ["reality.example.com"], 5714, 9443,
            b"<html>x</html>", root=None,
            client_page={"path": "secret", "headers": {"Cache-Control": "no-store"}})
        server = json.loads(caddy)["apps"]["http"]["servers"]["site"]
        self.assertTrue(server["strict_sni_host"])
        routes = server["routes"]
        self.assertIn('string({http.request.orig_uri}) == "/secret"', routes[0]["match"][0]["expression"])
        self.assertEqual(routes[1]["match"][0]["expression"], routes[0]["match"][0]["expression"])
        self.assertEqual(routes[0]["match"][0]["method"], ["GET", "HEAD"])
        self.assertNotIn("headers", routes[0])
        self.assertEqual(routes[0]["handle"][0]["handler"], "headers")
        self.assertEqual(
            routes[0]["handle"][0]["response"]["set"]["Cache-Control"],
            ["no-store"])
        self.assertIn("/client-page", json.dumps(routes[0]["handle"]))
        self.assertEqual(routes[-2]["match"][0]["host"], ["example.com"])
        self.assertEqual(routes[-1]["match"][0]["host"], ["www.example.com"])

    def test_plain_http_redirects_and_serves_acme_webroot(self):
        import ingress
        import json
        _ha, caddy = ingress.configs("example.com", ["reality.example.com"], 5714, 9443,
            b"<html>x</html>", root="/var/www/html")
        servers = json.loads(caddy)["apps"]["http"]["servers"]
        redirect = servers["http_redirect"]
        self.assertEqual(redirect["listen"], ["0.0.0.0:80"])
        self.assertEqual(redirect["automatic_https"], {"disable": True})
        self.assertEqual(redirect["routes"][0]["match"][0]["path"],
                         ["/.well-known/acme-challenge/*"])
        self.assertEqual(redirect["routes"][0]["handle"][0]["root"], "/site")
        last = redirect["routes"][-1]["handle"][0]
        self.assertEqual(last["status_code"], 308)
        self.assertEqual(last["headers"]["Location"],
                         ["https://{http.request.host}{http.request.uri}"])

    def test_reality_browser_route_returns_fixed_denial(self):
        import ingress
        import json
        _ha, caddy = ingress.configs("example.com", ["static.example.com"], 5714, 9443,
            b"<html>site body</html>", reality_names=["static.example.com"])
        routes = json.loads(caddy)["apps"]["http"]["servers"]["site"]["routes"]
        denial = [route for route in routes
            if route["match"][0]["host"] == ["static.example.com"]][0]["handle"][0]
        self.assertEqual(denial["status_code"], 403)
        self.assertIn("Access denied", denial["body"])
        self.assertNotIn("site body", denial["body"])

    def test_www_redirects_to_the_apex(self):
        import ingress
        import json
        haproxy, caddy = ingress.configs("example.com", ["static.example.com"], 5714, 9443,
            b"<html>x</html>")
        self.assertIn(b"www.example.com", haproxy)
        servers = json.loads(caddy)["apps"]["http"]["servers"]["site"]
        redirect = [route for route in servers["routes"]
            if route["match"][0]["host"] == ["www.example.com"]][0]["handle"][0]
        self.assertEqual(redirect["status_code"], 308)
        self.assertEqual(redirect["headers"]["Location"],
                         ["https://example.com{http.request.uri}"])
        self.assertIn("www.example.com",
                      servers["tls_connection_policies"][0]["match"]["sni"])

    def test_placeholder_site_keeps_http_port_free(self):
        import ingress
        import json
        _ha, caddy = ingress.configs("example.com", ["reality.example.com"], 5714, 9443,
            b"<html>x</html>", root=None)
        servers = json.loads(caddy)["apps"]["http"]["servers"]
        self.assertNotIn("http_redirect", servers)

    def test_reality_names_use_the_site_certificate(self):
        import ingress
        import json
        _ha, caddy = ingress.configs("example.com", ["static.example.com"], 5714, 9443,
            b"<html>x</html>", reality_names=["static.example.com"])
        app = json.loads(caddy)["apps"]
        self.assertEqual(app["tls"]["certificates"]["load_files"],
                         [{"certificate": "/certs/fullchain.pem",
                           "key": "/certs/key.pem"}])
        policy = app["http"]["servers"]["site"]["tls_connection_policies"][0]
        self.assertEqual(policy["match"]["sni"],
                         ["example.com", "www.example.com", "static.example.com"])


class ManagedAmneziaTests(unittest.TestCase):
    def test_key_material_and_server_config(self):
        material = managed_amnezia.generate_key_material()
        self.assertEqual(len(material["privateKey"]), 43)
        self.assertEqual(len(material["publicKey"]), 43)
        plan_value = managed_amnezia.ManagedAmneziaPlan("provision", "amnezia-xray",
            "img@sha256:" + "a" * 64, ("static.example.com",), (), 5714, "guest", {"access": "generate"})
        config = managed_amnezia.server_config(plan_value, material)
        import json
        parsed = json.loads(config)
        reality = parsed["inbounds"][0]["streamSettings"]["realitySettings"]
        self.assertEqual(reality["privateKey"], material["privateKey"])
        self.assertEqual(reality["serverNames"], ["static.example.com"])
        self.assertNotIn("dns", parsed)

    def test_client_inventory_link_label_keeps_display_name(self):
        from urllib.parse import quote
        import json
        material = managed_amnezia.generate_key_material()
        config = json.dumps({"inbounds": [{
            "settings": {"clients": [{"id": "u1", "email": "d6"}]},
            "streamSettings": {"security": "reality", "realitySettings": {
                "privateKey": material["privateKey"],
                "serverNames": ["example.com"], "shortIds": ["abcd1234"]}}}]}).encode()
        entries = managed_amnezia.client_inventory(config, link_label="🇷🇺 Россия")
        self.assertEqual(entries[0]["credentialId"], "d6")
        self.assertEqual(entries[0]["displayName"], "d6")
        self.assertTrue(entries[0]["payload"].endswith("#" + quote("🇷🇺 Россия XRay")))

    def test_client_inventory_labels_and_bad_config(self):
        config = b'{"inbounds":[{"settings":{"clients":[{"id":"u1","email":"guest"}]}}]}'
        entries = managed_amnezia.client_inventory(config, {"guest": "Guest"})
        self.assertEqual(entries[0]["credentialId"], "guest")
        self.assertEqual(entries[0]["displayName"], "Guest")
        with self.assertRaises(managed_amnezia.ManagedAmneziaError):
            managed_amnezia.client_inventory(b"not-json")

    def test_plan_requires_pinned_image(self):
        foreign = type("F", (), {"mode": "provision", "server_names": ("a.example.com",),
            "host_ports": {"tcp": 5714, "udp": 5714}, "container": "x",
            "legacy_server_names": (), "client_name": None, "admin": {}})()
        with self.assertRaises(managed_amnezia.ManagedAmneziaError):
            managed_amnezia.plan(foreign, pins={})

    def test_plan_accepts_resolved_tag_reference(self):
        foreign = type("F", (), {"mode": "provision", "server_names": ("a.example.com",),
            "host_ports": {"tcp": 5714, "udp": 5714}, "container": "x",
            "legacy_server_names": (), "client_name": None, "admin": {}})()
        managed = managed_amnezia.plan(foreign,
            pins={"xray": "ghcr.io/xtls/xray-core:26.9.9"})
        self.assertTrue(managed.official)
        self.assertEqual(managed.image, "ghcr.io/xtls/xray-core:26.9.9")
        self.assertEqual(managed.binary, "/usr/local/bin/xray")

    def test_native_inventory_exports_vless_link(self):
        material = managed_amnezia.generate_key_material()
        plan_value = managed_amnezia.ManagedAmneziaPlan("provision", "amnezia-xray",
            "img@sha256:" + "a" * 64, ("example.com",), (), 5714, "guest", {})
        config = managed_amnezia.server_config(plan_value, material)
        entries = managed_amnezia.client_inventory(config)
        self.assertEqual(entries[0]["protocol"], "vless")
        self.assertTrue(entries[0]["payload"].startswith("vless://"))
        self.assertIn(material["uuid"], entries[0]["payload"])
        self.assertIn("pbk=", entries[0]["payload"])


class ProtocolInventoryTests(unittest.TestCase):
    def test_all_guest_protocols_are_aggregated(self):
        plan = {
            "outline": {"users": [{"name": "guest", "secret": "s3"}],
                "tcpPath": "/tcp", "udpPath": "/udp", "host": "example.com"},
            "mtproto": {"secret": "ee00"},
            "hysteria": {"password": "pw", "port": 443},
        }
        entries = install.protocol_page_entries(plan, "example.com")
        self.assertEqual({entry["protocol"] for entry in entries},
            {"outline-wss", "mtproto", "hysteria"})
        self.assertTrue(any(entry["payload"].startswith("hysteria2://") for entry in entries))
        self.assertTrue(any(entry["payload"].startswith("tg://proxy") for entry in entries))
        self.assertTrue(any(entry["payload"].startswith("transport:") for entry in entries))


class SecretShareTests(Base):
    def plan(self):
        return {
            "outline": {"users": [{"name": "alice", "secret": "s3"}],
                "tcpPath": "/tcp", "udpPath": "/udp", "host": "example.com"},
            "mtproto": {"secret": "ee00"},
            "hysteria": {"password": "pw", "port": 443},
        }

    def native(self):
        return [
            {"credentialId": "guest", "displayName": "Guest", "status": "active",
             "payload": "vless://guest@example.com:443"},
            {"credentialId": "retired", "displayName": "Retired", "status": "unavailable",
             "payload": None},
        ]

    def test_only_active_guest_credentials_and_bearer_url_are_published(self):
        artifacts = install.secret_share_artifacts(self.plan(), "example.com",
            self.native(), "secret-path", {"guest": "Primary guest"})
        self.assertEqual(set(artifacts), {"guest.md", "alice.md", "client-page.md"})
        guest = artifacts["guest.md"]
        self.assertIn(b"# Primary guest", guest)
        self.assertIn(b"vless://guest@example.com:443", guest)
        self.assertIn(b"tg://proxy", guest)
        self.assertIn(b"hysteria2://pw@example.com:443", guest)
        alice = artifacts["alice.md"]
        self.assertIn(b"# alice", alice)
        self.assertIn(b"wss://example.com/tcp", alice)
        self.assertIn(b"tg://proxy", alice)
        self.assertIn(b"hysteria2://pw@example.com:443", alice)
        self.assertEqual(artifacts["client-page.md"],
            b"# Client access page\n\nhttps://example.com/secret-path\n\n"
            b"This URL is a bearer secret. Keep this repository private.\n")
        self.assertNotIn(b"retired", b"".join(artifacts.values()))

    def test_cyrillic_guest_name_produces_a_safe_share_artifact(self):
        entries = [{"credentialId": "Гость", "displayName": "Гость",
                    "status": "active", "payload": "vless://guest@example.com:443"}]
        artifacts = install.secret_share_artifacts(self.plan(), "example.com",
            entries, "")
        filename = install.clients.share_name("Гость")
        self.assertIn("# Гость".encode(), artifacts[filename])
        self.assertIn(b"vless://guest@example.com:443", artifacts[filename])
        for name in artifacts:
            self.assertIsNotNone(install._SAFE_SHARE_NAME.match(name))
        self.assertIsNone(install._SAFE_SHARE_NAME.match("../escape"))
        self.assertIsNone(install._SAFE_SHARE_NAME.match("a/b"))

    def test_user_bundles_preserve_case_and_reserved_names(self):
        names = ("Alice", "alice", "a.b", "a-b", "client-page")
        entries = [{"credentialId": name, "displayName": name, "status": "active",
            "payload": "vless://" + name + "@example.com"} for name in names]
        artifacts = install.secret_share_artifacts({}, "example.com", entries, "secret-path")
        self.assertEqual(len(artifacts), len(names) + 1)
        self.assertIn(b"https://example.com/secret-path", artifacts["client-page.md"])
        for name in names:
            self.assertIn(("vless://" + name + "@").encode(), artifacts[install.clients.share_name(name)])

    def test_matching_outline_identity_merges_into_one_user_bundle(self):
        plan = {"outline": {"users": [{"name": "guest", "secret": "s3"}],
                "tcpPath": "/tcp", "udpPath": "/udp", "host": "example.com"},
            "mtproto": {"secret": "ee00"}, "hysteria": {"password": "pw", "port": 443}}
        artifacts = install.secret_share_artifacts(plan, "example.com", self.native(), "")
        self.assertEqual(set(artifacts), {"guest.md"})
        text = artifacts["guest.md"]
        self.assertIn(b"vless://guest@example.com:443", text)
        self.assertIn(b"wss://example.com/tcp", text)
        self.assertIn(b"tg://proxy", text)
        self.assertIn(b"hysteria2://pw@example.com:443", text)

    def test_explicit_empty_path_publishes_no_url(self):
        artifacts = install.secret_share_artifacts(self.plan(), "example.com",
            self.native(), "")
        self.assertNotIn("client-page.md", artifacts)

    def args(self, rotate=False):
        return argparse.Namespace(action="install", json=True, quiet=True, yes=True,
            rotate_client_page=rotate)

    def test_first_install_generates_and_explicit_empty_path_opts_out(self):
        config = Store(self.root / "config", create=True)
        try:
            root = {"core": {"clientAccessPage": {"names": {}, "refreshSeconds": 60}}}
            self.assertTrue(install.ensure_client_page_path(root, self.args(), config))
            path = config.json("hton.yaml")["core"]["clientAccessPage"]["path"]
            self.assertTrue(client_page._valid_path(path))
            self.assertFalse(install.ensure_client_page_path(root, self.args(), config))
            self.assertEqual(config.json("hton.yaml")["core"]["clientAccessPage"]["path"], path)
            opted_out = {"core": {"clientAccessPage": {"path": ""}}}
            self.assertFalse(install.ensure_client_page_path(opted_out, self.args(), config))
            self.assertEqual(opted_out["core"]["clientAccessPage"]["path"], "")
            invalid = {"core": {"clientAccessPage": {"path": 7}}}
            self.assertFalse(install.ensure_client_page_path(invalid, self.args(), config))
            self.assertEqual(invalid["core"]["clientAccessPage"]["path"], 7)
        finally:
            config.close()

    def test_rotation_replaces_the_existing_path(self):
        config = Store(self.root / "config", create=True)
        try:
            root = {"core": {"clientAccessPage": {"path": "old-path"}}}
            self.assertTrue(install.ensure_client_page_path(root, self.args(True), config))
            new_path = config.json("hton.yaml")["core"]["clientAccessPage"]["path"]
            self.assertNotEqual(new_path, "old-path")
            self.assertTrue(client_page._valid_path(new_path))
        finally:
            config.close()

    def test_publication_writes_both_repositories_and_removes_stale_files(self):
        source = self.root / "source"
        runtime = self.root / "runtime"
        for base in (source, runtime):
            (base / ".secrets").mkdir(parents=True)
            (base / ".secrets/share").mkdir()
            (base / ".secrets/share/stale.md").write_text("old")
            (base / ".secrets/share/amnezia-d6.md").write_text("legacy")
            (base / ".secrets/share/mtproto.md").write_text("legacy")
            (base / ".secrets/profile.yml").write_text(
                "core:\n  clientAccessPage:\n    names: {}\n    refreshSeconds: 60\n")
        config = Store(self.root / "config", create=True)
        config.save("hton.yaml", {"core": {"clientAccessPage": {"path": "secret-path"}}})
        work = self.make_work()
        work.save("secret-share.json", {"files": ["stale.md", "old-name.md"]})
        try:
            with patch("install.ROOT", runtime), \
                    patch("install._install_source_root", return_value=source):
                install.publish_secret_share_artifacts(config, work, self.plan(),
                    "example.com", None, "secret-path")
            for base in (source, runtime):
                share = base / ".secrets/share"
                self.assertFalse((share / "stale.md").exists())
                self.assertFalse((share / "amnezia-d6.md").exists())
                self.assertFalse((share / "mtproto.md").exists())
                self.assertEqual((share / "client-page.md").read_bytes(),
                    b"# Client access page\n\nhttps://example.com/secret-path\n\n"
                    b"This URL is a bearer secret. Keep this repository private.\n")
                self.assertTrue((share / "alice.md").is_file())
                self.assertIn(b"tg://proxy", (share / "alice.md").read_bytes())
                self.assertEqual((share / "alice.md").stat().st_mode & 0o777, 0o640)
                profile = (base / ".secrets/profile.yml").read_text()
                self.assertIn('    path: "secret-path"', profile)
            self.assertEqual(work.json("secret-share.json")["files"],
                sorted(["alice.md", "client-page.md"]))
        finally:
            work.close()
            config.close()


class ManagedTransactionTests(Base):
    MODULE = {
        "schemaVersion": 1,
        "moduleId": "foreign-amnezia",
        "settings": {
            "mode": "provision", "container": "amnezia-xray",
            "serverNames": ["static.example.com"],
            "hostPorts": {"tcp": 5714, "udp": 5714},
            "provision": {"clientName": "guest"},
            "admin": {"access": "none"},
        },
    }

    def args(self):
        import argparse
        return argparse.Namespace(action="install", dns_credentials=None, json=False,
            quiet=True, with_features=None, without_features=None, mtproto_camouflage=None,
            outline_binary=None, chain_sni=None, hysteria_port=None,
            amnezia_container=None, site_domain=None, certificate_email=None, yes=False)

    def environment(self, stack, website=None, names=("static.example.com",)):
        certificate = self.root / "cert"
        certificate.mkdir()
        (certificate / "fullchain.pem").write_bytes(b"cert")
        plan = managed_amnezia.ManagedAmneziaPlan("provision", "amnezia-xray",
            "img@sha256:" + "a" * 64, tuple(names), (), 5714, "guest",
            {"access": "none"})
        receipt = {"replacement": "c" * 64, "configDigest": "d" * 64}
        container = {"Id": "c" * 64, "Name": "/amnezia-xray",
            "State": {"Running": False}, "Config": {"Entrypoint": ["/usr/bin/xray"]},
            "HostConfig": {"RestartPolicy": {"Name": "no"},
                "ExtraHosts": ["example.com:host-gateway"]}}
        protocol_plan = {"enabled": [], "documents": {}, "mtproto": None,
            "hysteria": None, "outline": None, "selfsteal": None, "facts": {},
            "generation": "g"}

        def ingress_prepare(work, state, *args, **kwargs):
            state["candidateIngress"] = {"site": "1" * 64, "edge": "2" * 64}
            return {}

        def accept(state):
            state["activeProtocols"] = state.pop("candidateProtocols", {})

        patches = [
            patch("install.ROOT", self.root),
            patch("install._install_source_root", return_value=self.root),
            patch("install.load_settings", return_value={}),
            patch("install.load_site_inputs", return_value=("example.com", "op@example.com")),
            patch("managed_amnezia.plan", return_value=plan),
            patch("install.docker.inspect", return_value=container),
            patch("install.docker.qualify", return_value={"nativeDigest": "n" * 64}),
            patch("install.docker.inventory", return_value={}),
            patch("install.docker.command", return_value=b""),
            patch("install.docker.native_config", return_value=b'{"inbounds":[{"streamSettings":{"realitySettings":{"dest":"example.com:9443","serverNames":["static.example.com"],"minClientVer":"1.0.0"}}}]}'),
            patch("install.docker.native_bridge_gateway", return_value="172.17.0.1"),
            patch("install.docker.bridge_gateway", return_value="172.17.0.1"),
            patch("certificates.select_policy", return_value="dns-01"),
            patch("certificates.ensure", return_value=certificate),
            patch("install.ingress.webroot", return_value=b"<html>x</html>"),
            patch("protocols.selected", return_value=[]),
            patch("protocols.plan", return_value=protocol_plan),
            patch("protocols.record_state"),
            patch("protocols.save_config"),
            patch("protocols.validate"),
            patch("protocols.prepare"),
            patch("protocols.start_chain"),
            patch("protocols.start_candidates"),
            patch("protocols.probe"),
            patch("protocols.promote"),
            patch("protocols.accept", side_effect=accept),
            patch("protocols.rollback_candidates"),
            patch("protocols.share_links", return_value={}),
            patch("install.ingress.prepare", side_effect=ingress_prepare),
            patch("install.ingress.owned", return_value={"State": {"Running": False}}),
            patch("install.ingress.website", side_effect=website or (lambda *a: None)),
            patch("install.ingress.authentication"),
            patch("install.install_units"),
            patch("install.timer"),
            patch("install.refresh_client_page", return_value={"enabled": False}),
            patch("install.ready_result", return_value={}),
            patch("install.emit"),
            patch("install.boot_id", return_value="boot"),
            patch("install.process_identity", return_value={"pid": 123}),
            patch("install.os.setsid"),
            patch("install.time.sleep"),
            patch("install.time.monotonic", return_value=1.0),
            patch("install.time.time", return_value=1.0),
        ]
        prepare = stack.enter_context(patch("managed_amnezia.prepare", return_value=receipt))
        for item in patches:
            stack.enter_context(item)
        return prepare

    def test_happy_path_commits_managed_state(self):
        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            import contextlib
            with contextlib.ExitStack() as stack:
                self.environment(stack)
                install.run_managed(config, work, {}, self.MODULE, self.args(),
                    pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
                ensure = install.certificates.ensure
            state = work.json("managed.json")
            self.assertEqual(state["kind"], "managed")
            self.assertEqual(state["phase"], "committed")
            self.assertEqual(state["container"], "c" * 64)
            self.assertEqual(state["port"], 5714)
            self.assertEqual(state["activeIngress"], {"site": "1" * 64, "edge": "2" * 64})
            self.assertIsNone(work.read("state.json"))
            self.assertEqual(ensure.call_args.kwargs["names"],
                             ["www.example.com", "static.example.com"])
            self.assertFalse((self.root / "cert" / "reality-fullchain.pem").exists())
        finally:
            config.close()
            work.close()

    def test_install_replaces_reality_subdomains_with_the_apex(self):
        self.assertEqual(install.apex_server_names("example.com",
            ["static.example.com", "assets.example.com"]), ["example.com"])
        self.assertEqual(install.apex_server_names("example.com",
            ["example.com", "static.example.com"]), ["example.com", "static.example.com"])
        self.assertEqual(install.apex_server_names("example.com",
            ["static.example.com", "legacy.third-party.test"]),
            ["static.example.com", "legacy.third-party.test"])

    def test_managed_install_passes_apex_names_to_the_plan(self):
        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            import contextlib
            with contextlib.ExitStack() as stack:
                self.environment(stack)
                stack.enter_context(patch("install.sys.stdin.isatty", return_value=True))
                args = self.args()
                args.quiet = False
                args.yes = True
                install.run_managed(config, work, {}, self.MODULE, args,
                    pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
                plan_call = install.managed_amnezia.plan.call_args
            self.assertEqual(plan_call.args[0]["server_names"], ("example.com",))
            self.assertEqual(work.json("managed.json")["phase"], "committed")
        finally:
            config.close()
            work.close()

    def test_apex_reality_name_nests_behind_the_site(self):
        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            calls = []
            import contextlib

            def record(work_value, state, *args, **kwargs):
                calls.append((args, kwargs))
                state["candidateIngress"] = {"site": "1" * 64, "edge": "2" * 64}
                return {}

            with contextlib.ExitStack() as stack:
                self.environment(stack, names=("example.com",))
                stack.enter_context(patch("install.ingress.prepare", side_effect=record))
                install.run_managed(config, work, {}, self.MODULE, self.args(),
                    pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
                ensure = install.certificates.ensure
            self.assertTrue(calls[0][1]["native_selfsteal"])
            self.assertEqual(calls[0][1]["reality_names"], ())
            self.assertEqual(ensure.call_args.kwargs["names"], ["www.example.com"])
            self.assertFalse(ensure.call_args.kwargs["wildcard"])
        finally:
            config.close()
            work.close()

    def test_reality_names_are_limited_to_own_domain(self):
        self.assertEqual(install.reality_certificate_names("example.com",
            ["static.example.com", "example.com", "legacy.third-party.test"]),
            ["static.example.com"])

    def test_website_failure_rolls_back_and_retains_receipt(self):
        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            import contextlib

            def broken(*_args):
                raise OSError("tls")

            with contextlib.ExitStack() as stack:
                self.environment(stack, website=broken)
                with self.assertRaises(InstallError):
                    install.run_managed(config, work, {}, self.MODULE, self.args(),
                        pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
            state = work.json("managed.json")
            self.assertEqual(state["phase"], "rolled_back")
            self.assertEqual(state["container"], "c" * 64)
        finally:
            config.close()
            work.close()

    def test_second_run_does_not_recreate_container(self):
        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            import contextlib
            with contextlib.ExitStack() as stack:
                prepare = self.environment(stack)
                install.run_managed(config, work, {}, self.MODULE, self.args(),
                    pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
                self.assertEqual(prepare.call_count, 1)
                install.run_managed(config, work, {}, self.MODULE, self.args(),
                    pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
                self.assertEqual(prepare.call_count, 1)
        finally:
            config.close()
            work.close()

    def managed_state(self, work):
        work.save("managed.json", {"schemaVersion": 1, "kind": "managed",
            "instance": "a" * 32, "operation": "b" * 32, "container": "c" * 64,
            "port": 5714, "sitePort": 9443, "phase": "committed", "siteDomain": "example.com",
            "certificateEmail": "op@example.com"})

    def test_ambiguous_inspect_error_does_not_destroy_the_container(self):
        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            import contextlib
            self.managed_state(work)
            with contextlib.ExitStack() as stack:
                prepare = self.environment(stack)
                stack.enter_context(patch("install.docker.inspect",
                    side_effect=docker.NativeError("Read container settings", 1,
                        "The command did not finish successfully.")))
                with self.assertRaises(InstallError):
                    install.run_managed(config, work, {}, self.MODULE, self.args(),
                        pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
                self.assertEqual(prepare.call_count, 0)
            self.assertEqual(work.json("managed.json")["container"], "c" * 64)
        finally:
            config.close()
            work.close()

    def test_recreation_reuses_the_live_client_array(self):
        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            import contextlib
            self.managed_state(work)
            plan_value = managed_amnezia.ManagedAmneziaPlan("provision", "amnezia-xray",
                "img@sha256:" + "a" * 64, ("static.example.com",), (), 5714, "guest",
                {"access": "none"})
            material = managed_amnezia.generate_key_material()
            material["clients"] = [
                {"id": material["uuid"], "email": "guest", "flow": "xtls-rprx-vision"},
                {"id": "11111111-2222-3333-4444-555555555555", "email": "phone",
                 "flow": "xtls-rprx-vision"},
            ]
            live = managed_amnezia.server_config(plan_value, material, dest="old.example:9443")
            with contextlib.ExitStack() as stack:
                prepare = self.environment(stack)
                stack.enter_context(patch("install.docker.native_config", return_value=live))
                install.run_managed(config, work, {}, self.MODULE, self.args(),
                    pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
            self.assertEqual(prepare.call_count, 1)
            sent = prepare.call_args.kwargs["material"]
            self.assertEqual(sent["privateKey"], material["privateKey"])
            self.assertEqual(sent["clients"], material["clients"])
        finally:
            config.close()
            work.close()

    def test_failed_certificate_preparation_keeps_previous_native_running(self):
        import contextlib

        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            self.managed_state(work)
            plan = managed_amnezia.ManagedAmneziaPlan("provision", "amnezia-xray",
                "img@sha256:" + "a" * 64, ("static.example.com",), (), 5714, "guest",
                {"access": "none"})
            live = managed_amnezia.server_config(plan, managed_amnezia.generate_key_material(),
                dest="old.example:9443")
            previous_id, candidate_id = "c" * 64, "f" * 64
            with contextlib.ExitStack() as stack:
                prepare = self.environment(stack)
                prepare.return_value = {"replacement": candidate_id, "configDigest": "d" * 64}
                stack.enter_context(patch("install.docker.native_config", return_value=live))
                stack.enter_context(patch("certificates.ensure", side_effect=InstallError("fixture failure")))
                commands = stack.enter_context(patch("install.docker.command", return_value=b""))
                with self.assertRaisesRegex(InstallError, "fixture failure"):
                    install.run_managed(config, work, {}, self.MODULE, self.args(),
                        pins_override={"amnezia-xray": plan.image})
            self.assertEqual(work.json("managed.json")["container"], previous_id)
            self.assertFalse(any(previous_id in call.args for call in commands.call_args_list))
            commands.assert_any_call("container", "rm", "--force", candidate_id)
        finally:
            config.close()
            work.close()

    def old_ingress_state(self, work):
        state = work.json("managed.json")
        state["activeIngress"] = {"site": "9" * 64, "edge": "8" * 64}
        state["retainedIngress"] = [{"site": "7" * 64, "edge": "6" * 64}]
        state["ownedReceipts"] = {identifier: {"role": role} for identifier, role in (
            ("9" * 64, "site"), ("8" * 64, "edge"),
            ("7" * 64, "site"), ("6" * 64, "edge"))}
        work.save("managed.json", state)

    def test_reinstall_retires_the_previous_ingress(self):
        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            import contextlib
            self.managed_state(work)
            self.old_ingress_state(work)
            with contextlib.ExitStack() as stack:
                self.environment(stack)
                commands = stack.enter_context(patch("install.docker.command", return_value=b""))
                install.run_managed(config, work, {}, self.MODULE, self.args(),
                    pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
            after = work.json("managed.json")
            self.assertEqual(after["activeIngress"]["site"], "1" * 64)
            self.assertEqual(after["retainedIngress"], [])
            self.assertNotIn("suspendedIngress", after)
            removed = {call.args[-1] for call in commands.call_args_list
                if call.args[:2] == ("container", "rm")}
            self.assertEqual(removed, {"9" * 64, "8" * 64, "7" * 64, "6" * 64})
        finally:
            config.close()
            work.close()

    def test_failed_reinstall_restores_previous_site_and_vpn(self):
        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            import contextlib

            def running(value):
                if value == "c" * 64:
                    return {"Id": value, "State": {"Running": True},
                        "Config": {"Entrypoint": ["/usr/bin/xray"]},
                        "HostConfig": {"ExtraHosts": ["example.com:host-gateway"],
                            "RestartPolicy": {"Name": "always"}}}
                return {"Id": value, "State": {"Running": False}}

            def broken(*_args):
                raise InstallError("HTTPS body differs from the immutable website artifact")

            self.managed_state(work)
            self.old_ingress_state(work)
            with contextlib.ExitStack() as stack:
                self.environment(stack, website=broken)
                commands = stack.enter_context(patch("install.docker.command", return_value=b""))
                stack.enter_context(patch("install.docker.inspect", side_effect=running))
                with self.assertRaises(InstallError):
                    install.run_managed(config, work, {}, self.MODULE, self.args(),
                        pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
            self.assertEqual(work.json("managed.json")["phase"], "rolled_back")
            restarted = {call.args[-1] for call in commands.call_args_list
                if call.args[:3] == ("container", "update", "--restart=always")}
            self.assertTrue({"9" * 64, "8" * 64, "7" * 64, "6" * 64} <= restarted)
            started = {call.args[-1] for call in commands.call_args_list
                if call.args[:2] == ("container", "start")}
            self.assertIn("c" * 64, started)
        finally:
            config.close()
            work.close()

    def test_missing_container_is_recreated_from_saved_material(self):
        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            import contextlib
            self.managed_state(work)
            with contextlib.ExitStack() as stack:
                prepare = self.environment(stack)
                seen = []

                def inspect(value):
                    if value == "c" * 64 and not seen:
                        seen.append(value)
                        raise docker.NativeError("Read container settings", 1,
                            "The container no longer exists.")
                    return {"Id": value, "State": {"Running": False},
                        "Config": {"Entrypoint": ["/usr/bin/xray"]}, "HostConfig": {}}

                stack.enter_context(patch("install.docker.inspect", side_effect=inspect))
                stack.enter_context(patch("managed_amnezia.saved_material",
                    return_value=managed_amnezia.generate_key_material()))
                install.run_managed(config, work, {}, self.MODULE, self.args(),
                    pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
                self.assertEqual(prepare.call_count, 1)
        finally:
            config.close()
            work.close()

    def site_args(self, **overrides):
        values = {"action": "install", "json": False, "quiet": False, "yes": False}
        values.update(overrides)
        return argparse.Namespace(**values)

    def test_configured_site_root_is_used_without_asking(self):
        site = self.root / "site"
        site.mkdir()
        (site / "index.html").write_text("real", encoding="utf-8")
        document = {"core": {"site": {"root": str(site)}}}
        with patch("install.prompt", side_effect=AssertionError("must not ask")):
            self.assertEqual(install.resolve_site_root(document, "example.com",
                self.site_args()), site)
        with self.assertRaises(InstallError):
            install.resolve_site_root({"core": {"site": {"root": "/missing/site"}}},
                "example.com", self.site_args())

    def test_detected_site_is_adopted_after_one_question(self):
        site = self.root / "www"
        site.mkdir()
        (site / "index.html").write_text("real", encoding="utf-8")
        document = {}
        with patch("install.ingress.site_root", return_value=site), \
             patch("install.sys.stdin.isatty", return_value=True), \
             patch("install.prompt", return_value="") as asked:
            self.assertEqual(install.resolve_site_root(document, "example.com",
                self.site_args()), site)
        asked.assert_called_once()
        self.assertIn(str(site), asked.call_args.args[0])
        self.assertIn("example.com", asked.call_args.args[0])
        self.assertEqual(document["core"]["site"]["root"], str(site))

    def test_declining_the_detected_site_keeps_the_placeholder(self):
        site = self.root / "www"
        site.mkdir()
        (site / "index.html").write_text("real", encoding="utf-8")
        document = {}
        with patch("install.ingress.site_root", return_value=site), \
             patch("install.sys.stdin.isatty", return_value=True), \
             patch("install.prompt", return_value="n"):
            self.assertIsNone(install.resolve_site_root(document, "example.com",
                self.site_args()))
        self.assertIs(document["core"]["site"]["useExisting"], False)
        with patch("install.ingress.site_root", return_value=site), \
             patch("install.prompt", side_effect=AssertionError("must not ask")):
            self.assertIsNone(install.resolve_site_root(document, "example.com",
                self.site_args()))

    def test_quiet_install_adopts_detected_site_without_asking(self):
        site = self.root / "www"
        site.mkdir()
        (site / "index.html").write_text("real", encoding="utf-8")
        document = {}
        with patch("install.ingress.site_root", return_value=site), \
             patch("install.prompt", side_effect=AssertionError("must not ask")):
            self.assertEqual(install.resolve_site_root(document, "example.com",
                self.site_args(quiet=True)), site)
        with patch("install.ingress.site_root", return_value=None), \
             patch("install.prompt", side_effect=AssertionError("must not ask")):
            self.assertIsNone(install.resolve_site_root({}, "example.com",
                self.site_args()))

    def test_configured_site_root_reaches_the_renderer(self):
        config = Store(self.root / "config", create=True)
        work = self.make_work()
        try:
            import contextlib
            site = self.root / "site"
            site.mkdir()
            (site / "index.html").write_text("real", encoding="utf-8")
            with contextlib.ExitStack() as stack:
                self.environment(stack)
                webroot = stack.enter_context(patch("install.ingress.webroot",
                    return_value=b"<html>real</html>"))
                install.run_managed(config, work, {"core": {"site": {"root": str(site)}}},
                    self.MODULE, self.args(),
                    pins_override={"amnezia-xray": "img@sha256:" + "a" * 64})
            self.assertEqual(webroot.call_args.args, (str(site),))
        finally:
            config.close()
            work.close()

    def test_managed_state_validation(self):
        work = self.make_work()
        try:
            work.save("managed.json", {"schemaVersion": 1, "kind": "managed",
                "instance": "a" * 32, "operation": "b" * 32, "port": 4000,
                "phase": "committed"})
            with self.assertRaises(InstallError):
                install.managed_state_record(work)
        finally:
            work.close()


class SavedMaterialTests(Base):
    def test_saved_material_preserves_keys_and_every_client(self):
        work = Store(self.root / "work", create=True)
        try:
            material = managed_amnezia.generate_key_material()
            material["clients"] = [
                {"id": material["uuid"], "email": "guest", "flow": "xtls-rprx-vision"},
                {"id": "11111111-2222-3333-4444-555555555555", "email": "phone",
                 "flow": "xtls-rprx-vision"},
            ]
            plan_value = managed_amnezia.ManagedAmneziaPlan("provision", "amnezia-xray", None,
                ("static.example.com",), (), 5714, "guest", {})
            first = managed_amnezia.server_config(plan_value, material)
            work.write("amnezia/server.json", first)
            saved = managed_amnezia.saved_material(work)
            self.assertEqual(saved["privateKey"], material["privateKey"])
            self.assertEqual(saved["publicKey"], material["publicKey"])
            self.assertEqual(saved["shortIds"], [material["shortId"]])
            self.assertEqual(saved["clients"], material["clients"])
            self.assertEqual(managed_amnezia.server_config(plan_value, saved), first)
        finally:
            work.close()

    def test_corrupt_saved_material_blocks_instead_of_rerolling(self):
        work = Store(self.root / "work", create=True)
        try:
            work.write("amnezia/server.json", b"{}")
            with self.assertRaises(managed_amnezia.ManagedAmneziaError):
                managed_amnezia.saved_material(work)
        finally:
            work.close()

    def test_prepare_reuses_saved_keys_and_clients(self):
        work = Store(self.root / "work", create=True)
        try:
            material = managed_amnezia.generate_key_material()
            material["clients"] = [
                {"id": material["uuid"], "email": "guest", "flow": "xtls-rprx-vision"},
                {"id": "11111111-2222-3333-4444-555555555555", "email": "phone",
                 "flow": "xtls-rprx-vision"},
            ]
            plan_value = managed_amnezia.ManagedAmneziaPlan("provision", "amnezia-xray",
                "img@sha256:" + "a" * 64, ("static.example.com",), (), 5714, "guest", {})
            work.write("amnezia/server.json", managed_amnezia.server_config(plan_value, material))
            with patch("managed_amnezia.generate_key_material",
                       side_effect=AssertionError("keys must not be rerolled")), \
                 patch("docker.exact_id", return_value="d" * 64), \
                 patch("docker.command", return_value=b""), \
                 patch("docker.bridge_gateway", return_value="172.17.0.1"), \
                 patch("docker.native_config",
                       side_effect=lambda _container: work.read("amnezia/server.json")):
                receipt = managed_amnezia.prepare(plan_value, work)
            self.assertEqual(receipt["uuid"], material["uuid"])
            self.assertEqual(receipt["shortId"], material["shortId"])
            saved = json.loads(work.read("amnezia/server.json"))
            self.assertEqual(saved["inbounds"][0]["settings"]["clients"], material["clients"])
            self.assertEqual(saved["inbounds"][0]["streamSettings"]["realitySettings"]
                ["privateKey"], material["privateKey"])
        finally:
            work.close()



class NativeAmneziaReportTests(Base):
    def native(self):
        material = managed_amnezia.generate_key_material()
        plan_value = managed_amnezia.ManagedAmneziaPlan("provision", "amnezia-xray", None,
            ("static.example.com",), (), 5714, "guest", {})
        return material, managed_amnezia.server_config(plan_value, material)

    def test_guest_reality_link_is_saved_and_reported(self):
        work = self.make_work()
        try:
            material, native = self.native()
            with patch("install.docker.native_config", return_value=native):
                links, shares = install.native_share_links(work, "c" * 64)
            expected = managed_amnezia.vless_link(material["uuid"], "static.example.com",
                "XRay", material["publicKey"], material["shortId"])
            self.assertEqual(list(links), ["Amnezia"])
            self.assertEqual(links["Amnezia"], expected)
            self.assertTrue(links["Amnezia"].startswith(
                "vless://" + material["uuid"] + "@static.example.com:443?"))
            self.assertIn("security=reality", links["Amnezia"])
            self.assertEqual(shares["Amnezia"], str(work.root / "share/amnezia-guest.txt"))
            self.assertEqual(work.read("share/amnezia-guest.txt"), (expected + "\n").encode())
        finally:
            work.close()

    def test_native_share_link_uses_connection_label_for_fragment(self):
        from urllib.parse import quote
        work = self.make_work()
        try:
            material, native = self.native()
            with patch("install.docker.native_config", return_value=native):
                links, shares = install.native_share_links(work, "c" * 64, "🇷🇺 Россия")
            expected = managed_amnezia.vless_link(material["uuid"], "static.example.com",
                "🇷🇺 Россия XRay", material["publicKey"], material["shortId"])
            self.assertEqual(links["Amnezia"], expected)
            self.assertTrue(links["Amnezia"].endswith("#" + quote("🇷🇺 Россия XRay")))
            self.assertEqual(shares["Amnezia"], str(work.root / "share/amnezia-guest.txt"))
        finally:
            work.close()

    def test_runtime_tree_becomes_operator_readable(self):
        work = self.make_work()
        try:
            work.write("share/legacy.txt", b"legacy")
            path = work.root / "share/legacy.txt"
            os.chmod(path, 0o600)
            install.normalize_runtime_permissions(work)
            self.assertEqual(path.stat().st_mode & 0o777, work.file_mode)
            self.assertEqual((work.root / "share").stat().st_mode & 0o777, work.dir_mode)
            if storage.group_id() is not None:
                self.assertEqual(path.stat().st_gid, storage.group_id())
                self.assertNotEqual(path.stat().st_mode & 0o040, 0)
        finally:
            work.close()

    def test_unreadable_native_config_does_not_fail_install(self):
        work = self.make_work()
        try:
            with patch("install.docker.native_config", return_value=b"{}"):
                self.assertEqual(install.native_share_links(work, "c" * 64), ({}, {}))
        finally:
            work.close()

    def test_ready_summary_names_amnezia(self):
        work = self.make_work()
        try:
            plan = {"enabled": [], "mtproto": None, "hysteria": None,
                "outline": None, "selfsteal": None}
            link = "vless://guest@static.example.com:443?security=reality"
            path = str(work.root / "share/amnezia-guest.txt")
            result = install.ready_result("example.com", 5714, "dns-01", "verified", None, plan, work,
                native_links={"Amnezia": link}, native_shares={"Amnezia": path})
            self.assertEqual(result["protocols"]["amnezia"], {"listener": "reality/tcp/443", "guestLinks": 1})
            self.assertEqual(result["shareFiles"]["Amnezia"], path)
            stream = io.StringIO()
            with patch("install.sys.stdout", stream):
                install.emit(argparse.Namespace(quiet=False, json=False), result,
                    page_url="https://example.com/secret",
                    share_directory="/opt/hton/.secrets/share")
            text = stream.getvalue()
            self.assertIn("Amnezia: REALITY on TCP 443 (guest links: 1).", text)
            self.assertIn("Client access page: https://example.com/secret", text)
            self.assertIn("Client files: /opt/hton/.secrets/share", text)
            self.assertNotIn(link, text)
        finally:
            work.close()


class ImagePolicyTests(unittest.TestCase):
    def test_hybrid_pulls_source_tag(self):
        lock = {"images": {"caddy": {
            "reference": "docker.io/library/caddy@sha256:" + "a" * 64,
            "sourceTag": "2-alpine"}}}
        image_id = "sha256:" + "1" * 64
        with patch("install.docker.command", side_effect=[b"", image_id.encode()]) as command:
            pins = install.resolve_pins(lock, ["caddy"])
        self.assertEqual(command.call_args_list[0].args,
            ("image", "pull", "--quiet", "docker.io/library/caddy:2-alpine"))
        self.assertEqual(pins["caddy"], image_id)

    def test_digest_fallback_without_tag(self):
        reference = "docker.io/example/x@sha256:" + "b" * 64
        image_id = "sha256:" + "2" * 64
        with patch("install.docker.command", side_effect=[b"", image_id.encode()]) as command:
            pins = install.resolve_pins({"images": {"x": {"reference": reference}}}, ["x"])
        self.assertEqual(command.call_args_list[0].args, ("image", "pull", "--quiet", reference))
        self.assertEqual(pins["x"], image_id)

    def test_missing_image_entry_raises(self):
        with self.assertRaises(InstallError):
            install.resolve_pins({"images": {}}, ["caddy"])


class SecretsProfileTests(Base):
    def test_init_seeds_single_profile_and_imports(self):
        result = secrets_store.init(self.root, assume_yes=True)
        self.assertFalse(result["status"]["repo_complete"])
        self.assertFalse(result["status"]["config_complete"])
        self.assertTrue((self.root / ".secrets/profile.yml").is_file())
        self.assertTrue((self.root / ".secrets/images.lock.json").is_file())
        import yaml
        profile = yaml.safe_load((self.root / ".secrets/profile.yml").read_text())
        for dotted, _description, _default, _relative in secrets_store.SECRET_FIELDS:
            parts = dotted.split(".")
            current = profile
            for part in parts[:-1]:
                current = current[part]
            current[parts[-1]] = "value-" + dotted
        (self.root / ".secrets/profile.yml").write_text(
            yaml.safe_dump(profile, sort_keys=False))
        receipt = secrets_store.import_secrets(self.root)
        self.assertTrue(receipt["imported"])
        status = secrets_store.status(self.root)
        self.assertTrue(status["config_complete"])
        store = Store(self.root / "config")
        try:
            self.assertIsNotNone(store.read("hton.yaml"))
            self.assertIsNotNone(store.read("modules/foreign-amnezia.yaml"))
            self.assertIsNotNone(store.read("locks/images.lock.json"))
            self.assertIsNotNone(store.read("site/index.html"))
            credentials = store.json("secrets/minimum-internetbs.json")
            self.assertEqual(credentials["apiKey"], "value-secrets.dns.internetbsApiKey")
        finally:
            store.close()

    def test_site_root_imports_into_root_config(self):
        import yaml
        secrets_store.init(self.root, assume_yes=True)
        path = self.root / ".secrets/profile.yml"
        profile = yaml.safe_load(path.read_text())
        for dotted, _description, _default, _relative in secrets_store.SECRET_FIELDS:
            parts = dotted.split(".")
            current = profile
            for part in parts[:-1]:
                current = current[part]
            current[parts[-1]] = "value-" + dotted
        profile["site"]["root"] = "/srv/site"
        path.write_text(yaml.safe_dump(profile, sort_keys=False))
        secrets_store.import_secrets(self.root)
        store = Store(self.root / "config")
        try:
            root = yaml.safe_load(store.read("hton.yaml"))
            self.assertEqual(root["core"]["site"]["root"], "/srv/site")
        finally:
            store.close()

    def test_module_enabled_flag_drives_features(self):
        import yaml
        secrets_store.init(self.root, assume_yes=True)
        path = self.root / ".secrets/profile.yml"
        profile = yaml.safe_load(path.read_text())
        profile["modules"]["mtproto"]["enabled"] = False
        path.write_text(yaml.safe_dump(profile, sort_keys=False))
        secrets_store.import_secrets(self.root)
        store = Store(self.root / "config")
        try:
            root = yaml.safe_load(store.read("hton.yaml"))
            self.assertFalse(root["features"]["mtproto"]["enabled"])
            self.assertTrue(root["features"]["foreign-amnezia"]["enabled"])
            module = yaml.safe_load(store.read("modules/mtproto.yaml"))
            self.assertEqual(module["settings"]["backend"], "mtg")
        finally:
            store.close()

    def test_http01_profile_does_not_require_dns_keys(self):
        secrets_store.init(self.root, assume_yes=True)
        secrets_store.set_value(self.root, "root.core.certificates.challenge", "http-01")
        secrets_store.set_value(self.root, "secrets.amnezia.adminSshKey", "key-material")
        secrets_store.set_value(self.root, "secrets.hysteria.password", "password-material")
        current = secrets_store.status(self.root)
        self.assertTrue(current["repo_complete"])
        dns = [entry for entry in current["entries"] if entry["path"].startswith("secrets.dns.")]
        self.assertTrue(dns and all(not entry["required"] for entry in dns))

    def test_ensure_non_interactive_requires_profile(self):
        with self.assertRaises(InstallError):
            secrets_store.ensure(self.root, interactive=False)

    def test_status_reports_no_values(self):
        import json
        secrets_store.init(self.root, assume_yes=True)
        text = json.dumps(secrets_store.status(self.root))
        self.assertNotIn("CHANGE-ME", text)
        self.assertNotIn("assets/stream", text)


class IntegrationTests(Base):
    def test_install_units_adds_client_page_when_enabled(self):
        with patch("install.ROOT", Path("/opt/hton-fixture")), \
                patch("install.unit_preflight"), patch("docker.run"):
            install.install_units(self.store, client_page_enabled=True, refresh_seconds=30)
        self.assertIn(b"client-page-refresh", self.store.read("units/hton-minimum-client-page.service"))
        self.assertIn(b"OnUnitActiveSec=30s", self.store.read("units/hton-minimum-client-page.timer"))
        self.assertIsNone(self.store.read("units/hton-minimum-client-page.service.off"))

    def test_install_units_omits_client_page_when_disabled(self):
        with patch("install.ROOT", Path("/opt/hton-fixture")), \
                patch("install.unit_preflight"), patch("docker.run"):
            install.install_units(self.store, client_page_enabled=False)
        self.assertIsNone(self.store.read("units/hton-minimum-client-page.service"))

    def test_refresh_client_page_withdraws_for_disabled_config(self):
        tree = build_tree(self.root)
        config = Store(tree)
        work = Store(self.root / "work", create=True)
        try:
            receipt = install.refresh_client_page(config, work, {})
            self.assertTrue(receipt["withdrawn"])
        finally:
            config.close()
            work.close()

    def setUp(self):
        super().setUp()
        self.store = Store(self.root / "units-store", create=True)

    def tearDown(self):
        self.store.close()
        super().tearDown()


if __name__ == "__main__":
    unittest.main()
