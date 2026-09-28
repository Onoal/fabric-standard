# Fabric Standard Architecture

Fabric Standard is a developer layer, not a replacement for Fabric.

The core law is:

```text
excellent domain DX
AND
native Fabric composition
```

The domain surface should feel natural to application authors. The Fabric surface must remain real: resource truth, materialization, composition, and runtime relations still belong to Fabric.

Standards compose through Fabric participation rather than through pairwise plugin systems. Fabric Standard generalizes machinery when pressure repeats, but it does not generalize application vocabulary into framework ontology.

## HTTP

Standard HTTP owns a small HTTP application framework:

- route declaration;
- request context;
- request and response projections;
- middleware execution;
- socket-free dispatch.

It does not own backend modules, storage, authentication, OpenAPI, RPC, streaming, or a middleware catalog.

Standard HTTP Core is runtime-independent application semantics. The Native
Fabric runtime is a separate crate that realizes a Standard HTTP application as
a long-running Fabric HTTP/TCP server. Cloudflare Workers is the only current
external runtime target under active investigation; it is not implemented yet.

The current execution boundaries are intentionally distinct:

```text
App
    HTTP declaration plus Fabric authoring contributions

PreparedApp
    prepared pure HTTP semantics

TestRuntime
    testing-oriented Fabric-bound socket-free execution

serve
    Native Fabric runtime execution through real HTTP transport
```

`TestRuntime` is not a universal production runtime ontology. It exists so
resource-backed Standard HTTP applications can be tested through real Fabric
materialization and relation binding without TCP.

Standard HTTP application semantics are runtime-independent. Native Fabric TCP
serving is one runtime realization, not the identity of the HTTP application.
The future Cloudflare Workers runtime will also be a runtime realization, not
HTTP identity. `App` and `PreparedApp` are not native TCP servers, Cloudflare
Workers, or any other runtime target.
