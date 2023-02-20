## Unreleased

- increase compatibility with jsonrpc 1.0 by allowing `id == 0` and to omit `"jsonrpc":"2.0"` property #31
- upgrade axum to `0.6.6`
- add CommonJS build
- fix: update generated ts types that were forgotten in `0.4.0`

## 0.4.0

- also allow strings as ids #27
- remove `__AllTyps` ts type from output #18
- Do not crash if "params" are omitted from request #22
- fix: correct feature flags axum for tests

## Older

see git commit history for older releases 