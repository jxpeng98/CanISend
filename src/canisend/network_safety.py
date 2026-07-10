from __future__ import annotations

import ipaddress
import socket
from collections.abc import Callable, Iterable


AddressResolver = Callable[[str], Iterable[str]]


class ResolvedAddressError(ValueError):
    """Raised when a hostname cannot be resolved only to public addresses."""


def resolve_host_addresses(hostname: str) -> tuple[str, ...]:
    try:
        records = socket.getaddrinfo(hostname, None, type=socket.SOCK_STREAM)
    except OSError as exc:
        raise ResolvedAddressError("Host name resolution failed.") from exc
    addresses = tuple(sorted({str(record[4][0]) for record in records if record[4]}))
    if not addresses:
        raise ResolvedAddressError("Host name resolution returned no addresses.")
    return addresses


def require_public_resolved_addresses(
    hostname: str,
    *,
    resolver: AddressResolver | None = None,
) -> tuple[str, ...]:
    try:
        literal = ipaddress.ip_address(hostname)
    except ValueError:
        literal = None
    if literal is not None:
        if not literal.is_global:
            raise ResolvedAddressError("Host address is not publicly routable.")
        return (str(literal),)

    active_resolver = resolver or resolve_host_addresses
    try:
        resolved = tuple(str(value) for value in active_resolver(hostname))
    except ResolvedAddressError:
        raise
    except Exception as exc:
        raise ResolvedAddressError("Host name resolution failed.") from exc
    if not resolved:
        raise ResolvedAddressError("Host name resolution returned no addresses.")
    for value in resolved:
        try:
            address = ipaddress.ip_address(value)
        except ValueError as exc:
            raise ResolvedAddressError("Host name resolution returned invalid address data.") from exc
        if not address.is_global:
            raise ResolvedAddressError(
                "Host name resolved to an address that is not publicly routable."
            )
    return resolved
