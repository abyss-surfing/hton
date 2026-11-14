"""Unit acceptance of the selected minimum installation closure."""
import copy
import argparse
import json
import io
import os
import pty
import select
import signal
import time
from pathlib import Path
import sys
import tempfile
import shutil
import subprocess
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'python/minimum'))
import docker
import ingress
import install
import install_config
import updates
import migrations
import protocols
import certificates
import publication
import http_challenge
import http.server
import functools
import threading
from storage import Store, InstallError, document


def native():
    return {'Id': 'a'*64, 'Name': '/native', 'Image': 'sha256:'+'b'*64,
        'Config': {'Image': 'native:latest', 'Env': ['KEY=preserved'], 'Labels': {}, 'Entrypoint': ['/start'], 'Cmd': ['run']},
        'HostConfig': {'RestartPolicy': {'Name': 'always', 'MaximumRetryCount': 0},
            'PortBindings': {'443/tcp': [{'HostIp': '', 'HostPort': '443'}]}, 'NetworkMode': 'bridge', 'Memory': 134217728},
        'Mounts': [], 'State': {'Running': True}}


class MinimumTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.store = Store(self.root)

    def tearDown(self):
        self.store.close()
        self.temp.cleanup()

    def test_roundtrip_and_protected_modes(self):
        self.store.save('nested/state.json', {'port': 5512})
        self.assertEqual(self.store.json('nested/state.json'), {'port': 5512})
        self.assertEqual((self.root/'nested').stat().st_mode & 0o777, self.store.dir_mode)
        self.assertEqual((self.root/'nested/state.json').stat().st_mode & 0o777, self.store.file_mode)

    def test_log_rotation_forgets_entries_older_than_a_week(self):
        directory = self.root/'logs'
        directory.mkdir()
        log = directory/'minimum.log'
        now = time.time()
        log.write_text(json.dumps({'time': int(now - 8*24*3600), 'phase': 'old'})+'\n')
        install._rotate_log(log, os.getgid(), now)
        self.assertFalse(log.exists())
        archived = directory/'minimum.log.1'
        self.assertTrue(archived.exists())
        install._prune_logs(directory, now)
        self.assertFalse(archived.exists())

    def test_log_rotation_and_pruning_keep_recent_material(self):
        directory = self.root/'logs'
        directory.mkdir()
        log = directory/'minimum.log'
        now = time.time()
        log.write_text(json.dumps({'time': int(now - 3600)})+'\n')
        install._rotate_log(log, os.getgid(), now)
        self.assertTrue(log.exists())
        self.assertFalse((directory/'minimum.log.1').exists())
        with patch.object(install, 'LOG_MAX_BYTES', 10):
            install._rotate_log(log, os.getgid(), now)
        self.assertTrue((directory/'minimum.log.1').exists())
        install._prune_logs(directory, now)
        self.assertTrue((directory/'minimum.log.1').exists())
        target = directory/'outside'
        target.write_text('keep')
        (directory/'minimum.log.9').symlink_to(target)
        install._prune_logs(directory, now + 10*24*3600)
        self.assertEqual(target.read_text(), 'keep')

    def test_write_and_read_refuse_symlink_hardlink_and_public_file(self):
        self.store.write('real', b'original')
        (self.root/'link').symlink_to('real')
        os.link(self.root/'real', self.root/'hard')
        for name in ('link', 'hard', 'real'):
            with self.assertRaises((InstallError, OSError)):
                self.store.write(name, b'changed')
            with self.assertRaises((InstallError, OSError)):
                self.store.read(name)
        (self.root/'hard').unlink()
        os.chmod(self.root/'real', 0o644)
        with self.assertRaises(InstallError): self.store.write('real', b'changed')
        self.assertEqual((self.root/'real').read_bytes(), b'original')

    def test_invalid_json_and_duplicate_yaml_rejected(self):
        for data in (b'{"port":1,"port":2}', b'{"port":NaN}'):
            self.store.write('state', data)
            with self.assertRaises(InstallError): self.store.json('state')
        self.store.write('settings', b'port: 5001\nport: 5002\n')
        with self.assertRaises(InstallError): document(self.store, 'settings')

    def test_failed_atomic_replace_keeps_old_state(self):
        self.store.write('state', b'original')
        with patch('storage.os.replace', side_effect=OSError('disk error')):
            with self.assertRaises(OSError): self.store.write('state', b'new')
        self.assertEqual(self.store.read('state'), b'original')
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), ['state'])

    def test_no_path_escape(self):
        for name in ('../escape', '/escape', 'child/../../escape'):
            with self.assertRaises(InstallError): self.store.write(name, b'bad')

    def test_launch_preservation_and_loopback(self):
        original = native()
        before = copy.deepcopy(original)
        payload = docker.create_payload(original, 'sha256:'+'c'*64, 5521)
        self.assertEqual(original, before)
        host = payload.pop('HostConfig')
        payload['Image'] = original['Config']['Image']
        self.assertEqual(payload, original['Config'])
        bindings = host.pop('PortBindings')
        host['PortBindings'] = original['HostConfig']['PortBindings']
        host['RestartPolicy'] = original['HostConfig']['RestartPolicy']
        self.assertEqual(host, original['HostConfig'])
        for key in ('443/tcp','443/udp'):
            self.assertEqual(bindings[key], [{'HostIp':'127.0.0.1','HostPort':'5521'}])
        for port in (True, 4999, 6001, '5500'):
            with self.assertRaises(InstallError): docker.create_payload(original, 'snapshot', port)

    def test_docker_nullable_default_is_canonical_without_changing_original(self):
        value=native();value['HostConfig']['OomKillDisable']=None
        payload=docker.create_payload(value,'snapshot',5512)
        self.assertIs(payload['HostConfig']['OomKillDisable'],False)
        self.assertIsNone(value['HostConfig']['OomKillDisable'])

    def test_native_dns_network_reaches_native_qualification(self):
        value=native()
        value['HostConfig']['PortBindings']['443/udp']=[{'HostIp':'','HostPort':'443'}]
        value['NetworkSettings']={'Networks':{'bridge':{'NetworkID':'b'*64,'IPAMConfig':None},
            'amnezia-dns-net':{'NetworkID':'d'*64,'IPAMConfig':{}}}}
        with patch('docker.native_config',side_effect=InstallError('native qualification reached')):
            with self.assertRaisesRegex(InstallError,'native qualification reached'):
                docker.qualify(value)

    def test_network_attachments_preserved_and_drift_rejected(self):
        value=native()
        value['NetworkSettings']={'Networks':{'bridge':{'NetworkID':'b'*64},
            'amnezia-dns-net':{'NetworkID':'d'*64,'IPAMConfig':{},'Aliases':['native-dns'],'GwPriority':1}}}
        payload=docker.create_payload(value,'snapshot',5512)
        self.assertEqual(payload['NetworkingConfig']['EndpointsConfig'],docker.network_attachments(value))
        altered=copy.deepcopy(value)
        self.assertTrue(docker.same_networks(value,altered))
        altered['NetworkSettings']['Networks']['amnezia-dns-net']['IPAMConfig']={
            'IPv4Address':'','IPv6Address':'','LinkLocalIPs':[]}
        self.assertTrue(docker.same_networks(value,altered))
        altered['NetworkSettings']['Networks'].pop('amnezia-dns-net')
        self.assertFalse(docker.same_networks(value,altered))
        self.assertNotEqual(docker.fingerprint(value),docker.fingerprint(altered))
        altered=copy.deepcopy(value)
        altered['NetworkSettings']['Networks']['amnezia-dns-net']['NetworkID']='e'*64
        self.assertFalse(docker.same_networks(value,altered))
        altered=copy.deepcopy(value)
        altered['NetworkSettings']['Networks']['amnezia-dns-net']['IPAMConfig']={'IPv4Address':'172.29.172.2'}
        with self.assertRaisesRegex(InstallError,'fixed IPAM'):docker.network_attachments(altered)

    def test_default_bridge_recreation_does_not_drift_fingerprint(self):
        value=native()
        value['NetworkSettings']={'Networks':{'bridge':{'NetworkID':'b'*64},
            'amnezia-dns-net':{'NetworkID':'d'*64,'Aliases':['native-dns'],'GwPriority':1}}}
        recreated=copy.deepcopy(value)
        recreated['NetworkSettings']['Networks']['bridge']['NetworkID']='f'*64
        self.assertEqual(docker.fingerprint(value),docker.fingerprint(recreated))
        self.assertTrue(docker.same_networks(value,recreated))
        recreated['NetworkSettings']['Networks']['amnezia-dns-net']['NetworkID']='e'*64
        self.assertNotEqual(docker.fingerprint(value),docker.fingerprint(recreated))
        self.assertFalse(docker.same_networks(value,recreated))

    def test_network_dependencies_require_exact_local_bridge_identity(self):
        value=native();value['NetworkSettings']={'Networks':{'bridge':{'NetworkID':'b'*64},
            'amnezia-dns-net':{'NetworkID':'d'*64}}}
        observed={'Id':'d'*64,'Name':'amnezia-dns-net','Driver':'bridge','Scope':'local','Labels':{}}
        with patch('docker.command',return_value=json.dumps(observed).encode()) as read:docker.verify_networks(value)
        self.assertEqual(read.call_args[0][-1],'d'*64)
        value['NetworkSettings']['Networks']['bridge']['NetworkID']='c'*64
        with patch('docker.command',return_value=json.dumps(observed).encode()) as read:docker.verify_networks(value)
        self.assertEqual(read.call_args[0][-1],'d'*64)
        for change in ({'Id':'c'*64},{'Name':'other'},{'Driver':'overlay'},{'Scope':'swarm'},{'Labels':{'com.docker.compose.project':'owner'}}):
            with patch('docker.command',return_value=json.dumps(dict(observed,**change)).encode()):
                with self.assertRaises(InstallError):docker.verify_networks(value)

    def test_consent_binds_launch_native_binary_and_port(self):
        value = native()
        facts = {'nativeDigest':'1'*64,'binaryDigest':'2'*64,'nativeVersion':'1','names':['native.test']}
        original = install.consent_scope(value, facts, 5500)
        self.assertNotEqual(original, install.consent_scope(value, facts, 5501))
        for field in ('nativeDigest','binaryDigest','nativeVersion'):
            altered = dict(facts, **{field:'changed'})
            self.assertNotEqual(original, install.consent_scope(value, altered, 5500))
        value['HostConfig']['Memory'] += 1
        self.assertNotEqual(original, install.consent_scope(value, facts, 5500))

    def test_owned_receipt_detects_launch_and_artifact_drift(self):
        value = native()
        value['Config']['Labels'] = {'io.hton.minimum':'instance','io.hton.role':'edge'}
        self.store.write('artifact', b'config')
        state = {'instance':'instance'}
        with patch('ingress.inspect', return_value=value):
            ingress.record_owned(self.store,state,value['Id'],'edge',['artifact'])
            ingress.owned(state,value['Id'],'edge',self.store)
            value['HostConfig']['Memory'] += 1
            with self.assertRaises(InstallError): ingress.owned(state,value['Id'],'edge',self.store)
            value['HostConfig']['Memory'] -= 1
            self.store.write('artifact', b'drift')
            with self.assertRaises(InstallError): ingress.owned(state,value['Id'],'edge',self.store)

    def test_owned_receipt_accepts_daemon_restore_empty_list_encoding(self):
        value = native()
        value['Config']['Labels'] = {'io.hton.minimum':'instance','io.hton.role':'edge'}
        value['HostConfig']['Dns'] = None
        state = {'instance':'instance'}
        with patch('ingress.inspect', return_value=value):
            ingress.record_owned(self.store,state,value['Id'],'edge')
        value['HostConfig']['Dns'] = []
        with patch('ingress.inspect', return_value=value):
            ingress.owned(state,value['Id'],'edge',self.store)
        value['HostConfig']['Dns'] = ['10.0.0.53']
        with patch('ingress.inspect', return_value=value):
            with self.assertRaisesRegex(InstallError,'drifted'): ingress.owned(state,value['Id'],'edge',self.store)

    def test_lease_and_actor_safety(self):
        with patch('install.boot_id',return_value='boot'), patch('install.time.time',return_value=100), patch('install.time.monotonic',return_value=50):
            value={'lease':200,'leaseBoot':'boot','leaseMonotonic':150}
            self.assertFalse(install.lease_expired(value))
            with self.assertRaises(InstallError): install.terminate_expired_actor(value)
            self.assertTrue(install.lease_expired(dict(value,leaseBoot='old')))
            self.assertTrue(install.lease_expired(dict(value,leaseMonotonic=49)))
            self.assertFalse(install.lease_expired(dict(value,lease=0)))

    def test_legacy_fingerprint_and_approval_rebaseline_from_protected_record(self):
        original=native()
        original['NetworkSettings']={'Networks':{'bridge':{'NetworkID':'b'*64}}}
        self.store.save('original.json',original)
        facts={'nativeDigest':'1'*64,'binaryDigest':'2'*64,'nativeVersion':'1','names':['native.test']}
        state={'originalFingerprint':docker.fingerprint(original,version=1),'hasCommitted':False,
            'nativeDigest':facts['nativeDigest'],'originalNativeDigest':facts['nativeDigest'],
            'originalNativeNames':facts['names'],'binaryDigest':facts['binaryDigest'],
            'nativeVersion':facts['nativeVersion'],'port':5512,'approval':'stale'}
        install.upgrade_state_fingerprints(None,self.store,state)
        self.assertEqual(state['fingerprintVersion'],docker.FINGERPRINT_VERSION)
        self.assertEqual(state['originalFingerprint'],docker.fingerprint(original))
        self.assertEqual(state['approvalVersion'],install.APPROVAL_VERSION)
        self.assertEqual(state['approval'],install.consent_scope(original,facts,5512))
        self.assertEqual(self.store.json('state.json')['approval'],state['approval'])

    def test_configuration_routes_only_exact_native_sni(self):
        ha, site = ingress.configs('site.test',['native.test'],5512,9443,b'hello')
        self.assertIn(b'server native 127.0.0.1:5512',ha)
        self.assertIn(b'tcp-request content reject if hello !allowed',ha)
        self.assertTrue(json.loads(site)['apps']['http']['servers']['site']['tls_connection_policies'])
        with self.assertRaises(InstallError): ingress.configs('native.test',['native.test'],5512,9443,b'x')

    def test_site_backend_serves_document_root_with_assets(self):
        from pathlib import Path
        _ha, site = ingress.configs('site.test',['native.test'],5512,9443,b'hello',Path('/var/www/html'))
        handler = json.loads(site)['apps']['http']['servers']['site']['routes'][0]['handle'][0]
        self.assertEqual(handler['handler'],'file_server')
        self.assertEqual(handler['root'],'/site')
        self.assertIn('.git',handler['hide'])
        _ha, placeholder = ingress.configs('site.test',['native.test'],5512,9443,b'hello',None)
        fallback = json.loads(placeholder)['apps']['http']['servers']['site']['routes'][0]['handle'][0]
        self.assertEqual(fallback['handler'],'static_response')
        self.assertEqual(fallback['body'],'hello')

    def test_configured_site_root_is_authoritative(self):
        site = self.root / 'site'
        site.mkdir()
        (site / 'index.html').write_text('<html>real</html>', encoding='utf-8')
        self.assertEqual(ingress.site_root(str(site)), site)
        self.assertEqual(ingress.webroot(str(site)), b'<html>real</html>')
        with self.assertRaises(InstallError):
            ingress.site_root(str(self.root / 'missing'))
        (site / 'index.html').unlink()
        with self.assertRaises(InstallError):
            ingress.site_root(str(site))
        with self.assertRaises(InstallError):
            ingress.site_root('relative/path')

    def test_index_htm_is_accepted_as_site_entry(self):
        site = self.root / 'htm'
        site.mkdir()
        (site / 'index.htm').write_text('<html>htm</html>', encoding='utf-8')
        self.assertEqual(ingress.site_root(str(site)), site)
        self.assertEqual(ingress.webroot(str(site)), b'<html>htm</html>')
        _ha, config = ingress.configs('site.test', ['native.test'], 5512, 9443,
            b'<html>htm</html>', site)
        handler = json.loads(config)['apps']['http']['servers']['site']['routes'][0]['handle'][0]
        self.assertEqual(handler['index_names'], ['index.html', 'index.htm'])

    def test_domain_validation(self):
        for domain in ('127.0.0.1','bad\n.test','-bad.test','one','UPPER.test'):
            self.assertFalse(docker.valid_domain(domain))
        self.assertTrue(docker.valid_domain('example.com'))

    def test_rollback_restores_only_exact_original_and_is_idempotent(self):
        original=native(); state={'originalFingerprint':docker.fingerprint(original),'originalParked':'parked',
            'replacementName':'replacement','replacementParked':'retained','nativeDigest':docker.digest(b'config')}
        self.store.save('original.json',original)
        current=copy.deepcopy(original);current['Name']='/parked';current['State']['Running']=False
        current['HostConfig']['RestartPolicy']={'Name':'no','MaximumRetryCount':0}
        mutations=[]
        def command(*args,**kwargs):
            mutations.append(args)
            self.assertEqual(args[-1] if args[1]!='rename' else args[2], original['Id'])
            if args[1]=='rename':current['Name']='/'+args[3]
            if args[1]=='update':current['HostConfig']['RestartPolicy']={'Name':args[3],'MaximumRetryCount':0}
            if args[1]=='start':current['State']['Running']=True
            return b''
        op=install.Installation(self.store,self.store,state)
        with patch('install.process_identity',return_value={}),patch('install.os.getpgrp',return_value=os.getpid()),patch('ingress.cleanup_temporary_jobs'),patch('docker.inspect',return_value=current),patch('docker.native_config',return_value=b'config'),patch('docker.command',side_effect=command):
            op.rollback();op.rollback()
        self.assertEqual(docker.fingerprint(current),docker.fingerprint(original))
        self.assertEqual(state['phase'],'restored')
        self.assertFalse(state['lease'])
        self.assertTrue(mutations)

    def test_check_foreign_treats_unspecified_oom_disable_equally_across_start(self):
        original=native();original['HostConfig']['OomKillDisable']=None
        state={'originalFingerprint':docker.fingerprint(original),'originalParked':'parked',
            'replacementName':'replacement','replacementParked':'retained','nativeDigest':docker.digest(b'config'),
            'snapshotImage':'sha256:'+'d'*64,'port':5512,'replacement':'c'*64}
        self.store.save('original.json',original)
        base=copy.deepcopy(original);base['Id']='c'*64;base['Name']='/replacement';base['Image']=state['snapshotImage']
        base['Config']=copy.deepcopy(original['Config']);base['Config']['Image']=state['snapshotImage']
        payload=docker.create_payload(original,state['snapshotImage'],5512)
        base['HostConfig']=payload.pop('HostConfig')
        base['State']={'Running':True}
        op=install.Installation(self.store,self.store,state)
        for oom in (None,False):
            current=copy.deepcopy(base);current['HostConfig']['OomKillDisable']=oom
            with patch('docker.inspect',return_value=current),patch('docker.native_config',return_value=b'config'):
                self.assertEqual(op.check_foreign(current['Id'],replacement=True),current)
        current=copy.deepcopy(base);current['HostConfig']['OomKillDisable']=True
        with patch('docker.inspect',return_value=current),patch('docker.native_config',return_value=b'config'):
            with self.assertRaisesRegex(InstallError,'Foreign launch/config drift'):
                op.check_foreign(current['Id'],replacement=True)

    def test_check_foreign_treats_restored_dns_encoding_equally(self):
        original=native();original['HostConfig']['Dns']=None
        state={'originalFingerprint':docker.fingerprint(original),'originalParked':'parked',
            'replacementName':'replacement','replacementParked':'retained','nativeDigest':docker.digest(b'config'),
            'snapshotImage':'sha256:'+'d'*64,'port':5512,'replacement':'c'*64}
        self.store.save('original.json',original)
        base=copy.deepcopy(original);base['Id']='c'*64;base['Name']='/replacement';base['Image']=state['snapshotImage']
        base['Config']=copy.deepcopy(original['Config']);base['Config']['Image']=state['snapshotImage']
        payload=docker.create_payload(original,state['snapshotImage'],5512)
        base['HostConfig']=payload.pop('HostConfig');base['State']={'Running':True}
        op=install.Installation(self.store,self.store,state)
        for dns in (None,[]):
            current=copy.deepcopy(base);current['HostConfig']['Dns']=dns
            with patch('docker.inspect',return_value=current),patch('docker.native_config',return_value=b'config'):
                self.assertEqual(op.check_foreign(current['Id'],replacement=True),current)
        current=copy.deepcopy(base);current['HostConfig']['Dns']=['10.0.0.53']
        with patch('docker.inspect',return_value=current),patch('docker.native_config',return_value=b'config'):
            with self.assertRaisesRegex(InstallError,'Foreign launch/config drift'):
                op.check_foreign(current['Id'],replacement=True)

    def test_cutover_probe_failure_restores_native_before_returning(self):
        original=native()
        replacement=copy.deepcopy(original)
        replacement['Id']='c'*64;replacement['Image']='sha256:'+'d'*64
        payload=docker.create_payload(original,replacement['Image'],5512)
        replacement['HostConfig']=payload.pop('HostConfig');replacement['Config']=payload
        replacement['Name']='/replacement';replacement['State']['Running']=False
        state={'originalId':original['Id'],'originalFingerprint':docker.fingerprint(original),
            'originalParked':'parked','replacementName':'replacement','replacementParked':'retained',
            'snapshotImage':replacement['Image'],'port':5512,'replacement':replacement['Id'],
            'nativeDigest':docker.digest(b'config'),'binaryDigest':'f'*64,'candidateIngress':{'site':'e'*64,'edge':'f'*64}}
        self.store.save('original.json',original)
        values={original['Id']:copy.deepcopy(original),replacement['Id']:replacement}
        values.update({i:{'State':{'Running':False}} for i in state['candidateIngress'].values()})
        def command(*args,**kwargs):
            if args[1]=='rename':identifier=args[2]
            else:identifier=args[-1]
            self.assertIn(identifier,values)
            value=values[identifier]
            if args[1]=='rename':value['Name']='/'+args[3]
            elif args[1]=='update':
                policy='no' if '--restart=no' in args else args[3]
                value.setdefault('HostConfig',{})['RestartPolicy']={'Name':policy,'MaximumRetryCount':0}
            elif args[1]=='stop':value['State']['Running']=False
            elif args[1]=='start':value['State']['Running']=True
            else:raise AssertionError(args)
            return b''
        op=install.Installation(self.store,self.store,state)
        with patch('install.process_identity',return_value={}),patch('install.os.getpgrp',return_value=os.getpid()),patch('install.log'),patch('install.timer'),patch('docker.inspect',side_effect=lambda i:values[i]),patch('docker.native_config',return_value=b'config'),patch('docker.command',side_effect=command),patch('docker.qualify',return_value={'nativeDigest':state['nativeDigest'],'binaryDigest':state['binaryDigest']}),patch('ingress.owned',side_effect=lambda state,i,role:values[i]),patch('ingress.website',side_effect=InstallError('probe failed')):
            with self.assertRaisesRegex(InstallError,'probe failed'):
                op.cutover(b'body',['native.test'],'site.test',{},{})
        self.assertEqual(docker.fingerprint(values[original['Id']]),docker.fingerprint(original))
        self.assertTrue(values[original['Id']]['State']['Running'])
        self.assertFalse(values[replacement['Id']]['State']['Running'])
        self.assertEqual(state['phase'],'restored')
        for identifier in ('e'*64,'f'*64):self.assertFalse(values[identifier]['State']['Running'])

    def test_unexpected_failure_diagnostics_redact_messages_and_locals(self):
        with patch('install.os.geteuid',side_effect=RuntimeError('fixture-sensitive-provider-key')):
            try:
                install.main([])
            except RuntimeError as error:
                detail=install.failure_detail(error)
                message=install.failure_message(error)
            else:self.fail('Expected diagnostic fixture failure')
        self.assertEqual(detail['kind'],'RuntimeError')
        self.assertEqual(detail['module'],'install')
        self.assertEqual(detail['function'],'main')
        self.assertNotIn('fixture-sensitive',message+json.dumps(detail))
        self.assertIn('install.main:',message)
        self.assertEqual(set(detail),{'kind','module','function','line'})

    def test_real_tty_prompts_consent_and_credentials_without_secret_echo(self):
        value=native()
        facts={'nativeDigest':'1'*64,'binaryDigest':'2'*64,'nativeVersion':'fixture','names':['native.test']}
        module_root=Path(install.__file__).parent
        script="""
import sys,json
sys.dont_write_bytecode=True
sys.path.insert(0,sys.argv[1])
import install,certificates
from storage import Store,digest
assert install.prompt('Site domain','default.test')=='site.test'
assert install.prompt('Use default','kept')=='kept'
value=json.loads(sys.argv[3]);facts=json.loads(sys.argv[4])
install.consent(value,facts,5512)
store=Store(__import__('pathlib').Path(sys.argv[2]))
secret=certificates.credentials(store,None)
assert secret==store.json('secrets/minimum-internetbs.json')
assert digest((secret['apiKey']+'|'+secret['apiPassword']).encode())==sys.argv[5]
print('TTY-ACCEPTED',flush=True)
"""
        from storage import digest
        key='fixture-provider-key-value';password='fixture-provider-password-value'
        pid,master=pty.fork()
        if pid==0:
            os.execl(sys.executable,sys.executable,'-B','-c',script,str(module_root),str(self.root),json.dumps(value),json.dumps(facts),digest((key+'|'+password).encode()))
        transcript=bytearray();reaped=False
        deadline=time.monotonic()+10
        def receive_until(marker):
            while marker not in transcript:
                if time.monotonic()>deadline:raise AssertionError('TTY fixture timed out')
                if select.select([master],[],[],.1)[0]:
                    try:block=os.read(master,16384)
                    except OSError:raise AssertionError('TTY fixture closed before expected prompt') from None
                    if not block:raise AssertionError('TTY fixture ended before expected prompt')
                    transcript.extend(block)
        try:
            for marker,answer in ((b'Site domain [default.test]: ',b'site.test'),(b'Use default [kept]: ',b''),
                    (b'Move Amnezia and continue? [y/N]: ',b'y'),
                    (b'InternetBS API key (stored privately): ',key.encode()),(b'InternetBS API password (stored privately): ',password.encode())):
                receive_until(marker);os.write(master,answer+b'\n')
            receive_until(b'TTY-ACCEPTED')
            _,status=os.waitpid(pid,0);reaped=True
            self.assertEqual(os.waitstatus_to_exitcode(status),0)
            self.assertNotIn(key.encode(),transcript)
            self.assertNotIn(password.encode(),transcript)
        finally:
            if not reaped:
                try:os.kill(pid,signal.SIGKILL)
                except ProcessLookupError:pass
                os.waitpid(pid,0)
            os.close(master)

    def write_bootstrap(self, value):
        directory=self.root/'platform/etc';directory.mkdir(parents=True,exist_ok=True)
        (directory/'install.json').write_text(json.dumps(value))

    def test_site_inputs_prompt_only_missing_and_persist_for_next_install(self):
        args=argparse.Namespace(site_domain=None,certificate_email=None)
        root={'features':{'other':{'enabled':False}}}
        with patch('install.SOURCE',self.root),patch('install.sys.stdin.isatty',return_value=True),patch('install.prompt',side_effect=['site.test','owner@example.test']) as prompts:
            domain,email=install.load_site_inputs(root,args)
            self.assertEqual(prompts.call_count,2)
            install.save_site_inputs(self.store,root,domain,email)
        saved=self.store.json('hton.yaml')
        self.assertEqual(saved['core']['edge']['domains'],['site.test'])
        self.assertEqual(saved['core']['certificates']['email'],'owner@example.test')
        self.assertEqual(saved['installation']['directory'],str(install.ROOT))
        self.assertEqual(saved['features'],{'other':{'enabled':False}})
        with patch('install.SOURCE',self.root),patch('install.prompt') as prompts:
            self.assertEqual(install.load_site_inputs(saved,args),('site.test','owner@example.test'))
            prompts.assert_not_called()

    def test_bootstrap_fills_missing_but_never_replaces_saved_values(self):
        self.write_bootstrap({'schemaVersion':1,'core':{'edge':{'domains':['bootstrap.test']},'certificates':{'email':'bootstrap@example.test'}}})
        args=argparse.Namespace(site_domain=None,certificate_email=None)
        with patch('install.SOURCE',self.root),patch('install.prompt') as prompts:
            self.assertEqual(install.load_site_inputs({},args),('bootstrap.test','bootstrap@example.test'))
            saved={'core':{'edge':{'domains':['saved.test']},'certificates':{'email':'saved@example.test'}}}
            self.assertEqual(install.load_site_inputs(saved,args),('saved.test','saved@example.test'))
            prompts.assert_not_called()
            args.site_domain='different.test'
            with self.assertRaisesRegex(InstallError,'Saved site settings differ'):install.load_site_inputs(saved,args)
        self.write_bootstrap({'core':{'edge':{'domains':['bootstrap.test']}}})
        args.site_domain=None
        with patch('install.SOURCE',self.root),patch('install.sys.stdin.isatty',return_value=True),patch('install.prompt',return_value='missing@example.test') as prompts:
            self.assertEqual(install.load_site_inputs({},args),('bootstrap.test','missing@example.test'))
            prompts.assert_called_once_with('Certificate account email')

    def test_quiet_bootstrap_never_prompts_for_missing_site_inputs(self):
        args = argparse.Namespace(site_domain=None, certificate_email=None, quiet=True)
        with patch('install.SOURCE', self.root), patch('install.sys.stdin.isatty', return_value=True), \
                patch('install.prompt') as prompts, self.assertRaises(InstallError):
            install.load_site_inputs({}, args)
        prompts.assert_not_called()

    def test_dns_credentials_placeholders_are_rejected(self):
        self.store.save('secrets/minimum-internetbs.json', {'apiKey':'CHANGE-ME-key','apiPassword':'CHANGE-ME-pass'})
        with self.assertRaisesRegex(InstallError,'placeholder'):
            certificates.credentials(self.store,None,interactive=False)

    def test_configured_deployment_directory_and_protected_source_binding(self):
        self.write_bootstrap({'installation':{'directory':'/opt/hton-fixture-custom'}})
        self.assertEqual(install_config.deployment_root(self.root),Path('/opt/hton-fixture-custom'))
        with patch('install_config.load_settings',side_effect=AssertionError('Installed root must not be overridden')):
            with self.assertRaises(AssertionError):install_config.deployment_root(Path('/opt/hton-fixture-custom'))
        for directory in ('/','/tmp/hton','/opt','/opt/a/b','/opt/../root','/opt/unsafe name'):
            with self.assertRaises(install_config.ConfigError):install_config.target_directory({'installation':{'directory':directory}})
        root={'installation':{'directory':'/opt/different'}}
        with self.assertRaisesRegex(InstallError,'implicit relocation'):install.save_site_inputs(self.store,root,'site.test','owner@example.test')
        self.assertIsNone(self.store.read('hton.yaml'))

    def test_bootstrap_duplicate_keys_are_rejected_without_value_disclosure(self):
        directory=self.root/'platform/etc';directory.mkdir(parents=True)
        (directory/'install.json').write_text('{"core":{},"core":{"secret":"fixture-sensitive"}}')
        with self.assertRaises(install_config.ConfigError) as context:install_config.load_settings(self.root)
        self.assertNotIn('fixture-sensitive',str(context.exception))

    def test_unit_working_directory_uses_configured_root(self):
        with patch('install.ROOT',Path('/opt/hton-fixture-custom')),patch('install.unit_preflight'),patch('docker.run'):
            install.install_units(self.store)
        content=self.store.read('units/hton-minimum-renew.service').decode()
        self.assertIn('WorkingDirectory=/opt/hton-fixture-custom',content)

    def test_actual_deploy_and_install_preview_follow_configured_target(self):
        repository=Path(install.__file__).resolve().parents[2]
        source=self.root/'source';source.mkdir()
        required=['bin/hton','scripts/install','scripts/setup','scripts/deploy','scripts/secrets','scripts/update','scripts/clients','scripts/usage','lib/hton','lib/bootstrap','lib/install_config.py','lib/secrets_store.py']
        required += ['python/minimum/'+name+'.py' for name in ('storage','docker','certificates','ingress','protocols','publication','http_challenge','migrations','configuration','managed_amnezia','host_access','access_links','client_page','install','clients','updates')]
        required.append('services/onomat/runtime/dns/python/dns.py')
        for name in required:
            destination=source/name;destination.parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(repository/name,destination)
            if name.startswith(('bin/','scripts/')):destination.chmod(0o755)
        (source/'platform/etc').mkdir(parents=True)
        (source/'platform/etc/install.json').write_text(json.dumps({'schemaVersion':1,'installation':{'directory':'/opt/hton-fixture-config-preview'}}))
        result=subprocess.run([sys.executable,'-B',str(source/'scripts/deploy'),'--dry-run','--json'],capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(json.loads(result.stdout)['target'],'/opt/hton-fixture-config-preview')
        result=subprocess.run([str(source/'scripts/install'),'--dry-run','--json'],cwd=source,capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(json.loads(result.stdout)['target'],'/opt/hton-fixture-config-preview')

    def test_setup_refuses_wrong_working_directory_before_any_effect(self):
        repository=Path(install.__file__).resolve().parents[2]
        result=subprocess.run([str(repository/'scripts/setup'),'--dry-run','--json'],cwd=self.root,capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,2)
        self.assertIn('setup must run from the configured target directory',result.stderr)
        self.assertEqual(list(self.root.iterdir()),[])

    def test_install_entrypoint_runs_isolated_without_sys_path_leak(self):
        repository=Path(install.__file__).resolve().parents[2]
        result=subprocess.run([sys.executable,'-E','-s','-B',str(repository/'python/minimum/install.py'),'--dry-run'],capture_output=True,text=True,timeout=20)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertIn('"status": "preview"',result.stdout)

    def test_install_orchestration_deploys_changes_directory_and_calls_setup(self):
        repository=Path(install.__file__).resolve().parents[2]
        source=self.root/'source';target=self.root/'target';log=self.root/'calls'
        for directory in (source/'scripts',target/'scripts'):directory.mkdir(parents=True)
        (source/'scripts/deploy').write_text('#!/bin/bash\nprintf "deploy\\n" >> "$UNIT_LOG"\n')
        (target/'scripts/setup').write_text('#!/bin/bash\nprintf "setup:%s\\n" "$PWD" >> "$UNIT_LOG"\nprintf "%s\\n" "$@" >> "$UNIT_LOG"\n')
        for executable in (source/'scripts/deploy',target/'scripts/setup'):executable.chmod(0o755)
        script='source "$1"; hton_parse_common --json; hton_install_target "$2" "$3" --dns-credentials "private file.json"'
        env=dict(os.environ,UNIT_LOG=str(log))
        result=subprocess.run(['/bin/bash','-c',script,'test',str(repository/'lib/hton'),str(source),str(target)],cwd=self.root,env=env,capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(log.read_text().splitlines(),['deploy','setup:'+str(target),'--json','--dns-credentials','private file.json'])
        log.unlink()
        result=subprocess.run(['/bin/bash','-c',script,'test',str(repository/'lib/hton'),str(target),str(target)],cwd=target,env=env,capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(log.read_text().splitlines()[0],'setup:'+str(target))
        self.assertNotIn('deploy',log.read_text().splitlines())
        log.unlink();(source/'scripts/deploy').write_text('#!/bin/bash\nexit 7\n')
        result=subprocess.run(['/bin/bash','-c',script,'test',str(repository/'lib/hton'),str(source),str(target)],cwd=self.root,env=env,capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,7)
        self.assertFalse(log.exists())

    def test_minimum_setup_bootstraps_before_core_and_stops_on_failure(self):
        repository = Path(install.__file__).resolve().parents[2]
        target = self.root / 'target'
        (target / 'python/minimum').mkdir(parents=True)
        log = self.root / 'calls'
        (target / 'python/minimum/install.py').write_text(
            'import os,sys\n'
            'with open(os.environ["UNIT_LOG"], "a") as out:\n'
            '    out.write("core:" + "|".join(sys.argv[1:]) + "\\n")\n')
        script = ('source "$1"; hton_parse_common; '
            'hton_install_bootstrap() { printf "bootstrap\\n" >> "$UNIT_LOG"; return "$UNIT_BOOTSTRAP_STATUS"; }; '
            'hton_minimum_setup "$2" --site-domain example.com')
        env = dict(os.environ, UNIT_LOG=str(log), UNIT_BOOTSTRAP_STATUS='0')
        command = ['/bin/bash', '-c', script, 'test', str(repository / 'lib/hton'), str(target)]
        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(log.read_text().splitlines(), ['bootstrap', 'core:--site-domain|example.com'])
        log.unlink()
        env['UNIT_BOOTSTRAP_STATUS'] = '4'
        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 4, result.stderr)
        self.assertEqual(log.read_text().splitlines(), ['bootstrap'])
        log.unlink()
        command[2] = script.replace('hton_parse_common;', 'hton_parse_common --dry-run;')
        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(log.read_text().splitlines(), ['core:--site-domain|example.com'])

    def test_bootstrap_detects_missing_certificate_tool(self):
        repository = Path(install.__file__).resolve().parents[2]
        script = ('source "$1"; '
            'command() { if [[ "$*" == "-v openssl" ]]; then return 1; fi; builtin command "$@"; }; '
            'hton_check_dependencies || true; printf "%s\\n" "${hton_missing_dependencies[@]}"')
        result = subprocess.run(['/bin/bash', '-c', script, 'test', str(repository / 'lib/bootstrap')],
            capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('openssl', result.stdout.splitlines())

    def test_install_without_python_elevates_before_target_resolution(self):
        if os.geteuid() == 0:
            self.skipTest('exercise the unprivileged reentry path')
        repository = Path(install.__file__).resolve().parents[2]
        script = ('source "$1"; hton_python_available() { return 1; }; '
            '/usr/bin/python3() { printf "unexpected interpreter call\\n" >&2; return 9; }; '
            'sudo() { printf "<%s>\\n" "$@"; }; '
            'hton_source_update() { printf "unexpected source update\\n"; return 9; }; '
            'hton_cli install --yes --json --quiet --site-domain example.com')
        result = subprocess.run(['/bin/bash', '-c', script, 'test', str(repository / 'lib/hton')],
            cwd=repository, capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), ['<-n>', '<-->', f'<{repository}/bin/hton>',
            '<--yes>', '<--json>', '<--quiet>', '<install>', '<--site-domain>', '<example.com>'])

    def test_missing_python_preview_and_invalid_options_do_not_bootstrap(self):
        repository = Path(install.__file__).resolve().parents[2]
        script = ('source "$1"; hton_python_available() { return 1; }; '
            '/usr/bin/python3() { printf "unexpected interpreter call\\n" >&2; return 9; }; '
            'hton_install_elevate() { printf "unexpected elevation\\n"; return 9; }; '
            'hton_cli install "$2"')
        for argument, expected in [('--dry-run', 3), ('--invalid', 2)]:
            with self.subTest(argument=argument):
                result = subprocess.run(['/bin/bash', '-c', script, 'test',
                    str(repository / 'lib/hton'), argument], cwd=repository,
                    capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, expected, result.stderr)
                self.assertEqual(result.stdout, '')

    def test_install_sudo_reentry_passes_env_before_the_command(self):
        # GNU env stops option parsing at the first assignment: `env VAR=1 -- cmd`
        # runs `--` as the program. The escalated install must not emit that form.
        if os.geteuid() == 0:
            self.skipTest('the privilege branch is skipped when already root')
        repository=Path(install.__file__).resolve().parents[2]
        script=('source "$1"; sudo() { printf "<%s>\\n" "$@"; }; '
            'export HTON_SOURCE_UPDATED=1; hton_cli install --yes')
        result=subprocess.run(['/bin/bash','-c',script,'test',str(repository/'lib/hton')],
            cwd=repository,capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(result.stdout.splitlines(),
            ['<-n>','<-->','<env>','<HTON_SOURCE_UPDATED=1>',f'<{repository}/bin/hton>','<--yes>','<install>'])

    def test_install_target_forwards_rotation_and_source_root(self):
        repository=Path(install.__file__).resolve().parents[2]
        source=self.root/'source';target=self.root/'target';log=self.root/'calls'
        for directory in (source/'scripts',target/'scripts'):directory.mkdir(parents=True)
        (source/'scripts/deploy').write_text('#!/bin/bash\nprintf "deploy\\n" >> "$UNIT_LOG"\n')
        (target/'scripts/setup').write_text('#!/bin/bash\nprintf "setup:%s\\n" "$PWD" >> "$UNIT_LOG"\nprintf "source:%s\\n" "${HTON_INSTALL_SOURCE:-}" >> "$UNIT_LOG"\nprintf "%s\\n" "$@" >> "$UNIT_LOG"\n')
        for executable in (source/'scripts/deploy',target/'scripts/setup'):executable.chmod(0o755)
        script='source "$1"; hton_parse_common --json; hton_install_target "$2" "$3" --rotate-client-page'
        env=dict(os.environ,UNIT_LOG=str(log))
        result=subprocess.run(['/bin/bash','-c',script,'test',str(repository/'lib/hton'),str(source),str(target)],cwd=self.root,env=env,capture_output=True,text=True,timeout=10)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(log.read_text().splitlines(),['deploy','setup:'+str(target),'source:'+str(source),'--json','--rotate-client-page'])

    def test_remap_consent_is_readable_explicit_and_defaults_to_decline(self):
        value=native();facts={'nativeDigest':'1'*64,'binaryDigest':'2'*64,'nativeVersion':'fixture','names':['native.test']}
        for answer in ('y','yes','Y','YES'):
            with patch('install.prompt',return_value=answer) as question:
                self.assertEqual(install.consent(value,facts,5512),install.consent_scope(value,facts,5512))
                text=question.call_args.args[0]
                self.assertIn('Move Amnezia and continue? [y/N]',text)
                self.assertIn('127.0.0.1:5512',text)
                self.assertIn(value['Id'],text)
                self.assertNotIn('Type REMAP',text)
        for answer in ('','n','no','anything else'):
            with patch('install.prompt',return_value=answer):
                with self.assertRaisesRegex(InstallError,'cancelled'):install.consent(value,facts,5512)

    def test_http_policy_migrates_only_unissued_uncredentialed_legacy_attempt(self):
        root={'core':{'certificates':{'ca':'letsencrypt','challenge':'dns-01','provider':'internetbs'}}}
        self.assertEqual(certificates.select_policy({},None,self.store),'http-01')
        self.assertEqual(certificates.select_policy(root,{'phase':'prepared'},self.store),'http-01')
        self.assertEqual(certificates.select_policy(root,{'hasCommitted':True},self.store),'dns-01')
        self.assertEqual(certificates.select_policy(root,{'certificate':'retained'},self.store),'dns-01')
        self.assertEqual(certificates.select_policy(root,{'issuerJob':'known'},self.store),'dns-01')
        self.assertEqual(certificates.select_policy(root,{},self.store,'/private/input'),'dns-01')
        self.assertEqual(certificates.select_policy(root,{},self.store,defaults={'challenge':'dns-01'}),'dns-01')
        self.store.save('secrets/minimum-internetbs.json',{'apiKey':'fixture','apiPassword':'fixture'})
        self.assertEqual(certificates.select_policy(root,{},self.store),'dns-01')
        with self.assertRaises(InstallError):certificates.select_policy({'core':{'certificates':{'challenge':'http-01'}}},None,self.store,'/input')

    def test_http_issuer_argv_has_no_dns_secrets_and_only_binds_http(self):
        args=certificates.issuer_arguments(self.store,{'instance':'fixture'},'pinned-image','token',self.root/'output','site.test','email@example.test','http-01')
        self.assertNotIn('--env-file',args)
        self.assertNotIn('--dns',args)
        self.assertIn('--standalone',args)
        self.assertEqual(args[args.index('--network')+1],'host')
        self.assertEqual(args[args.index('--httpport')+1],'80')
        self.assertNotIn('443',args)
        self.assertNotIn('--force',args)
        self.assertEqual(args[args.index('--server')+1],'letsencrypt')
        dns=certificates.issuer_arguments(self.store,{'instance':'fixture'},'pinned-image','token',self.root/'output','site.test','email@example.test','dns-01')
        self.assertIn('--env-file',dns);self.assertNotIn('--standalone',dns)
        wildcard=certificates.issuer_arguments(self.store,{'instance':'fixture'},'pinned-image','token',self.root/'output','site.test','email@example.test','dns-01',names=('www.site.test',),wildcard=True)
        self.assertEqual(wildcard[wildcard.index('*.site.test')-1],'-d')
        self.assertEqual(wildcard[wildcard.index('www.site.test')-1],'-d')
        self.assertNotIn('*.site.test',args)

    def test_http_ensure_never_requests_dns_credentials(self):
        captured=[]
        def command(*args,**kwargs):
            captured.append(args)
            return ('a'*64).encode() if args[:2]==('container','create') else b''
        state={'instance':'fixture'}
        with patch('certificates.credentials',side_effect=AssertionError('HTTP must not request DNS credentials')),patch('certificates.cached_output',return_value=False),patch('certificates.http_challenge.select',return_value={'mode':'standalone'}),patch('certificates.require_http_port') as port_check,patch('certificates.command',side_effect=command),patch('docker.inspect',return_value={'State':{'Running':False,'ExitCode':0}}),patch('ingress.record_owned',side_effect=lambda work,state,i,role:state.setdefault('ownedReceipts',{}).update({i:{'role':role}})),patch('ingress.owned'),patch('certificates.accept_output'),patch('certificates.validate'):
            result=certificates.ensure(self.store,self.store,state,{'acme':'pinned-image'},'site.test','email@example.test',None,lambda:None,interactive=False,challenge='http-01')
        port_check.assert_called_once()
        self.assertEqual(result.parent,self.root/'certificates')
        self.assertNotIn('issuerJob',state)
        self.assertFalse(any('--env-file' in call for call in captured))
        self.assertIsNone(self.store.read('issuer/dns.env'))

    def test_renewal_window_adapts_to_actual_certificate_lifetime(self):
        for days,expected in ((90,30),(60,20),(6,2)):
            import datetime
            start=datetime.datetime(2026,1,1,tzinfo=datetime.timezone.utc)
            end=start+datetime.timedelta(days=days)
            data=('notBefore='+start.strftime('%b %d %H:%M:%S %Y GMT')+'\nnotAfter='+end.strftime('%b %d %H:%M:%S %Y GMT')+'\n').encode()
            with patch('certificates.run',return_value=data):self.assertEqual(certificates.renewal_window(Path('fixture.pem')),expected*86400)

    def test_publication_dns_match_and_manual_retry_are_provider_neutral(self):
        edge={'publication':{'addresses':['1.1.1.1'],'resolver':'google'}}
        with patch('publication.records',side_effect=[['1.1.1.1'],[]]),patch('publication.external_ipv4',side_effect=AssertionError('Explicit entry must win')):
            result=publication.check('site.test',edge,interactive=False,prompt=lambda _:self.fail('Unexpected prompt'))
            self.assertEqual(result['status'],'matched')
        with patch('publication.records',side_effect=[['8.8.8.8'],[],['1.1.1.1'],[]]),patch('install.prompt',return_value='') as question:
            publication.check('site.test',edge,interactive=True,prompt=install.prompt)
            text=question.call_args.args[0]
            self.assertIn('Expected A: 1.1.1.1',text)
            self.assertIn('DNS provider panel',text)
            self.assertNotIn('InternetBS',text)
        with patch('publication.records',side_effect=[['8.8.8.8'],[]]):
            with self.assertRaisesRegex(InstallError,'Correct DNS'):publication.check('site.test',edge,interactive=False,prompt=lambda _:None)
        with patch('publication.records',side_effect=[['1.1.1.1'],['2606:4700:4700::1111']]):
            with self.assertRaisesRegex(InstallError,'AAAA'):publication.check('site.test',edge,interactive=False,prompt=lambda _:None)

    def test_dns_parser_follows_only_answered_aliases(self):
        data={'Status':0,'Answer':[{'type':5,'name':'site.test.','data':'target.test.'},
            {'type':1,'name':'target.test.','data':'1.1.1.1'},{'type':1,'name':'unrelated.test.','data':'8.8.8.8'}]}
        with patch('publication.get',return_value=json.dumps(data).encode()):
            self.assertEqual(publication.records('site.test','A','cloudflare'),['1.1.1.1'])
        with patch('publication.get',return_value=b'{"Status":2}'):
            with self.assertRaises(InstallError):publication.records('site.test','A','google')
        with patch('publication.get',return_value=b'ip=1.1.1.1\nloc=fixture\n'):
            self.assertEqual(publication.external_ipv4(),'1.1.1.1')

    def test_multiple_domains_are_not_silently_discarded_by_minimum_profile(self):
        self.write_bootstrap({'core':{'edge':{'domains':['one.test','two.test']}}})
        with patch('install.SOURCE',self.root):
            with self.assertRaisesRegex(InstallError,'not ignored'):
                install.load_site_inputs({},argparse.Namespace(site_domain=None,certificate_email='email@example.test'))

    def test_existing_http_webroot_route_proof_preserves_server_and_files(self):
        root=self.root/'public';root.mkdir(mode=0o755)
        (root/'index.html').write_text('existing website')
        class Static(http.server.SimpleHTTPRequestHandler):
            def log_message(self,*args):pass
            def do_GET(self):
                filename=Path(self.translate_path(self.path))
                if filename.exists() and not filename.stat().st_mode & 0o004:
                    self.send_error(403);return
                super().do_GET()
        server=http.server.ThreadingHTTPServer(('127.0.0.1',0),functools.partial(Static,directory=str(root)))
        threading.Thread(target=server.serve_forever,daemon=True).start()
        try:
            previous_umask=os.umask(0o077)
            try:self.assertEqual(http_challenge.prove(root,'site.test',port=server.server_port),root)
            finally:os.umask(previous_umask)
            self.assertFalse((root/'.well-known').exists())
            self.assertEqual((root/'index.html').read_text(),'existing website')
            with http_challenge.open_directory(root) as (directory,_):(directory/'other-token').write_text('preserve')
            http_challenge.prove(root,'site.test',port=server.server_port)
            self.assertEqual((root/'.well-known/acme-challenge/other-token').read_text(),'preserve')
            self.assertEqual(list((root/'.well-known/acme-challenge').iterdir()),[root/'.well-known/acme-challenge/other-token'])
        finally:server.shutdown();server.server_close()

    def test_webroot_rejects_symlink_and_world_writable_directory(self):
        root=self.root/'public';root.mkdir(mode=0o755)
        other=self.root/'other';other.mkdir()
        (root/'.well-known').symlink_to(other,target_is_directory=True)
        with self.assertRaises((OSError,InstallError)):
            with http_challenge.open_directory(root):pass
        self.assertEqual(list(other.iterdir()),[])
        (root/'.well-known').unlink();root.chmod(0o777)
        with self.assertRaises(InstallError):
            with http_challenge.open_directory(root):pass

    def test_webroot_issuer_mount_is_limited_to_public_challenge_directory(self):
        args=certificates.issuer_arguments(self.store,{'instance':'fixture'},'image','token',self.root/'output','site.test','email@example.test','http-01',http={'mode':'webroot','webroot':'/var/www/html'})
        self.assertNotIn('--standalone',args)
        self.assertNotIn('--httpport',args)
        self.assertNotIn('--env-file',args)
        self.assertIn('--webroot',args)
        self.assertIn('type=bind,src=/var/www/html/.well-known/acme-challenge,dst=/challenge/.well-known/acme-challenge',args)
        self.assertFalse(any('src=/var/www/html,' in arg for arg in args))

    def test_owned_oom_default_lifecycle_and_legacy_receipt_recovery(self):
        value=native();value['HostConfig']['OomKillDisable']=None
        value['Config']['Labels']={'io.hton.minimum':'fixture','io.hton.role':'acme'}
        state={'instance':'fixture'}
        with patch('ingress.inspect',return_value=value):
            ingress.record_owned(self.store,state,value['Id'],'acme')
            self.assertEqual(state['ownedReceipts'][value['Id']]['launchVersion'],2)
            value['HostConfig']['OomKillDisable']=False
            ingress.owned(state,value['Id'],'acme',self.store)
            state['ownedReceipts'][value['Id']].pop('launchVersion')
            state['ownedReceipts'][value['Id']]['launch']=ingress.launch_digest(value,legacy_nullable=True)
            ingress.owned(state,value['Id'],'acme',self.store)
            value['HostConfig']['OomKillDisable']=True
            with self.assertRaisesRegex(InstallError,'drifted'):ingress.owned(state,value['Id'],'acme',self.store)
            value['HostConfig']['OomKillDisable']=False
            value['HostConfig']['Memory']+=1
            with self.assertRaisesRegex(InstallError,'drifted'):ingress.owned(state,value['Id'],'acme',self.store)
            value['HostConfig']['Memory']-=1
            state['ownedReceipts'][value['Id']]['launchVersion']=99
            with self.assertRaisesRegex(InstallError,'version'):ingress.owned(state,value['Id'],'acme',self.store)

    def test_command_failure_is_plain_and_never_echoes_native_secrets(self):
        script="import sys;sys.stderr.write('fixture-private-password: permission denied');sys.stdout.write('fixture-private-token');sys.exit(23)"
        with self.assertRaises(docker.NativeError) as context:
            docker.run([sys.executable,'-c',script])
        error=context.exception
        self.assertEqual(error.code,23)
        self.assertIn('Access denied',str(error))
        self.assertNotIn('fixture-private',str(error)+repr(error))
        self.assertNotIn('withheld',str(error))
        detail=install.failure_detail(error)
        self.assertEqual(detail['exitCode'],23)
        self.assertNotIn('fixture-private',json.dumps(detail))
        stream=io.StringIO()
        with patch('install.sys.stderr',stream):install.report_failure(error)
        self.assertIn('ERROR: Setup stopped.',stream.getvalue())
        self.assertIn('exit 23',stream.getvalue())
        self.assertNotIn('fixture-private',stream.getvalue())

    def test_command_stderr_is_drained_and_bounded(self):
        result=docker.run([sys.executable,'-c',"import sys;sys.stderr.write('private');sys.stdout.write('ok')"])
        self.assertEqual(result,b'ok')
        with self.assertRaisesRegex(docker.NativeError,'too much error data'):
            docker.run([sys.executable,'-c',"import sys;sys.stderr.write('x'*100000)"])
        self.assertNotIn('fixture-secret',docker.operation_name(docker.DOCKER+['container','create','--env','PASSWORD=fixture-secret']))

    def test_human_setup_result_has_clear_success_and_json_stays_structured(self):
        args=argparse.Namespace(quiet=False,json=False)
        result={'ready':True,'site':'site.test','status':'ready',
            'shareFiles':{'MTProto':'/opt/hton/work/minimum/share/mtproto.txt'}}
        stream=io.StringIO()
        with patch('install.sys.stdout',stream):
            install.emit(args,result,page_url='https://site.test/secret',
                share_directory='/opt/hton/.secrets/share')
        text=stream.getvalue()
        self.assertIn('OK: Setup complete.',text)
        self.assertIn('https://site.test',text)
        self.assertIn('Client access page: https://site.test/secret',text)
        self.assertIn('Client files: /opt/hton/.secrets/share',text)
        self.assertNotIn('MTProto:',text)
        args.json=True;stream=io.StringIO()
        with patch('install.sys.stdout',stream):install.emit(args,result)
        self.assertEqual(json.loads(stream.getvalue()),result)

    def test_migrations_remove_legacy_material_once(self):
        config = Store(self.root / 'config', create=True)
        try:
            self.store.write('share/outline.yaml', b'legacy')
            self.store.write('secrets/outline-secret', b'legacy')
            config.write('modules/foreign-outline.yaml', b'legacy')
            migrations.apply(self.store, config, {}, None)
            self.assertFalse((self.root / 'share/outline.yaml').exists())
            self.assertTrue((self.root / 'secrets/outline-secret').exists())
            self.assertFalse((self.root / 'config/modules/foreign-outline.yaml').exists())
            self.assertEqual(self.store.json('migrations.json')['done'],
                ['0001-remove-legacy-outline-material', '0002-runtime-permissions'])
            config.write('modules/foreign-outline.yaml', b'again')
            migrations.apply(self.store, config, {}, None)
            self.assertTrue((self.root / 'config/modules/foreign-outline.yaml').exists())
        finally:
            config.close()

    def test_minimum_deploy_manifest_is_runtime_closure(self):
        import importlib.machinery
        import importlib.util
        repo = Path(__file__).resolve().parents[2]
        loader = importlib.machinery.SourceFileLoader('hton_deploy_test', str(repo / 'scripts' / 'deploy'))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        deploy = importlib.util.module_from_spec(spec)
        loader.exec_module(deploy)
        files, _digest = deploy.source_files(repo)
        names = {name for name, *_rest in files}
        for required in ('bin/hton', 'scripts/install', 'scripts/setup', 'scripts/deploy',
                'lib/hton', 'lib/bootstrap', 'lib/install_config.py',
                'platform/etc/install.json', 'platform/images.lock.json'):
            self.assertIn(required, names)
        for prefix in ('crates/', 'runtime/', 'contracts/', 'adapters/', 'schemas/',
                'modules/', 'platform/modules/', 'platform/systemd/', 'platform/web/', 'tests/'):
            self.assertFalse(any(name.startswith(prefix) for name in names), prefix)
        for forbidden in ('Cargo.toml', 'Cargo.lock', 'hton.example.yaml',
                'tooling/build-minimum', 'tooling/package-release',
                'scripts/install-ubuntu', 'scripts/install-unknown'):
            self.assertNotIn(forbidden, names)
        self.assertLess(len(files), 120)
        self.assertIn('python', deploy.MANAGED_TOP)
        self.assertNotIn('python/minimum', deploy.MANAGED_TOP)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'python/minimum').mkdir(parents=True)
            (root / 'python/minimum/install.py').write_text('x')
            (root / 'config').mkdir()
            deploy.ROOT = root
            deploy.prune_top_level({'python', 'config'})
            self.assertTrue((root / 'python/minimum/install.py').exists())
            self.assertTrue((root / 'config').is_dir())

    def test_deploy_copies_private_secrets_without_git(self):
        import importlib.machinery
        import importlib.util
        repo = Path(__file__).resolve().parents[2]
        loader = importlib.machinery.SourceFileLoader('hton_deploy_secrets_test', str(repo / 'scripts' / 'deploy'))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        deploy = importlib.util.module_from_spec(spec)
        loader.exec_module(deploy)
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            source = base / 'source'
            (source / '.secrets').mkdir(parents=True)
            (source / '.secrets/profile.yml').write_text('schemaVersion: 1\n')
            (source / '.secrets/images.lock.json').write_text('{}\n')
            (source / '.secrets/.git').mkdir()
            (source / '.secrets/.git' / 'config').write_text('x')
            target = base / 'target'
            target.mkdir()
            deploy.ROOT = target
            count = deploy.copy_secrets_tree(source)
            self.assertEqual(count, 2)
            self.assertTrue((target / '.secrets/profile.yml').is_file())
            self.assertFalse((target / '.secrets/.git').exists())
            self.assertEqual((target / '.secrets/profile.yml').stat().st_mode & 0o777, 0o600)
            (source / '.secrets/leak').symlink_to('/etc/passwd')
            with self.assertRaises(deploy.DeployError):
                deploy.copy_secrets_tree(source)

    def test_protocol_selection_is_bounded_and_idempotent(self):
        self.assertIsNone(protocols.parse_selection(None, '--with'))
        self.assertEqual(protocols.parse_selection('mtg,outline', '--with'),
            {'mtproto', 'outline-wss'})
        for value in ('', 'mtproto,', 'unknown', 'mtproto,mtg', 'mtproto,hysteria,outline,mtg'):
            with self.assertRaises(InstallError): protocols.parse_selection(value, '--with')
        self.assertEqual(protocols.selected({'features': {'foreign-amnezia': {'enabled': True}}}, None, None),
            set(protocols.PROTOCOLS))
        self.assertEqual(protocols.selected({'features': {'foreign-amnezia': {'enabled': True}}},
            None, None, state={'protocolsChosen': True}), set())
        root = {'features': {'mtproto': {'enabled': True}, 'foreign-amnezia': {'enabled': True}}}
        self.assertEqual(protocols.selected(root, None, None), {'mtproto'})
        self.assertEqual(protocols.selected(root, 'hysteria', None), {'mtproto', 'hysteria'})
        self.assertEqual(protocols.selected(root, None, 'mtproto'), set())
        with self.assertRaises(InstallError): protocols.selected(root, 'hysteria', 'hysteria')
        with self.assertRaises(InstallError): protocols.selected({'features': {'openvpn': {'enabled': True}}}, None, None)

    def test_protocol_rendering_and_share_links(self):
        mtg = protocols.mtg_config('ee' + 'a' * 64, 41000)
        self.assertIn(b'bind-to = "127.0.0.1:41000"', mtg)
        self.assertIn(b'secret = "ee', mtg)
        config = protocols.hysteria_config('site.test', 443, 'a' * 64)
        self.assertIn(b'listen: :443', config)
        self.assertIn(b'url: https://site.test/', config)
        plan = {'mtproto': {'secret': 'ee' + 'a' * 64}, 'hysteria': {'password': 'b' * 64, 'port': 443}}
        links = protocols.share_links(plan, 'site.test')
        self.assertTrue(links['MTProto'].startswith('tg://proxy?server=site.test&port=443&secret=ee'))
        self.assertEqual(links['Hysteria'],
            'hysteria2://' + 'b' * 64 + '@site.test:443/?sni=site.test#Hysteria2')

    def test_outline_wss_rendering_and_access_key(self):
        config = protocols.outline_config('b' * 43, 41002, '/' + 'a' * 48, '/' + 'b' * 48)
        self.assertIn(b'websocket-stream', config)
        self.assertIn(b'websocket-packet', config)
        self.assertIn(b'127.0.0.1:41002', config)
        key = protocols.outline_key('site.test',
            {'secret': 'b' * 43, 'tcpPath': '/' + 'a' * 48, 'udpPath': '/' + 'b' * 48})
        self.assertIn(b'wss://site.test/' + b'a' * 48, key)
        self.assertIn(b'wss://site.test/' + b'b' * 48, key)
        users = [{'name': 'alice', 'secret': 'a' * 43}, {'name': 'bob', 'secret': 'b' * 43}]
        multi = protocols.outline_config('a' * 43, 41002, '/' + 'c' * 48, '/' + 'd' * 48, users)
        self.assertIn(b'id: alice', multi)
        self.assertIn(b'id: bob', multi)
        self.assertIn(b'secret: "' + b'a' * 43 + b'"', multi)
        self.assertIn(b'secret: "' + b'b' * 43 + b'"', multi)
        bob = protocols.outline_key('site.test',
            {'tcpPath': '/' + 'c' * 48, 'udpPath': '/' + 'd' * 48},
            secret=users[1]['secret'], name='bob')
        self.assertIn(b'secret: "' + b'b' * 43 + b'"', bob)
        self.assertNotIn(b'secret: "' + b'a' * 43 + b'"', bob)

    def test_chain_selfsteal_and_single_sni_ingress(self):
        native = {"inbounds": [{"listen": "0.0.0.0", "port": 443, "protocol": "vless",
            "settings": {"clients": [{"id": "u", "flow": "xtls-rprx-vision"}]},
            "streamSettings": {"network": "tcp", "security": "reality",
                "realitySettings": {"privateKey": "k", "shortIds": ["0123"],
                    "serverNames": ["old.test"], "dest": "old.test:443"}}}],
            "outbounds": [{"protocol": "freedom"}]}
        config = json.loads(protocols.selfsteal_config(native, "www.site.test", 41003, 41000))
        inbound = config["inbounds"][0]
        self.assertEqual(inbound["listen"], "127.0.0.1")
        self.assertEqual(inbound["port"], 41003)
        self.assertEqual(inbound["streamSettings"]["realitySettings"]["serverNames"], ["www.site.test"])
        self.assertEqual(inbound["streamSettings"]["realitySettings"]["dest"], "127.0.0.1:41000")
        self.assertEqual(inbound["settings"]["clients"], native["inbounds"][0]["settings"]["clients"])
        ha, site = ingress.configs("site.test", ["www.googletagmanager.com"], 5512, 9443, b"hello",
            mtproto={"camouflage": "www.site.test", "port": 41000},
            selfsteal={"port": 41003}, chain_sni="www.site.test")
        self.assertIn(b"use_backend selfsteal if { req.ssl_sni -i www.site.test }", ha)
        self.assertIn(b"server reality 127.0.0.1:41003 check", ha)
        self.assertIn(b"server site 127.0.0.1:9443 check backup", ha)
        self.assertNotIn(b"use_backend mtproto", ha)
        caddy = json.loads(site)
        policy = caddy["apps"]["http"]["servers"]["site"]["tls_connection_policies"][0]["match"]["sni"]
        self.assertEqual(policy, ["site.test", "www.site.test"])
        mtg = protocols.mtg_config("ee" + "a" * 64, 41000, 9443)
        self.assertIn(b"[domain-fronting]", mtg)
        self.assertIn(b"port = 9443", mtg)

    def test_test_tunnel_routes_only_its_own_sni(self):
        ha, _site = ingress.configs("site.test", ["reality.site.test"], 5512, 9443, b"hello",
            test_tunnel={"sni": "probe.site.test", "port": 19090})
        self.assertIn(b"use_backend test_tunnel if { req.ssl_sni -i probe.site.test }", ha)
        self.assertIn(b"backend test_tunnel", ha)
        self.assertIn(b"server tunnel 127.0.0.1:19090 check", ha)
        self.assertIn(b"acl allowed req.ssl_sni -i site.test www.site.test reality.site.test probe.site.test", ha)
        for bad in (
            {"sni": "site.test", "port": 19090},
            {"sni": "reality.site.test", "port": 19090},
            {"sni": "probe.site.test", "port": 80},
            {"sni": "", "port": 19090},
        ):
            with self.assertRaises(InstallError):
                ingress.configs("site.test", ["reality.site.test"], 5512, 9443, b"hello",
                    test_tunnel=bad)

    def test_native_selfsteal_config_preserves_reality_material(self):
        value = {"log": {"loglevel": "warning"}, "outbounds": [{"protocol": "freedom"}],
            "inbounds": [{"listen": "0.0.0.0", "port": 443, "protocol": "vless",
                "settings": {"clients": [{"id": "u", "flow": "xtls-rprx-vision"}], "decryption": "none"},
                "streamSettings": {"network": "tcp", "security": "reality",
                    "realitySettings": {"privateKey": "k", "shortIds": ["0123"],
                        "serverNames": ["old.test"], "dest": "old.test:443", "target": "old.test:443"}}}]}
        content = protocols.native_selfsteal_config(value, ["site.test", "old.test"], "172.17.0.1:41000")
        config = json.loads(content)
        inbound = config["inbounds"][0]
        reality = inbound["streamSettings"]["realitySettings"]
        self.assertEqual(reality["serverNames"], ["site.test", "old.test"])
        self.assertEqual(reality["dest"], "172.17.0.1:41000")
        self.assertEqual(reality["privateKey"], "k")
        self.assertEqual(reality["shortIds"], ["0123"])
        self.assertNotIn("target", reality)
        self.assertEqual(inbound["settings"]["clients"], value["inbounds"][0]["settings"]["clients"])
        self.assertEqual((inbound["listen"], inbound["port"]), ("0.0.0.0", 443))
        with self.assertRaises(InstallError): protocols.native_selfsteal_config(value, [], "172.17.0.1:1")
        with self.assertRaises(InstallError): protocols.native_selfsteal_config(value, ["site.test"], "not-a-destination")

    def test_native_selfsteal_ingress_routes_own_sni_with_site_fallback(self):
        ha, site = ingress.configs("site.test", ["site.test", "old.test"], 5512, 9443, b"hello",
            native_selfsteal=True, site_bind="172.17.0.1")
        self.assertIn(b"use_backend amnezia if { req.ssl_sni -i site.test old.test }", ha)
        self.assertNotIn(b"use_backend selfsteal", ha)
        self.assertIn(b"server site 127.0.0.1:9443 check backup", ha)
        caddy = json.loads(site)
        self.assertEqual(caddy["apps"]["http"]["servers"]["site"]["listen"],
            ["127.0.0.1:9443", "172.17.0.1:9443"])
        with self.assertRaises(InstallError):
            ingress.configs("site.test", ["site.test"], 5512, 9443, b"x")
        with self.assertRaises(InstallError):
            ingress.configs("site.test", ["site.test"], 5512, 9443, b"x",
                native_selfsteal=True, selfsteal={"port": 41003})

    def test_native_migration_plan_targets_bridge_mtg(self):
        config = Store(self.root / "config", create=True)
        native_server = {"inbounds": [{"listen": "0.0.0.0", "port": 443, "protocol": "vless",
            "settings": {"clients": [{"id": "u", "flow": "xtls-rprx-vision"}]},
            "streamSettings": {"network": "tcp", "security": "reality",
                "realitySettings": {"privateKey": "k", "shortIds": ["0123"],
                    "serverNames": ["old.test"], "dest": "old.test:443"}}}],
            "outbounds": [{"protocol": "freedom"}]}
        try:
            args = argparse.Namespace(yes=False, json=True, mtproto_camouflage=None,
                outline_binary=None, hysteria_port=8443)
            state = {"instance": "i" * 32, "sitePort": 9443}
            with patch("protocols._mtg_secret", return_value=("ee" + "a" * 64, "secrets/mtproto-secret")):
                plan = protocols.plan(config, self.store, state,
                    {"mtg": "mtg@sha256:" + "d" * 64}, "site.test", ["old.test"],
                    {"mtproto"}, args, {}, ask=lambda label, default="": default,
                    confirm=lambda label: False, native_server=native_server,
                    chain_sni="site.test", site_port=9443, migrate_native=True,
                    bridge_gateway="172.17.0.1", mtg_port=41000)
            self.assertIsNone(plan["selfsteal"])
            self.assertEqual(plan["native"]["names"], ["site.test", "old.test"])
            self.assertEqual(plan["native"]["dest"], "172.17.0.1:41000")
            self.assertEqual(plan["mtproto"]["bind"], "172.17.0.1")
            migrated = json.loads(plan["native"]["content"])
            self.assertEqual(migrated["inbounds"][0]["settings"]["clients"],
                native_server["inbounds"][0]["settings"]["clients"])
        finally:
            config.close()

    def test_native_structure_ignores_clients_but_detects_route_change(self):
        def config(clients, names=None, dest="old.test:443"):
            return json.dumps({"log": {"loglevel": "warning"},
                "inbounds": [{"listen": "0.0.0.0", "port": 443, "protocol": "vless",
                    "settings": {"clients": clients, "decryption": "none"},
                    "streamSettings": {"network": "tcp", "security": "reality",
                        "realitySettings": {"privateKey": "k", "shortIds": ["01"],
                            "serverNames": names or ["old.test"], "dest": dest}}}],
                "outbounds": [{"protocol": "freedom"}]}).encode()
        first = docker.native_structure_digest(config([{"id": "a"}, {"id": "b"}]))
        second = docker.native_structure_digest(config([{"id": "c"}]))
        self.assertEqual(first, second)
        changed = docker.native_structure_digest(config([{"id": "a"}], dest="172.17.0.1:1"))
        self.assertNotEqual(first, changed)

    def test_mtg_validation_rewrites_bridge_bind_for_networkless_check(self):
        self.store.write("protocols/mtg/mtg.toml",
            b'secret = "x"\nbind-to = "172.17.0.1:41000"\n')
        value = {"directory": "protocols/mtg", "files": ["protocols/mtg/mtg.toml"],
            "port": 41000, "bind": "172.17.0.1"}
        directory, files, temporary = protocols._mtg_validation_artifact(self.store, value)
        content = self.store.read(files[0])
        self.assertIn(b'bind-to = "127.0.0.1:', content)
        self.assertNotIn(b"172.17.0.1", content)
        self.assertEqual(directory + "/mtg.toml", temporary)
        path = self.store.root / temporary
        path.unlink()
        path.parent.rmdir()
        self.assertEqual(protocols._mtg_validation_artifact(self.store,
            {"directory": "protocols/mtg", "files": ["protocols/mtg/mtg.toml"],
             "port": 41000, "bind": "127.0.0.1"}), ("protocols/mtg",
             ["protocols/mtg/mtg.toml"], None))

    def test_native_migration_updates_digests_and_rolls_back_on_exit(self):
        from storage import digest
        old, new = b'{"old":true}\n', b'{"new":true}\n'
        def state(running=True):
            return {'instance': 'i' * 32, 'hasCommitted': True, 'replacement': 'a' * 64,
                'nativeDigest': digest(old), 'replacementNativeDigest': digest(old),
                'ownedReceipts': {}, 'originalId': 'b' * 64,
                'replacementName': 'hton-minimum-replacement-x',
                'replacementParked': 'hton-minimum-retained-x',
                'originalParked': 'hton-minimum-original-x'}
        migration = {'content': new, 'digest': digest(new), 'previous': old,
            'previousDigest': digest(old), 'names': ['site.test'], 'dest': '172.17.0.1:41000'}
        value = state()
        op = install.Installation(None, self.store, value)
        with patch.object(op, 'check_foreign', return_value={'State': {'Running': True}}), \
             patch.object(op, 'validate_native_bytes'), \
             patch('install.docker.native_config', side_effect=[old, new]), \
             patch('install.docker.replace_native_config'), \
             patch('install.docker.command'), \
             patch('install.docker.inspect', return_value={'State': {'Running': True}}):
            op.migrate_native(migration)
        self.assertEqual(value['nativeDigest'], digest(new))
        self.assertEqual(value['replacementNativeDigest'], digest(new))
        self.assertEqual(value['nativeMigration']['toDigest'], digest(new))
        self.assertNotIn('nativeMigrationPending', value)
        self.assertEqual(self.store.read('native/previous.json'), old)

        failed = state()
        op = install.Installation(None, self.store, failed)
        with patch.object(op, 'check_foreign', return_value={'State': {'Running': True}}), \
             patch.object(op, 'validate_native_bytes'), \
             patch('install.docker.native_config', side_effect=[old, new]), \
             patch('install.docker.replace_native_config'), \
             patch('install.docker.command'), \
             patch('install.docker.inspect', return_value={'State': {'Running': False}}), \
             patch('install.time.sleep'):
            with self.assertRaisesRegex(InstallError, 'previous configuration restored'):
                op.migrate_native(migration)
        self.assertEqual(failed['nativeDigest'], digest(old))
        self.assertEqual(failed['replacementNativeDigest'], digest(old))
        self.assertNotIn('nativeMigration', failed)

    def test_native_restore_clears_migration_and_keeps_backup(self):
        from storage import digest
        old, new = b'{"legacy":true}\n', b'{"own":true}\n'
        value = {'instance': 'i' * 32, 'hasCommitted': True, 'replacement': 'a' * 64,
            'nativeDigest': digest(new), 'replacementNativeDigest': digest(new),
            'originalNativeDigest': digest(old), 'originalNativeNames': ['legacy.test'],
            'nativeMigration': {'names': ['site.test'], 'dest': '172.17.0.1:1'},
            'nativeMigrationPending': {'names': ['site.test'], 'dest': '172.17.0.1:1'},
            'ownedReceipts': {}, 'originalId': 'b' * 64}
        op = install.Installation(None, self.store, value)
        migration = {'content': old, 'digest': digest(old), 'previous': new,
            'previousDigest': digest(new), 'names': ['legacy.test'], 'dest': 'legacy.test:443'}
        with patch.object(op, 'check_foreign', return_value={'State': {'Running': True}}), \
             patch.object(op, 'validate_native_bytes'), \
             patch('install.docker.native_config', side_effect=[new, old]), \
             patch('install.docker.replace_native_config'), \
             patch('install.docker.command'), \
             patch('install.docker.inspect', return_value={'State': {'Running': True}}):
            op.migrate_native(migration, legacy=True)
        self.assertEqual(value['nativeDigest'], digest(old))
        self.assertNotIn('nativeMigration', value)
        self.assertNotIn('nativeMigrationPending', value)
        self.assertEqual(value['legacySniRestored'], digest(old))
        self.assertEqual(self.store.read('native/migrated.json'), new)

    def test_mtproto_route_precedes_native_and_requires_distinct_sni(self):
        ha, _site = ingress.configs('site.test', ['native.test'], 5512, 9443, b'hello',
            mtproto={'camouflage': 'www.microsoft.com', 'port': 41000})
        self.assertIn(b'use_backend mtproto if { req.ssl_sni -i www.microsoft.com }', ha)
        self.assertIn(b'server proxy 127.0.0.1:41000 check', ha)
        self.assertLess(ha.index(b'use_backend mtproto'), ha.index(b'use_backend amnezia'))
        self.assertIn(b'www.microsoft.com', ha.split(b'acl allowed')[1].split(b'\n')[0])
        for camouflage in ('site.test', 'native.test'):
            with self.assertRaises(InstallError):
                ingress.configs('site.test', ['native.test'], 5512, 9443, b'hello',
                    mtproto={'camouflage': camouflage, 'port': 41000})

    def test_protocol_plan_persists_settings_and_rejects_conflicts(self):
        config = Store(self.root / 'config', create=True)
        try:
            args = argparse.Namespace(yes=True, json=True, mtproto_camouflage='www.microsoft.com',
                outline_binary='/tmp/outline-ss-server', hysteria_port=443)
            values = {'a' * 64: {'Id': 'a' * 64, 'Name': '/outline-test', 'State': {'Running': True},
                'Config': {'Cmd': ['/outline/outline-ss-server'], 'Labels': {}},
                'HostConfig': {'PortBindings': {'8443/tcp': [{'HostIp': '', 'HostPort': '8443'}]},
                    'RestartPolicy': {'Name': 'unless-stopped', 'MaximumRetryCount': 0}},
                'Image': 'sha256:' + 'b' * 64, 'Mounts': [], 'NetworkSettings': {'Networks': {}}}}
            state = {'instance': 'i' * 32}
            with patch('protocols._mtproto_port', return_value=41000), patch('protocols._mtg_secret', return_value=('ee' + 'a' * 64, 'secrets/mtproto-secret')), patch('protocols._hysteria_password', return_value='c' * 64), patch('protocols._outline_binary', return_value=('/tmp/outline-ss-server', protocols.OUTLINE_SHA256)), patch('protocols._outline_secret', return_value='b' * 43), patch('protocols._outline_path', side_effect=['/' + 'a' * 48, '/' + 'b' * 48]), patch('protocols._outline_web_port', return_value=41002), patch('docker.valid_domain', return_value=True), patch('protocols._port_free', return_value=True):
                plan = protocols.plan(config, self.store, state,
                    {'mtg': 'mtg@sha256:' + 'd' * 64, 'hysteria': 'hysteria@sha256:' + 'e' * 64},
                    'site.test', ['native.test'], {'mtproto', 'hysteria', 'outline-wss'},
                    args, values, ask=lambda label, default='': default,
                    confirm=lambda label: False)
            self.assertEqual(plan['mtproto']['camouflage'], 'www.microsoft.com')
            self.assertEqual(plan['hysteria']['port'], 443)
            self.assertEqual(plan['outline']['webPort'], 41002)
            self.assertTrue(plan['generation'])
            protocols.save_config(config, {'features': {}}, plan)
            protocols.record_state(state, plan)
            self.assertEqual(state['mtprotoPort'], 41000)
            self.assertEqual(state['outlineWebPort'], 41002)
            self.assertEqual(state['protocols']['outline-wss']['webPort'], 41002)
            self.assertTrue(document(config, 'modules/mtproto.yaml')['settings']['camouflageName'])
            self.assertEqual(document(config, 'modules/outline-wss.yaml')['settings']['streamPath']['secretRef'], 'outline/tcp-path')
            changed = argparse.Namespace(yes=False, json=True, mtproto_camouflage='other.example',
                outline_binary='/tmp/outline-ss-server', hysteria_port=443)
            with patch('protocols._mtproto_port', return_value=41001), patch('protocols._mtg_secret', return_value=('ee' + 'a' * 64, 'secrets/mtproto-secret')), patch('protocols._hysteria_password', return_value='c' * 64), patch('docker.valid_domain', return_value=True), patch('protocols._port_free', return_value=True):
                with self.assertRaisesRegex(InstallError, 'camouflage'):
                    protocols.plan(config, self.store, state,
                        {'mtg': 'mtg', 'hysteria': 'hysteria'}, 'site.test', ['native.test'],
                        {'mtproto'}, changed, values, ask=lambda label, default='': default,
                        confirm=lambda label: False)
        finally:
            config.close()

    def test_chain_dependency_starts_before_route_switch(self):
        state = {"instance": "i" * 32, "ownedReceipts": {},
            "candidateProtocols": {"mtproto": "1" * 64, "selfsteal": "2" * 64}}
        commands = []
        def command(*args, **kwargs):
            commands.append(args)
            return b""
        with patch("protocols.ingress.owned", return_value={"State": {"Running": True}}), \
             patch("protocols.docker.command", side_effect=command), \
             patch("protocols.docker.inspect", return_value={"State": {"Running": True}}):
            protocols.start_chain(self.store, state, {"native": {"names": ["x"]}, "mtproto": {"port": 1}})
            self.assertEqual([call[2] for call in commands if call[:2] == ("container", "start")],
                ["1" * 64])
            commands.clear()
            protocols.start_chain(self.store, state, {"selfsteal": {"port": 1}, "mtproto": {"port": 1}})
            self.assertEqual([call[2] for call in commands if call[:2] == ("container", "start")],
                ["2" * 64, "1" * 64])
            commands.clear()
            protocols.start_chain(self.store, state, {"native": {"names": ["x"]}})
            self.assertEqual(commands, [])
            with self.assertRaisesRegex(InstallError, "no prepared candidate"):
                protocols.start_chain(self.store, {"instance": "i" * 32, "ownedReceipts": {}},
                    {"native": {"names": ["x"]}, "mtproto": {"port": 1}})

    def test_protocol_candidate_promotion_and_rollback_state(self):
        state = {'instance': 'i' * 32, 'ownedReceipts': {}}
        plan = {'mtproto': {'directory': 'protocols/mtg', 'files': ['protocols/mtg/x.toml'],
                    'secretFile': 'secrets/mtproto-secret'},
            'hysteria': {'directory': 'protocols/hy', 'files': ['protocols/hy/x.yaml'],
                    'secretFile': 'secrets/hysteria-password'}}
        self.store.write('protocols/mtg/x.toml', b'x')
        self.store.write('protocols/hy/x.yaml', b'y')
        self.store.write('secrets/mtproto-secret', b'ee' + b'a' * 64)
        self.store.write('secrets/hysteria-password', b'b' * 64)
        created = iter(('1' * 64, '2' * 64))
        commands = []
        def command(*args, **kwargs):
            commands.append(args)
            if tuple(args[:2]) == ('container', 'create'):
                return next(created).encode()
            return b''
        with patch('install.docker.command', side_effect=command), patch('protocols.docker.command', side_effect=command), patch('protocols.docker.inventory', return_value={}), patch('protocols.ingress.record_owned'):
            protocols.prepare(self.store, state, {'mtg': 'mtg', 'hysteria': 'hy'},
                plan, Path('/certs'), lambda: None)
        self.assertEqual(state['candidateProtocols'], {'mtproto': '1' * 64, 'hysteria': '2' * 64})
        with patch('protocols.ingress.owned', return_value={'State': {'Running': True}}), patch('protocols.docker.command', side_effect=command):
            protocols.promote(self.store, state)
            protocols.accept(state)
            self.assertEqual(state['activeProtocols'], {'mtproto': '1' * 64, 'hysteria': '2' * 64})
            self.assertNotIn('candidateProtocols', state)
            state['candidateProtocols'] = {'mtproto': '3' * 64}
            protocols.rollback_candidates(self.store, state)
            self.assertEqual(state['activeProtocols'], {'mtproto': '1' * 64, 'hysteria': '2' * 64})
        updates = [call for call in commands if tuple(call[:2]) == ('container', 'update')]
        self.assertTrue(any('--restart=always' in call for call in updates))
        self.assertTrue(any('--restart=no' in call for call in updates))

    def test_update_due_after_seven_days_without_install(self):
        self.assertTrue(updates.due(self.store, now=1000))
        updates.record(self.store, 'install', now=1000)
        self.assertFalse(updates.due(self.store, now=1000 + 7 * 24 * 3600 - 1))
        self.assertTrue(updates.due(self.store, now=1000 + 7 * 24 * 3600))
        self.assertTrue(updates.due(self.store, now=999))
        self.assertEqual(updates.remaining(self.store, now=1000), 7 * 24 * 3600)

    def test_update_units_check_daily_and_run_the_update_script(self):
        with patch('install.ROOT', Path('/opt/hton-fixture')), \
                patch('install.unit_preflight'), patch('docker.run') as run:
            install.install_units(self.store)
        service = self.store.read('units/hton-minimum-update.service').decode()
        timer = self.store.read('units/hton-minimum-update.timer').decode()
        self.assertIn('scripts/update --auto --quiet', service)
        self.assertIn('TimeoutStartSec=3600', service)
        self.assertIn('OnCalendar=*-*-* 04:30:00', timer)
        self.assertIn('Persistent=true', timer)
        linked = [str(call) for call in run.call_args_list if 'link' in call.args[0]]
        self.assertTrue(linked and 'hton-minimum-update.service' in linked[0])

    def test_successful_update_records_the_schedule_stamp(self):
        args = argparse.Namespace(action='update', quiet=True, json=False)
        with patch('install.normalize_runtime_permissions'), \
                patch('install.protocols.share_links', return_value={}), \
                patch('install.native_share_links', return_value=({}, {})), \
                patch('install.ready_result', return_value={'ready': True}), \
                patch('install.emit'), patch('install.updates.record') as record:
            install.emit_ready(args, 'site.test', 5512, 'http-01', 'verified', None, {}, self.store, None)
        record.assert_called_once_with(self.store, 'update')

    def test_fast_forward_advances_only_main(self):
        if shutil.which('git') is None:
            self.skipTest('git is unavailable')
        remote = self.root / 'remote.git'
        work = self.root / 'work'
        other = self.root / 'other'
        def git(directory, *arguments):
            return subprocess.run(['git', '-C', str(directory), *arguments],
                capture_output=True, text=True, check=True)
        git(self.root, 'init', '--bare', '-b', 'main', str(remote))
        git(self.root, 'clone', '-q', str(remote), str(work))
        git(work, 'config', 'user.email', 'test@example.test')
        git(work, 'config', 'user.name', 'test')
        git(work, 'checkout', '-q', '-b', 'main')
        (work / 'f').write_text('one\n')
        git(work, 'add', 'f')
        git(work, 'commit', '-qm', 'one')
        git(work, 'push', '-q', '-u', 'origin', 'main')
        git(self.root, 'clone', '-q', '-b', 'main', str(remote), str(other))
        git(other, 'config', 'user.email', 'test@example.test')
        git(other, 'config', 'user.name', 'test')
        (other / 'f').write_text('two\n')
        git(other, 'commit', '-qam', 'two')
        git(other, 'push', '-q', 'origin', 'main')
        result = updates.fast_forward(work)
        self.assertEqual(result['status'], 'updated')
        self.assertEqual((work / 'f').read_text(), 'two\n')
        feature = self.root / 'feature'
        git(self.root, 'clone', '-q', str(remote), str(feature))
        git(feature, 'checkout', '-q', '-b', 'feature')
        self.assertEqual(updates.fast_forward(feature)['status'], 'skipped')
        self.assertEqual(updates.fast_forward(self.root / 'plain')['reason'], 'not-a-git-checkout')

if __name__=='__main__': unittest.main()
