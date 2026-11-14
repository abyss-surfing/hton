"""Explicit setup publication observation and provider-neutral manual assistance."""
from __future__ import annotations
import ipaddress
import json
from urllib.parse import urlencode
from docker import run
from storage import InstallError

RESOLVERS = {'cloudflare': 'https://cloudflare-dns.com/dns-query', 'google': 'https://dns.google/resolve'}


def get(url, *, accept='text/plain'):
    return run(['/usr/bin/curl','--disable','--silent','--show-error','--noproxy','*',
        '--proto','=https','--connect-timeout','5','--max-time','10','--fail',
        '--header','Accept: '+accept,url],timeout=12,limit=65536)


def public_ipv4(value):
    try:
        address=ipaddress.ip_address(value)
    except ValueError as error:
        raise InstallError('Publication address must be a public IPv4 address') from error
    if address.version != 4 or not address.is_global:
        raise InstallError('Publication address must be a public IPv4 address')
    return str(address)


def external_ipv4():
    data=get('https://1.1.1.1/cdn-cgi/trace').decode('ascii')
    values=[line[3:] for line in data.splitlines() if line.startswith('ip=')]
    if len(values)!=1:
        raise InstallError('External IPv4 observation failed; configure core.edge.publication.addresses explicitly')
    return public_ipv4(values[0])


def records(domain, kind, resolver):
    if resolver not in RESOLVERS:
        raise InstallError('Publication resolver must be cloudflare or google')
    data=json.loads(get(RESOLVERS[resolver]+'?'+urlencode({'name':domain,'type':kind}),accept='application/dns-json'))
    if data.get('Status') not in (0,3) or data.get('TC'):
        raise InstallError('Public DNS lookup was unsuccessful; no publication was assumed')
    if data.get('Status')==3:
        return []
    numeric=1 if kind=='A' else 28
    aliases={domain.rstrip('.').lower()}
    answers=data.get('Answer') or []
    if not isinstance(answers,list) or len(answers)>128:
        raise InstallError('Public DNS answer exceeds its supported bound')
    for _ in range(8):
        for item in answers:
            if item.get('type')==5 and item.get('name','').rstrip('.').lower() in aliases:
                aliases.add(item['data'].rstrip('.').lower())
    values=set()
    for item in answers:
        if item.get('type')==numeric and item.get('name','').rstrip('.').lower() in aliases:
            address=ipaddress.ip_address(item['data'])
            if address.version!=(4 if kind=='A' else 6):
                raise InstallError('Public DNS address family differs from the query')
            values.add(str(address))
    return sorted(values)


def instructions(domain, expected, actual, ipv6):
    return ('DNS does not match the configured public entry point.\n'
        +'Domain: '+domain+'\nExpected A: '+', '.join(expected)+'\nObserved A: '+(', '.join(actual) or '(missing)')
        +'\nObserved AAAA: '+(', '.join(ipv6) or '(none)')
        +'\nIn your DNS provider panel, set the A record for this domain to the expected IPv4 address(es).'
        +'\nRemove other A records and remove AAAA records for this IPv4-only installation.'
        +'\nFor the zone root, the record name is usually @; for a subdomain, use its relative name.'
        +'\nDisable DNS proxy/CDN mode for this direct entry point, save, and wait for DNS caches to update.'
        +'\nIf using NAT or a load balancer, configure its public entry addresses instead of the outbound IP.'
        +'\nPress Enter to check again, or type cancel to stop')


def check(domain, edge, *, interactive, prompt):
    settings=edge.get('publication',{})
    if not isinstance(settings,dict):
        raise InstallError('Invalid core.edge.publication configuration')
    mode=settings.get('mode','direct')
    expected=settings.get('addresses')
    if mode!='direct':
        raise InstallError('This minimum HTTP profile requires a direct entry point; shared frontend challenge routing needs an explicit supported policy')
    if expected is None:
        expected=[external_ipv4()]
    if not isinstance(expected,list) or not 1<=len(expected)<=8:
        raise InstallError('Configure one to eight public entry IPv4 addresses')
    expected=sorted({public_ipv4(address) for address in expected})
    resolver=settings.get('resolver','cloudflare')
    for attempt in range(5):
        actual=records(domain,'A',resolver);ipv6=records(domain,'AAAA',resolver)
        if actual==expected and not ipv6:
            return {'status':'matched','resolver':resolver,'domain':domain,'addresses':expected}
        text=instructions(domain,expected,actual,ipv6)
        if not interactive:
            raise InstallError(text.rsplit('\nPress Enter',1)[0]+'\nCorrect DNS and repeat scripts/setup')
        if prompt(text).lower() not in ('','retry'):
            raise InstallError('DNS setup cancelled; Amnezia was not changed')
    raise InstallError('DNS still differs after five checks; correct the records and repeat scripts/setup')
