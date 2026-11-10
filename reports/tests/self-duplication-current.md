# Self-duplication gate

Command: `env -u RUST_LOG nice -n 15 make -o build _dup-gate`. Exit status: **0**. Verified release binary reused.

Report SHA-256: `7d540a03392fdc32dbfbc51389bda11ce38e50f4144ed95c8c4a846cb8259fbb`.

```json
{
  "analysed_loc": 203910,
  "duplicated_loc": 9896,
  "duplication_percent": 4.85312147516061,
  "clusters_total": 1200,
  "duplicated_files": 463,
  "threshold": {
    "percent": 5.5,
    "breached": false,
    "source": "config"
  }
}
```
