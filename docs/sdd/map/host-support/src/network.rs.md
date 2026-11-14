# host-support/src/network.rs

Stable ID adapter.host-network · R1 · I3

Fixed HTTPS curl observations only: public_ipv4 uses ipify IPv4 JSON; dns_ipv4 uses
Cloudflare validated DoH JSON with exact question and bounded CNAME/A chain checks.
No arbitrary URL/headers/redirects/proxy/client configuration; fixed TLS-only sites,
cleared process environment and inherited monotonic/output/cancellation budget.
These observations establish a vantage-bound DNS/egress correspondence, not Internet
inbound reachability/NAT mapping or censorship. Native route acceptance is separate.
No requests execute during source implementation. Primary interface sources:
[ipify](https://www.ipify.org/) and [Cloudflare JSON](https://developers.cloudflare.com/1.1.1.1/encryption/dns-over-https/make-api-requests/dns-json/).
