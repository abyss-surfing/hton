#!/usr/bin/env python3
"""Disposable native routing acceptance; never uses production ports or secrets."""
import base64
import http.server
import json
import os
from pathlib import Path
import secrets
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
import time
import uuid

sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'python/minimum'))
import docker
import ingress
from client_page import HEADERS
from http_fixture import exercise_http_issuer
from storage import InstallError, Store


def port():
    with socket.socket() as s:
        s.bind(('127.0.0.1',0));return s.getsockname()[1]


def main():
    lock=ROOT/'.secrets/images.lock.json'
    if not lock.is_file(): lock=ROOT/'.secrets.example/images.lock.json'
    if not lock.is_file(): lock=ROOT/'platform/images.lock.json'
    pins={k:v['reference'] for k,v in json.loads(lock.read_text())['images'].items()}
    containers=[]; images=[]; networks=[]; token=secrets.token_hex(12)
    label='io.hton.minimum-test='+token
    def create(image,*args,options=()):
        identifier=docker.exact_id(docker.command('container','create','--pull','never','--label',label,
            '--restart','no',*options,image,*args).decode().strip())
        containers.append(identifier)
        return identifier
    def start(identifier):docker.command('container','start',identifier)
    try:
        for key in ('haproxy','caddy','xray','acme'):
            try:docker.command('image','inspect',pins[key])
            except InstallError:docker.command('image','pull','--quiet',pins[key],timeout=180)
        issuer=create(pins['acme'],'--version','--home','/acmebin',
            options=['--network','none','--read-only','--entrypoint','/usr/local/bin/acme.sh'])
        docker.command('container','start','--attach',issuer)
        if docker.inspect(issuer)['State']['ExitCode']!=0:raise AssertionError('Pinned issuer executable failed')
        # Read packaged native provider code without making any DNS/ACME calls.
        docker.command('container','cp',issuer+':/acmebin/dnsapi/dns_internetbs.sh','-',limit=131072)
        print('PASS: pinned issuer and InternetBS provider available offline',flush=True)
        with tempfile.TemporaryDirectory(prefix='hton-minimum-test-') as temporary:
            root=Path(temporary)
            uid=f'{os.getuid()}:{os.getgid()}'
            siteport,edgeport,nativeport,socksport=port(),port(),port(),port()
            body=b'HTON isolated acceptance\n'
            def openssl(*args):docker.run(['/usr/bin/openssl',*args])
            openssl('req','-x509','-newkey','rsa:2048','-nodes','-keyout',str(root/'ca.key'),'-out',str(root/'ca.pem'),'-days','2','-subj','/CN=HTON fixture CA')
            openssl('req','-new','-newkey','rsa:2048','-nodes','-keyout',str(root/'key.pem'),'-out',str(root/'site.csr'),'-subj','/CN=site.test')
            (root/'extensions').write_text('subjectAltName=DNS:site.test,DNS:native.test\nextendedKeyUsage=serverAuth\n')
            openssl('x509','-req','-in',str(root/'site.csr'),'-CA',str(root/'ca.pem'),'-CAkey',str(root/'ca.key'),'-CAcreateserial','-out',str(root/'fullchain.pem'),'-days','2','-extfile',str(root/'extensions'))
            exercise_http_issuer(root,pins,token,uid,containers)
            exercise_http_issuer(root,pins,token,uid,containers,webroot=True)
            page_dir = root / 'client-page'
            page_dir.mkdir()
            page_body = b'fixture guest inventory\n'
            (page_dir / 'index.html').write_bytes(page_body)
            ha,site=ingress.configs('site.test',['native.test'],nativeport,siteport,body,
                client_page={'path': 'Fixture-Page', 'headers': HEADERS})
            ha=ha.replace(b'bind 0.0.0.0:443',f'bind 127.0.0.1:{edgeport}'.encode())
            site=json.loads(site)
            srv=site['apps']['http']['servers']['site']
            srv['tls_connection_policies'][0]['match']['sni'].append('native.test')
            srv['routes'][-2]['match'][0]['host'].append('native.test')
            (root/'haproxy.cfg').write_bytes(ha)
            (root/'caddy.json').write_text(json.dumps(site))
            mount=['--network','host','--user',uid,'--read-only','--cap-drop','ALL','--cap-add','NET_BIND_SERVICE','--security-opt','no-new-privileges:true','--log-driver','local',
                '--tmpfs','/data:rw,mode=1777,size=16777216','--tmpfs','/config:rw,mode=1777,size=16777216',
                '--mount',f'type=bind,src={root},dst=/certs,readonly','--mount',f'type=bind,src={root},dst=/etc/hton,readonly',
                '--mount',f'type=bind,src={page_dir},dst=/client-page,readonly']
            siteid=create(pins['caddy'],'run','--config','/etc/hton/caddy.json',options=mount+['--entrypoint','caddy'])
            edgeid=create(pins['haproxy'],'-W','-db','-f','/etc/hton/haproxy.cfg',options=mount+['--entrypoint','haproxy'])
            start(siteid);start(edgeid)
            for attempt in range(40):
                try:
                    response=docker.run(['/usr/bin/curl','--disable','--silent','--show-error','--noproxy','*','--cacert',str(root/'ca.pem'),
                        '--resolve',f'site.test:{edgeport}:127.0.0.1',f'https://site.test:{edgeport}/'],timeout=5)
                    if response!=body:raise AssertionError('Site body mismatch')
                    break
                except InstallError:
                    if attempt==39:
                        for fixture in (siteid,edgeid):
                            print(docker.inspect(fixture)['State'],flush=True)
                            print(subprocess.run(docker.DOCKER+['container','logs',fixture],capture_output=True,text=True).stderr,flush=True)
                        raise
                    time.sleep(.15)
            try:ingress.website('site.test',edgeport,body)
            except ssl.SSLCertVerificationError:pass
            else:raise AssertionError('Production trust accepted fixture CA')
            print('PASS: native HAProxy/Caddy site routing and default trust rejection',flush=True)
            def request_page(target, *options):
                response = docker.run(['/usr/bin/curl', '--disable', '--silent', '--show-error',
                    '--noproxy', '*', '--cacert', str(root/'ca.pem'), '--path-as-is',
                    '--resolve', f'site.test:{edgeport}:127.0.0.1', '--write-out', '\n%{http_code}',
                    *options, f'https://site.test:{edgeport}{target}'], timeout=5)
                return response.rsplit(b'\n', 1)

            for target in ('/Fixture-Page', '/Fixture-Page?query=1'):
                if request_page(target) != [page_body, b'200']:
                    raise AssertionError('Exact client page request failed')
            for target in ('/fixture-page', '/%46ixture-Page', '//Fixture-Page', '/x/../Fixture-Page'):
                content, _status = request_page(target)
                if content == page_body:
                    raise AssertionError('Client page matched an alternate path')
            if request_page('/Fixture-Page', '--request', 'POST')[1] != b'405':
                raise AssertionError('Client page accepted an unsupported request method')
            if request_page('/Fixture-Page', '--header', 'Host: native.test')[1] != b'421':
                raise AssertionError('Caddy accepted a Host/SNI mismatch')
            print('PASS: client page exact request target, method and Host/SNI checks', flush=True)
            # Genuine REALITY server/client, target TLS stays inside this fixture.
            private=secrets.token_bytes(32)
            encoded=base64.urlsafe_b64encode(private).decode().rstrip('=')
            public=docker.run(['/usr/bin/openssl','pkey','-inform','DER','-pubout','-outform','DER'],
                data=bytes.fromhex('302e020100300506032b656e04220420')+private)[-32:]
            public=base64.urlsafe_b64encode(public).decode().rstrip('=')
            user=str(uuid.uuid4());short=secrets.token_hex(8)
            server={'log':{'loglevel':'debug'},'inbounds':[{'listen':'127.0.0.1','port':nativeport,'protocol':'vless',
                'settings':{'clients':[{'id':user,'flow':'xtls-rprx-vision'}],'decryption':'none'},
                'streamSettings':{'network':'tcp','security':'reality','realitySettings':{'target':f'127.0.0.1:{siteport}',
                    'serverNames':['native.test'],'privateKey':encoded,'shortIds':[short]}}}], 'outbounds':[{'protocol':'freedom','settings':{'finalRules':[{'action':'allow','network':'tcp','ip':['127.0.0.1/32']}]}}]}
            client={'log':{'loglevel':'debug'},'inbounds':[{'listen':'127.0.0.1','port':socksport,'protocol':'socks','settings':{'auth':'noauth'}}],
                'outbounds':[{'protocol':'vless','settings':{'vnext':[{'address':'127.0.0.1','port':edgeport,'users':[{'id':user,'flow':'xtls-rprx-vision','encryption':'none'}]}]},
                'streamSettings':{'network':'tcp','security':'reality','realitySettings':{'fingerprint':'chrome','serverName':'native.test','publicKey':public,'shortId':short}}}]}
            (root/'server.json').write_text(json.dumps(server));(root/'client.json').write_text(json.dumps(client))
            xoptions=mount+['--tmpfs','/etc/xray:rw,mode=1777,size=1048576','--tmpfs','/var/log/xray:rw,mode=1777,size=1048576','--entrypoint','xray']
            serverid=create(pins['xray'],'run','-config','/etc/hton/server.json',options=xoptions)
            clientid=create(pins['xray'],'run','-config','/etc/hton/client.json',options=xoptions)
            start(serverid);start(clientid)
            time.sleep(2.5)
            class Handler(http.server.BaseHTTPRequestHandler):
                def do_GET(self):
                    self.send_response(200);self.end_headers();self.wfile.write(body)
                def log_message(self,*args):pass
            http_fixture=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
            thread=threading.Thread(target=http_fixture.serve_forever,daemon=True);thread.start()
            try:
                curl=['/usr/bin/curl','--disable','--silent','--show-error','--noproxy','','--proxy',f'socks5h://127.0.0.1:{socksport}',
                    '--max-time','6',f'http://127.0.0.1:{http_fixture.server_port}/']
                for attempt in range(3):
                    try:
                        if docker.run(curl,timeout=8)!=body:raise AssertionError('VPN response mismatch')
                        break
                    except InstallError:
                        if attempt==2:
                            for fixture in (serverid,clientid):
                                result=subprocess.run(docker.DOCKER+['container','logs',fixture],capture_output=True,text=True)
                                (root/('diagnostic-'+fixture[:8])).write_text(result.stdout+result.stderr)
                                print((result.stdout+result.stderr)[-5000:],flush=True)
                            raise
                        time.sleep(.2)
                print('PASS: authenticated VLESS REALITY through exact-SNI HAProxy route',flush=True)
                docker.command('container','stop','--time','2',clientid)
                client['outbounds'][0]['settings']['vnext'][0]['users'][0]['id']=str(uuid.uuid4())
                (root/'client.json').write_text(json.dumps(client));start(clientid)
                time.sleep(.3)
                try:docker.run(curl,timeout=8)
                except InstallError:pass
                else:raise AssertionError('Invalid REALITY credentials accepted')
                print('PASS: invalid native credentials fail authentication',flush=True)
            finally:http_fixture.shutdown();http_fixture.server_close()
            # Production-like validator must read restrictive captured config
            # without chmod, network or write access.
            config_mode=(root/'server.json').stat().st_mode & 0o777
            (root/'server.json').chmod(0o001)
            custody_work=Store(root)
            try:
                validator=create(pins['xray'],'run','-test','-config','/fixture/server.json',options=[
                    '--network','none','--read-only','--user','0:0','--cap-drop','ALL','--cap-add','DAC_READ_SEARCH',
                    '--security-opt','no-new-privileges:true','--memory','128m','--pids-limit','64',
                    '--label','io.hton.minimum='+token,'--label','io.hton.role=validator',
                    '--mount',f'type=bind,src={root},dst=/fixture,readonly','--entrypoint','xray'])
                custody={'instance':token}
                ingress.record_owned(custody_work,custody,validator,'validator')
                docker.command('container','start','--attach',validator)
                ingress.owned(custody,validator,'validator',custody_work)
                if docker.inspect(validator)['State']['ExitCode']!=0:raise AssertionError('Read-only native validation failed')
                if (root/'server.json').stat().st_mode & 0o777 != 0o001:raise AssertionError('Native permissions changed')
                print('PASS: isolated native validator reads mode-0001 config without changing it',flush=True)
            finally:
                custody_work.close();(root/'server.json').chmod(config_mode)
            # Snapshot a disposable writable layer and preserve launch metadata.
            original=create(pins['haproxy'],'-c','echo preserved > /fixture-key; sleep 300',
                options=['--user','0:0','--entrypoint','/bin/sh','--env','FIXTURE=preserve','--network','bridge','--expose','443/tcp','--expose','443/udp'])
            network=docker.exact_id(docker.command('network','create','--driver','bridge','--label',label,'hton-fixture-net-'+token).decode().strip())
            networks.append(network)
            docker.command('network','connect','--alias','fixture-dns',network,original)
            start(original);time.sleep(.3)
            originalvalue=docker.inspect(original)
            image=docker.command('container','commit',original).decode().strip();images.append(image)
            used=docker.inventory()
            backend=next(p for p in range(5000,6001) if all(not any(b.get('HostPort')==str(p) for bs in (v['HostConfig'].get('PortBindings') or {}).values() for b in bs or []) for v in used.values()))
            payload=docker.create_payload(originalvalue,image,backend)
            payload.setdefault('Labels',{})['io.hton.minimum-test']=token
            try:
                replacement=docker.create('hton-fixture-'+token,payload)
            except InstallError:
                engine=docker.Engine()
                try:
                    engine.request('GET','/version');version=json.loads(engine.getresponse().read())['ApiVersion']
                    engine.request('POST','/v'+version+'/containers/create?name=hton-fixture-'+token,json.dumps(payload),{'Content-Type':'application/json'})
                    result=engine.getresponse()
                    print('Fixture-only create diagnostic:',result.status,result.read(65536).decode(),flush=True)
                finally:engine.close()
                raise
            containers.append(replacement)
            a=docker.command('container','cp',original+':/fixture-key','-')
            b=docker.command('container','cp',replacement+':/fixture-key','-')
            import io,tarfile
            def content(data):
                with tarfile.open(fileobj=io.BytesIO(data)) as t:return t.extractfile(t.getmembers()[0]).read()
            if content(a)!=content(b) or content(b)!=b'preserved\n':raise AssertionError('Writable native state lost')
            inspected=docker.inspect(replacement)
            expected=dict(payload);host=expected.pop('HostConfig');expected.pop('NetworkingConfig',None)
            if inspected['Config']!=expected or inspected['HostConfig']!=host:
                for section,actual,wanted in (('Config',inspected['Config'],expected),('HostConfig',inspected['HostConfig'],host)):
                    for key in sorted(set(actual)|set(wanted)):
                        if actual.get(key)!=wanted.get(key):print('Fixture metadata difference',section,key,repr(actual.get(key)),repr(wanted.get(key)),flush=True)
                raise AssertionError('Docker launch metadata changed during recreate')
            if not docker.same_networks(originalvalue,inspected):raise AssertionError('Native DNS network attachments lost')
            docker.verify_networks(originalvalue)
            start(replacement)
            running=docker.inspect(replacement)
            if not docker.same_networks(originalvalue,running):raise AssertionError('Running replacement network attachments differ')
            if set(running['NetworkSettings']['Networks'])!={'bridge','hton-fixture-net-'+token}:raise AssertionError('DNS network missing')
            for proto in ('443/tcp','443/udp'):
                if inspected['HostConfig']['PortBindings'][proto]!=[{'HostIp':'127.0.0.1','HostPort':str(backend)}]:raise AssertionError('Bad remap')
            if docker.fingerprint(docker.inspect(original))!=docker.fingerprint(originalvalue):raise AssertionError('Original mutated')
            print('PASS: Docker snapshot/recreate preserves data, full launch, DNS network attachments and original',flush=True)
    finally:
        for identifier in reversed(containers):
            value=docker.inspect(identifier)
            if value['Config'].get('Labels',{}).get('io.hton.minimum-test')!=token:raise AssertionError('Fixture ownership drift')
            docker.command('container','rm','--force',identifier)
        for network in networks:
            observed=json.loads(docker.command('network','inspect','--format','{{json .}}',network))
            if observed['Id']!=network or (observed.get('Labels') or {}).get('io.hton.minimum-test')!=token:
                raise AssertionError('Fixture network ownership drift')
            docker.command('network','rm',network)
        for image in images:docker.command('image','rm','--force',image)
    print('Minimum component acceptance passed. Production deployment remains separate.')

if __name__=='__main__':main()
