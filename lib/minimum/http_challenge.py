"""Bounded HTTP-01 webroot custody and local challenge route proof."""
from __future__ import annotations
import contextlib
import errno
import os
from pathlib import Path
import secrets
import socket
import stat
from docker import run
from storage import InstallError


@contextlib.contextmanager
def open_directory(root, *, cleanup=False, owner=None):
    root=Path(root)
    if not root.is_absolute() or root.resolve()!=root or any(c in str(root) for c in ',\r\n\x00'):
        raise InstallError('HTTP webroot must be a canonical absolute directory without symlinks or commas')
    trusted={0,os.geteuid(),int(os.environ.get('SUDO_UID','0'))}
    if owner is not None:
        if type(owner) is not int or not 0<=owner<=4294967294:
            raise InstallError('Invalid saved webroot owner')
        trusted.add(owner)
    fd=os.open('/',os.O_RDONLY|os.O_DIRECTORY|os.O_CLOEXEC)
    created=[]
    try:
        parts=list(root.parts[1:])+['.well-known','acme-challenge']
        for index,part in enumerate(parts):
            made=False
            if part in ('.','..'):
                raise InstallError('Invalid HTTP webroot path')
            if index>=len(parts)-2:
                try:
                    os.mkdir(part,0o755,dir_fd=fd)
                    made=True
                    info=os.stat(part,dir_fd=fd,follow_symlinks=False)
                    created.append((os.dup(fd),part,info.st_ino))
                    os.fsync(fd)
                except FileExistsError:pass
            child=os.open(part,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW|os.O_CLOEXEC,dir_fd=fd)
            info=os.fstat(child)
            if made:
                if info.st_ino!=created[-1][2] or info.st_uid!=os.geteuid() or info.st_dev!=os.fstat(fd).st_dev:
                    os.close(child)
                    raise InstallError('New HTTP challenge directory identity changed')
                os.fchmod(child,0o755)
                info=os.fstat(child)
            sticky_parent = index < len(root.parts)-2 and info.st_uid==0 and info.st_mode & stat.S_ISVTX
            if info.st_uid not in trusted or (info.st_mode & 0o022 and not sticky_parent):
                os.close(child)
                raise InstallError('HTTP challenge path has unsupported ownership or write permissions')
            os.close(fd);fd=child
        info=os.fstat(fd)
        if info.st_uid!=os.geteuid() or stat.S_IMODE(info.st_mode)&0o055!=0o055:
            raise InstallError('Existing HTTP challenge directory must be owned by the setup user and publicly readable; no ownership was changed')
        yield root/'.well-known/acme-challenge',fd
    finally:
        os.close(fd)
        for parent,name,inode in reversed(created):
            try:
                if cleanup and os.stat(name,dir_fd=parent,follow_symlinks=False).st_ino==inode:
                    try:os.rmdir(name,dir_fd=parent)
                    except OSError as error:
                        if error.errno not in (errno.ENOTEMPTY,errno.EEXIST):raise
            except FileNotFoundError:pass
            finally:os.close(parent)


def prove(root,domain,*,port=80,owner=None):
    if owner is not None and Path(root).stat().st_uid!=owner:
        raise InstallError("Saved HTTP webroot ownership changed")
    with open_directory(root,cleanup=True,owner=owner) as (directory,parent):
        name='hton-probe-'+secrets.token_hex(16)
        token=secrets.token_urlsafe(32).encode()
        fd=os.open(name,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW|os.O_CLOEXEC,0o644,dir_fd=parent)
        try:
            os.write(fd,token);os.fchmod(fd,0o644);os.fsync(fd)
            identity=os.fstat(fd)
        finally:os.close(fd)
        try:
            response=run(['/usr/bin/curl','--disable','--silent','--show-error','--noproxy','*',
                '--proto','=http','--connect-timeout','3','--max-time','5','--fail',
                '--header','Host: '+domain,'http://127.0.0.1:'+str(port)+'/.well-known/acme-challenge/'+name],timeout=7,limit=65536)
            if response!=token:
                raise InstallError('Existing HTTP server did not return the exact challenge probe')
        finally:
            info=os.stat(name,dir_fd=parent,follow_symlinks=False)
            if (info.st_dev,info.st_ino)!=(identity.st_dev,identity.st_ino) or not stat.S_ISREG(info.st_mode):
                raise InstallError('HTTP challenge probe identity changed; refusing cleanup')
            os.unlink(name,dir_fd=parent);os.fsync(parent)
    return Path(root)


def select(domain, saved=None, configured=None):
    def webroot(root, owner=None):
        prove(Path(root),domain,owner=owner)
        return {'mode':'webroot','webroot':str(root),'ownerUid':Path(root).stat().st_uid}
    if configured:
        owner = saved.get('ownerUid') if saved and saved.get('webroot')==str(configured) else None
        return webroot(configured,owner)
    if saved and saved.get('mode')=='webroot':
        return webroot(saved['webroot'],saved.get('ownerUid'))
    with socket.socket(socket.AF_INET,socket.SOCK_STREAM) as listener:
        try:
            listener.bind(('0.0.0.0',80))
            return {'mode':'standalone'}
        except OSError:pass
    for root in (Path('/var/www'),Path('/var/www/html')):
        try:return webroot(root)
        except (OSError,InstallError):continue
    raise InstallError('TCP/80 is occupied and no verified webroot was found. Configure core.certificates.http.webroot to the directory serving this domain, or select a supported DNS API mode; the existing HTTP service was not changed')
