# core/minimum/http_challenge.py

Stable ID: core.minimum.http-challenge · M0 · source

Owns safe public challenge directories, exclusive ephemeral route probes and
HTTP transport selection. open_directory walks nofollow descriptors and validates
owner/mode; prove verifies exact localhost HTTP body for the configured domain;
select returns standalone or verified webroot, cleaning only its temporary proof
and newly created empty directories. No global service/config/permission changes.
Native issuer receives only the selected challenge subdirectory. Persist selected
transport in protected state; future renewal rechecks locally without public DNS.
Contract: sdd-minimum-installation HTTP-01 delivery and existing HTTP listener.
