# Exact Alpha.10 qualification design

This task is a release pipeline, not a feature branch:

```text
protected headless merge -> protected metadata transition -> nonpublishing candidate
-> native + host qualification -> annotated tag -> same-byte promotion
-> independent public download verification -> authority reconciliation
```

Each arrow is a stop gate. Candidate failure returns to protected source with a new candidate; no
failed candidate is tagged. Promotion locates cached qualified bytes and never rebuilds. Public
Alpha.9 and historical evidence remain immutable. Synthetic host evidence records body-free
identities and outcomes only; it does not satisfy the invited cohort.
