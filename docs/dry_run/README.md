## Dry Run Example

`path/to/three_em dry-run --file dry_run_users_contract.json`

**Output**:

```json
{
  "state": {
    "users": [
      "Andres Pirela",
      "Divy",
      "Some Other"
    ]
  },
  "validity": {
    "tx1": true,
    "tx2": true,
    "tx3": false,
    "tx4": true
  }
}
```
