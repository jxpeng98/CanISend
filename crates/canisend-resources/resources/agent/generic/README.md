# CanISend Agent Protocol v2

This embedded bootstrap resource belongs to the Rust-native CanISend product.

Start by running:

```text
canisend agent capabilities --json
```

Read only capabilities marked `available`. Do not edit `.canisend/` internal state. Follow task input revisions,
candidate schemas, privacy scope, and required consents. Submit candidates only through CanISend task commands;
readiness is not evidence that an application was submitted.
