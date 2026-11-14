"""Local test ACME authority exercising the pinned native HTTP-01 client."""
import base64
import hashlib
import http.server
import json
import functools
import os
import secrets
import socket
import ssl
import threading
import time
from pathlib import Path

import certificates
import http_challenge
import ingress
import docker
from storage import Store, InstallError


def decoded(value):
    return base64.urlsafe_b64decode(value + '=' * (-len(value) % 4))


def exercise_http_issuer(root, pins, token, uid, containers, *, webroot=False):
    root=Path(root)
    directory=root/('http-issuer-webroot' if webroot else 'http-issuer');directory.mkdir(mode=0o700)
    work=Store(directory)
    work.write('issuer/owner',b'fixture\n')
    relative='certificates/'+secrets.token_hex(12)
    work.write(relative+'/pending',b'fixture\n')
    output=directory/relative
    challenge_server=None
    transport=None
    if webroot:
        public_root=root/'existing-http';public_root.mkdir(mode=0o755)
        with http_challenge.open_directory(public_root) as (challenge_dir,_):
            sentinel=challenge_dir/'unrelated-token';sentinel.write_text('preserved')
        class Static(http.server.SimpleHTTPRequestHandler):
            def log_message(self,*args):pass
        challenge_server=http.server.ThreadingHTTPServer(('127.0.0.1',0),functools.partial(Static,directory=str(public_root)))
        threading.Thread(target=challenge_server.serve_forever,daemon=True).start()
        challenge_port=challenge_server.server_port
        http_challenge.prove(public_root,'site.test',port=challenge_port)
        transport={'mode':'webroot','webroot':str(public_root),'ownerUid':os.getuid()}
    else:
        with socket.socket() as reservation:
            reservation.bind(('127.0.0.1',0));challenge_port=reservation.getsockname()[1]
    observed={'validated':False,'thumbprint':None,'certificate':None,'orders':0}
    challenge_token=secrets.token_urlsafe(24)
    body=b'HTON local ACME test'
    base=None

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self,*args):pass
        def reply(self,status,value=None,location=None,raw=False):
            content=value if raw else json.dumps(value or {}).encode()
            self.send_response(status)
            self.send_header('Replay-Nonce',secrets.token_urlsafe(24))
            self.send_header('Content-Type','application/pem-certificate-chain' if raw else 'application/json')
            self.send_header('Content-Length',str(len(content)))
            if location:self.send_header('Location',location)
            self.end_headers()
            if self.command!='HEAD':self.wfile.write(content)
        def do_HEAD(self):self.reply(200)
        def do_GET(self):
            if self.path=='/directory':
                self.reply(200,{'newNonce':base+'/nonce','newAccount':base+'/account','newOrder':base+'/order',
                    'revokeCert':base+'/revoke','keyChange':base+'/key-change','meta':{'termsOfService':base+'/terms'}})
            elif self.path=='/certificate':self.reply(200,observed['certificate'],raw=True)
            else:self.reply(404)
        def do_POST(self):
            try:
                request=json.loads(self.rfile.read(int(self.headers.get('Content-Length',0))))
                header=json.loads(decoded(request['protected']))
                payload=json.loads(decoded(request['payload'])) if request.get('payload') else {}
                if self.path=='/account':
                    jwk=header['jwk']
                    observed['thumbprint']=base64.urlsafe_b64encode(hashlib.sha256(json.dumps(jwk,sort_keys=True,separators=(',',':')).encode()).digest()).decode().rstrip('=')
                    self.reply(201,{'status':'valid','orders':base+'/orders'},base+'/account/1')
                elif self.path=='/order':
                    observed['orders']+=1
                    self.reply(201,self.order(),base+'/order/1')
                elif self.path=='/authz':
                    self.reply(200,{'identifier':{'type':'dns','value':'site.test'},'status':'valid' if observed['validated'] else 'pending',
                        'challenges':[{'type':'http-01','url':base+'/challenge','token':challenge_token,'status':'valid' if observed['validated'] else 'pending'}]})
                elif self.path=='/challenge':
                    if webroot and not (public_root/'.well-known/acme-challenge'/challenge_token).stat().st_mode & 0o004:
                        raise AssertionError('HTTP token is not readable by the web server')
                    with socket.create_connection(('127.0.0.1',challenge_port),timeout=5) as stream:
                        stream.sendall(('GET /.well-known/acme-challenge/'+challenge_token+' HTTP/1.0\r\nHost: site.test\r\n\r\n').encode())
                        response=b''
                        while block:=stream.recv(65536):response+=block
                    received=response.partition(b'\r\n\r\n')[2].strip()
                    expected=(challenge_token+'.'+observed['thumbprint']).encode()
                    if received!=expected:raise AssertionError('HTTP-01 token/thumbprint mismatch')
                    observed['validated']=True
                    self.reply(200,{'type':'http-01','url':base+'/challenge','token':challenge_token,'status':'valid'})
                elif self.path=='/finalize':
                    if not observed['validated']:raise AssertionError('Finalize without HTTP authorization')
                    (directory/'csr.der').write_bytes(decoded(payload['csr']))
                    (directory/'extensions').write_text('subjectAltName=DNS:site.test\nextendedKeyUsage=serverAuth\n')
                    docker.run(['/usr/bin/openssl','x509','-req','-inform','DER','-in',str(directory/'csr.der'),
                        '-CA',str(root/'ca.pem'),'-CAkey',str(root/'ca.key'),'-CAcreateserial','-out',str(directory/'leaf.pem'),
                        '-days','90','-extfile',str(directory/'extensions')])
                    observed['certificate']=(directory/'leaf.pem').read_bytes()+(root/'ca.pem').read_bytes()
                    self.reply(200,self.order(),base+'/order/1')
                elif self.path=='/order/1':self.reply(200,self.order())
                elif self.path=='/certificate':self.reply(200,observed['certificate'],raw=True)
                else:self.reply(404)
            except Exception as error:
                observed['error']=type(error).__name__
                self.reply(500,{'type':'urn:ietf:params:acme:error:serverInternal','detail':'Fixture request failed'})
        def order(self):
            result={'status':'valid' if observed['certificate'] else ('ready' if observed['validated'] else 'pending'),
                'identifiers':[{'type':'dns','value':'site.test'}],'authorizations':[base+'/authz'],'finalize':base+'/finalize'}
            if observed['certificate']:result['certificate']=base+'/certificate'
            return result

    server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
    base='http://127.0.0.1:'+str(server.server_port)
    thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
    try:
        args=certificates.issuer_arguments(work,{'instance':token},pins['acme'],token+('-webroot' if webroot else ''),output,'site.test','fixture@example.test','http-01',http=transport)
        args[args.index('--user')+1]=uid
        args[args.index('--server')+1]=base+'/directory'
        if not webroot:args[args.index('--httpport')+1]=str(challenge_port)
        args[2:2]=['--label','io.hton.minimum-test='+token]
        identifier=docker.exact_id(docker.command(*args).decode().strip());containers.append(identifier)
        custody={'instance':token}
        ingress.record_owned(work,custody,identifier,'acme')
        ingress.owned(custody,identifier,'acme',work)
        result=docker.command('container','start','--attach',identifier,allowed=(0,1,2),timeout=120)
        ingress.owned(custody,identifier,'acme',work)
        if docker.inspect(identifier)['State']['ExitCode']!=0:
            print(result.decode(errors='replace')[-4000:],flush=True)
            raise AssertionError('Native HTTP issuer failed: '+str(observed.get('error','no fixture error')))
        if not observed['validated'] or observed['orders']!=1:raise AssertionError('Native HTTP authorization was not completed exactly once')
        ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER).load_cert_chain(output/'fullchain.pem',output/'key.pem')
        docker.run(['/usr/bin/openssl','verify','-CAfile',str(root/'ca.pem'),'-verify_hostname','site.test',str(output/'fullchain.pem')])
        try:certificates.validate(output/'fullchain.pem',output/'key.pem','site.test')
        except InstallError:pass
        else:raise AssertionError('Production verifier trusted the private fixture CA')
        if webroot and sentinel.read_text()!='preserved':raise AssertionError('Unrelated challenge token was changed')
        print('PASS: native acme.sh HTTP-01 '+('existing-webroot' if webroot else 'standalone')+' issuance, token proof, key/SAN and default trust rejection',flush=True)
    finally:
        server.shutdown();server.server_close();work.close()
        if challenge_server:challenge_server.shutdown();challenge_server.server_close()
